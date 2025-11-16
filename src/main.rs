mod agent;
mod agents;
mod api;
mod error;
mod events;
mod llm;
mod mcp;
mod tool_executor;
mod ui;
mod version;

use agent::AgentConfig;
use api::RustbotApi;
use eframe::egui;
use error::{Result, RustbotError};
use events::{Event, EventBus, EventKind, SystemCommand};
use llm::{create_adapter, AdapterType, LlmAdapter};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::{broadcast, mpsc, Mutex};
use std::path::PathBuf;
use egui_phosphor::regular as icons;
use ui::{AppView, ChatMessage, ContextTracker, MessageRole, PluginsView, SettingsView, SystemPrompts, TokenStats, VisualEvent};
use ui::icon::create_window_icon;
use mcp::manager::McpPluginManager;

fn main() -> std::result::Result<(), eframe::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Load .env.local file - try multiple locations for robustness
    // First try current directory, then executable directory
    let env_loaded = dotenvy::from_filename(".env.local").is_ok()
        || if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                dotenvy::from_path(exe_dir.join(".env.local")).is_ok()
            } else {
                false
            }
        } else {
            false
        };

    if !env_loaded {
        tracing::warn!(".env.local file not found - will need OPENROUTER_API_KEY from environment");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Rustbot - AI Assistant")
            .with_icon(create_window_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "rustbot",
        options,
        Box::new(|cc| {
            // Setup custom fonts
            let mut fonts = egui::FontDefinitions::default();

            // Load Roboto Regular
            fonts.font_data.insert(
                "Roboto-Regular".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/Roboto-Regular.ttf"
                ))),
            );

            // Load Roboto Bold
            fonts.font_data.insert(
                "Roboto-Bold".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/Roboto-Bold.ttf"
                ))),
            );

            // Set Roboto as the default proportional font (first = highest priority)
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Roboto-Regular".to_owned());

            // Also use Roboto for monospace where appropriate
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("Roboto-Regular".to_owned());

            // Add Phosphor icon font for icons
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            // Apply fonts
            cc.egui_ctx.set_fonts(fonts);

            // Get API key with proper error handling to avoid panic in FFI boundary
            let api_key = match std::env::var("OPENROUTER_API_KEY") {
                Ok(key) => key,
                Err(_) => {
                    // Log error for debugging
                    tracing::error!("OPENROUTER_API_KEY not found in environment");
                    tracing::error!("Please ensure .env.local file exists with OPENROUTER_API_KEY=your_key");

                    // Display error message to user and exit gracefully
                    eprintln!("\n❌ ERROR: Missing OPENROUTER_API_KEY environment variable\n");
                    eprintln!("Please create a .env.local file in the project directory with:");
                    eprintln!("OPENROUTER_API_KEY=your_api_key_here\n");
                    eprintln!("Get your API key from: https://openrouter.ai/keys\n");

                    // Exit gracefully instead of panicking in FFI boundary
                    std::process::exit(1);
                }
            };
            Ok(Box::new(RustbotApp::new(api_key)))
        }),
    )
}

struct RustbotApp {
    // Core API for all functionality - wrapped in Arc<Mutex> for thread safety
    api: Arc<Mutex<RustbotApi>>,

    // UI state
    message_input: String,
    messages: Vec<ChatMessage>,
    response_rx: Option<mpsc::UnboundedReceiver<String>>,
    current_response: String,
    is_waiting: bool,
    spinner_rotation: f32,
    token_stats: TokenStats,
    context_tracker: ContextTracker,
    sidebar_open: bool,
    current_view: AppView,
    settings_view: SettingsView,
    system_prompts: SystemPrompts,
    selected_model: String,
    current_activity: Option<String>,  // Track current agent activity

    // Event visualization
    event_rx: broadcast::Receiver<Event>,
    event_history: VecDeque<VisualEvent>,
    show_event_visualizer: bool,

    // Agent UI state
    agent_configs: Vec<AgentConfig>,
    selected_agent_index: Option<usize>,

    // Pending agent result receiver
    pending_agent_result: Option<mpsc::UnboundedReceiver<anyhow::Result<mpsc::UnboundedReceiver<String>>>>,

    // MCP Plugin Manager and UI
    mcp_manager: Arc<Mutex<McpPluginManager>>,
    plugins_view: Option<PluginsView>,

    // Keep runtime for async operations
    runtime: Arc<tokio::runtime::Runtime>,
}

impl RustbotApp {
    fn new(api_key: String) -> Self {
        let token_stats = Self::load_token_stats().unwrap_or_default();
        let system_prompts = Self::load_system_prompts().unwrap_or_default();

        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        let event_rx = event_bus.subscribe();

        // Create runtime
        let runtime = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));

        // Create LLM adapter
        let llm_adapter: Arc<dyn LlmAdapter> = Arc::from(create_adapter(AdapterType::OpenRouter, api_key.clone()));

        // Load agents from JSON preset files using AgentLoader
        let agent_loader = agent::AgentLoader::new();
        let mut agent_configs = agent_loader.load_all()
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to load agents from presets: {}", e);
                vec![]
            });

        // If no agents loaded, fall back to default assistant
        if agent_configs.is_empty() {
            tracing::info!("No agents loaded from JSON, using default assistant");
            agent_configs.push(AgentConfig::default_assistant());
        }

        // Build the API using RustbotApiBuilder with all loaded agents
        let mut api_builder = api::RustbotApiBuilder::new()
            .event_bus(Arc::clone(&event_bus))
            .runtime(Arc::clone(&runtime))
            .llm_adapter(Arc::clone(&llm_adapter))
            .max_history_size(20)
            .system_instructions(system_prompts.system_instructions.clone());

        // Add all loaded agents
        for agent_config in &agent_configs {
            api_builder = api_builder.add_agent(agent_config.clone());
        }

        let api = api_builder.build().expect("Failed to build RustbotApi");

        // Initialize MCP plugin manager with event bus
        let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(
            Some(Arc::clone(&event_bus))
        )));

        // Load MCP configuration if available
        let mcp_config_path = std::path::Path::new("mcp_config.json");
        if mcp_config_path.exists() {
            let mgr = Arc::clone(&mcp_manager);
            runtime.block_on(async move {
                if let Ok(mut manager) = mgr.try_lock() {
                    match manager.load_config(mcp_config_path).await {
                        Ok(_) => {
                            tracing::info!("✓ Loaded MCP configuration from mcp_config.json");
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load MCP configuration: {}", e);
                        }
                    }
                }
            });
        } else {
            tracing::info!("No mcp_config.json found, MCP plugins disabled");
        }

        // Create plugins view with runtime handle
        let plugins_view = Some(PluginsView::new(
            Arc::clone(&mcp_manager),
            runtime.handle().clone()
        ));

        Self {
            api: Arc::new(Mutex::new(api)),
            message_input: String::new(),
            messages: Vec::new(),
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            spinner_rotation: 0.0,
            token_stats: Self::check_and_reset_daily_stats(token_stats),
            context_tracker: ContextTracker::default(),
            sidebar_open: true, // Start with sidebar open
            current_view: AppView::Chat,
            settings_view: SettingsView::Agents, // Start with Agents view to show loaded agents
            system_prompts,
            selected_model: "Claude Sonnet 4.5".to_string(),
            current_activity: None,
            event_rx,
            agent_configs: agent_configs.clone(),
            selected_agent_index: None,
            event_history: VecDeque::with_capacity(50),
            show_event_visualizer: true, // Start with visualizer open for debugging
            pending_agent_result: None,
            mcp_manager,
            plugins_view,
            runtime,
        }
    }

    fn get_instructions_dir() -> Result<PathBuf> {
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| RustbotError::EnvError(
                "Could not determine home directory: HOME or USERPROFILE not set".to_string()
            ))?;

        let mut dir = PathBuf::from(home_dir);
        dir.push(".rustbot");
        dir.push("instructions");

        // Create directory if it doesn't exist
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(|e| RustbotError::StorageError(
                    format!("Failed to create instructions directory: {}", e)
                ))?;
        }

        Ok(dir)
    }

    fn load_system_prompts() -> Result<SystemPrompts> {
        let mut dir = Self::get_instructions_dir()?;

        // Load system instructions
        dir.push("system");
        dir.push("current");
        let system_instructions = if dir.exists() {
            std::fs::read_to_string(&dir)
                .map_err(|e| RustbotError::StorageError(
                    format!("Failed to read system instructions from {:?}: {}", dir, e)
                ))?
        } else {
            String::new()
        };

        Ok(SystemPrompts {
            system_instructions,
        })
    }

    fn save_system_prompts(&self) -> Result<()> {
        let base_dir = Self::get_instructions_dir()?;

        // Save system instructions
        let mut system_dir = base_dir.clone();
        system_dir.push("system");
        std::fs::create_dir_all(&system_dir)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to create system directory: {}", e)
            ))?;

        let system_current = system_dir.join("current");

        // Create backup if current exists
        if system_current.exists() {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = system_dir.join(format!("backup_{}", timestamp));
            std::fs::copy(&system_current, &backup_path)
                .map_err(|e| RustbotError::StorageError(
                    format!("Failed to create backup of system instructions: {}", e)
                ))?;
        }

        // Write new current
        std::fs::write(&system_current, &self.system_prompts.system_instructions)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to write system instructions: {}", e)
            ))?;

        Ok(())
    }

    fn get_stats_file_path() -> PathBuf {
        let mut path = PathBuf::from(".");
        path.push("rustbot_stats.json");
        path
    }

    fn load_token_stats() -> Result<TokenStats> {
        let path = Self::get_stats_file_path();
        if !path.exists() {
            return Ok(TokenStats::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to read token stats from {:?}: {}", path, e)
            ))?;

        let stats: TokenStats = serde_json::from_str(&content)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to parse token stats JSON: {}", e)
            ))?;

        Ok(stats)
    }

    fn save_token_stats(&self) -> Result<()> {
        let path = Self::get_stats_file_path();
        let content = serde_json::to_string_pretty(&self.token_stats)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to serialize token stats: {}", e)
            ))?;

        std::fs::write(&path, content)
            .map_err(|e| RustbotError::StorageError(
                format!("Failed to write token stats to {:?}: {}", path, e)
            ))?;

        Ok(())
    }

    fn check_and_reset_daily_stats(mut stats: TokenStats) -> TokenStats {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        if stats.last_reset_date != today {
            stats.daily_input = 0;
            stats.daily_output = 0;
            stats.last_reset_date = today;
        }

        stats
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        // Rough estimation: ~4 characters per token
        ((text.len() as f32) / 4.0).ceil() as u32
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        // Claude Sonnet 4.5 pricing via OpenRouter
        // Input: $3.00 per million tokens
        // Output: $15.00 per million tokens
        const INPUT_COST_PER_MILLION: f64 = 3.0;
        const OUTPUT_COST_PER_MILLION: f64 = 15.0;

        let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;

        input_cost + output_cost
    }

    fn generate_system_context(&self) -> String {
        // Get current date and time
        let now = chrono::Local::now();
        let datetime = now.format("%Y-%m-%d %H:%M:%S %Z").to_string();
        let day_of_week = now.format("%A").to_string();

        // Get system information
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        // Build the system context
        format!(
            r#"## System Context

**Current Date & Time**: {} ({})
**LLM Model**: {}
**Application**: Rustbot v{}
**Operating System**: {} ({})
**Hostname**: {}
**User**: {}

This information is provided automatically to give you context about the current system environment."#,
            datetime,
            day_of_week,
            self.selected_model,
            version::version_string(),
            os,
            arch,
            hostname,
            user
        )
    }

    fn clear_conversation(&mut self) {
        tracing::info!("🗑️  Clearing conversation - UI messages: {}, Event history: {}",
            self.messages.len(), self.event_history.len());

        // Clear UI state
        self.messages.clear();
        self.current_response.clear();
        self.context_tracker.update_counts(0, 0);

        // Clear event flow display
        self.event_history.clear();

        // Clear API conversation history and publish event
        let api = Arc::clone(&self.api);
        self.runtime.spawn(async move {
            let mut api_guard = api.lock().await;
            api_guard.clear_history();
        });
    }

    fn reload_config(&mut self) {
        tracing::info!("🔄 Reloading Rustbot configuration...");

        // Get API key from environment
        let api_key = match std::env::var("OPENROUTER_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                tracing::error!("OPENROUTER_API_KEY not found - cannot reload");
                return;
            }
        };

        // Create fresh event bus
        let event_bus = Arc::new(EventBus::new());
        let event_rx = event_bus.subscribe();

        // Create fresh LLM adapter
        let llm_adapter: Arc<dyn LlmAdapter> = Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

        // Reload agents from JSON preset files
        let agent_loader = agent::AgentLoader::new();
        let agent_configs = agent_loader.load_all()
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to load agents from presets: {}", e);
                vec![AgentConfig::default_assistant()]
            });

        tracing::info!("📋 Reloaded {} agents", agent_configs.len());
        for config in &agent_configs {
            tracing::info!("   - {} (primary: {}, enabled: {})",
                         config.id, config.is_primary, config.enabled);
        }

        // Rebuild the API with reloaded agents
        let mut api_builder = api::RustbotApiBuilder::new()
            .event_bus(Arc::clone(&event_bus))
            .runtime(Arc::clone(&self.runtime))
            .llm_adapter(Arc::clone(&llm_adapter))
            .max_history_size(20)
            .system_instructions(self.system_prompts.system_instructions.clone());

        for agent_config in &agent_configs {
            api_builder = api_builder.add_agent(agent_config.clone());
        }

        let api = api_builder.build().expect("Failed to rebuild RustbotApi");

        // Update app state with new components
        self.api = Arc::new(Mutex::new(api));
        self.event_rx = event_rx;
        self.agent_configs = agent_configs;

        // Clear conversation on reload
        self.clear_conversation();

        tracing::info!("✅ Configuration reloaded successfully");
    }

    fn send_message(&mut self, _ctx: &egui::Context) {
        if self.message_input.trim().is_empty() || self.is_waiting {
            return;
        }

        // Calculate input tokens early
        let input_tokens = self.estimate_tokens(&self.message_input);
        self.token_stats.daily_input += input_tokens;
        self.token_stats.total_input += input_tokens;
        let _ = self.save_token_stats();

        // Add user message to UI
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: self.message_input.clone(),
            input_tokens: Some(input_tokens),
            output_tokens: None,
        });

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            input_tokens: None,
            output_tokens: None,
        });

        self.is_waiting = true;
        self.current_response.clear();

        // Update context tracker
        let system_content_tokens = self.estimate_tokens(&self.generate_system_context());
        let conversation_total_tokens: u32 = self.messages.iter()
            .map(|msg| self.estimate_tokens(&msg.content))
            .sum();
        self.context_tracker.update_counts(system_content_tokens, conversation_total_tokens);

        // Call send_message - we use a channel to communicate the result back
        let message = self.message_input.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending_agent_result = Some(rx);

        // Spawn async task using tokio runtime
        // This is the proper way to call async code from sync UI thread
        let api = Arc::clone(&self.api);
        self.runtime.spawn(async move {
            // Lock the API, call send_message, then release lock
            let mut api_guard = api.lock().await;
            let result = api_guard.send_message(&message).await;
            let _ = tx.send(result);
        });

        // Clear input after processing
        self.message_input.clear();
    }

    fn handle_user_message_event(&mut self, _ctx: &egui::Context, content: String) {
        // Calculate input tokens
        let input_tokens = self.estimate_tokens(&content);
        self.token_stats.daily_input += input_tokens;
        self.token_stats.total_input += input_tokens;
        let _ = self.save_token_stats();

        // Add user message to UI
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            input_tokens: Some(input_tokens),
            output_tokens: None,
        });

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            input_tokens: None,
            output_tokens: None,
        });

        self.is_waiting = true;
        self.current_response.clear();

        // Update context tracker
        let system_content_tokens = self.estimate_tokens(&self.generate_system_context());
        let conversation_total_tokens: u32 = self.messages.iter()
            .map(|msg| self.estimate_tokens(&msg.content))
            .sum();
        self.context_tracker.update_counts(system_content_tokens, conversation_total_tokens);

        // Spawn async task to send message
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending_agent_result = Some(rx);

        // Spawn async task using tokio runtime
        let api = Arc::clone(&self.api);
        self.runtime.spawn(async move {
            // Lock the API, call send_message, then release lock
            let mut api_guard = api.lock().await;
            let result = api_guard.send_message(&content).await;
            let _ = tx.send(result);
        });
    }

}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process events from the event bus
        // Use a flag to track if we processed any events
        let mut events_processed = false;

        while let Ok(event) = self.event_rx.try_recv() {
            events_processed = true;

            // Track event for visualization (keep last 50 events)
            let event_kind_str = match &event.kind {
                EventKind::UserMessage(_) => "UserMessage".to_string(),
                EventKind::AgentMessage { .. } => "AgentMessage".to_string(),
                EventKind::AgentStatusChange { .. } => "StatusChange".to_string(),
                EventKind::SystemCommand(_) => "SystemCommand".to_string(),
                EventKind::McpPluginEvent(_) => "McpPlugin".to_string(),
                EventKind::Test(_) => "Test".to_string(),
            };

            // Push to back of deque (O(1) operation)
            self.event_history.push_back(VisualEvent {
                source: event.source.clone(),
                destination: event.destination.clone(),
                kind: event_kind_str,
                timestamp: event.timestamp,
            });

            // Keep only last 50 events - pop from front (O(1) operation)
            if self.event_history.len() > 50 {
                self.event_history.pop_front();
            }

            // Check if this event is for us (user or broadcast)
            if event.is_for("user") {
                match event.kind {
                    EventKind::UserMessage(content) => {
                        // Handle user message by calling LLM
                        self.handle_user_message_event(ctx, content);
                    }
                    EventKind::AgentMessage { agent_id, content } => {
                        tracing::info!("Received agent message from {}: {}", agent_id, content);
                        // Agent messages are already handled in streaming
                    }
                    EventKind::AgentStatusChange { agent_id, status } => {
                        tracing::info!("Agent {} status changed to {:?}", agent_id, status);

                        // Update current activity based on agent status
                        use events::AgentStatus;
                        self.current_activity = match status {
                            AgentStatus::ExecutingTool(ref tool_name) => {
                                Some(format!("🔧 Executing tool: {}", tool_name))
                            }
                            AgentStatus::Thinking => {
                                Some("🤔 Thinking...".to_string())
                            }
                            AgentStatus::Responding => {
                                Some("💬 Generating response...".to_string())
                            }
                            AgentStatus::Idle => None,
                            AgentStatus::Error(_) => None,
                        };
                    }
                    EventKind::SystemCommand(cmd) => {
                        match cmd {
                            SystemCommand::ClearConversation => {
                                // DON'T call self.clear_conversation() here - that would create an infinite loop!
                                // The UI already cleared its state when the Clear button was clicked.
                                // This event is for other components (agents, etc.) to know the conversation was cleared.
                                tracing::debug!("ClearConversation event received (already handled)");
                            }
                            SystemCommand::SaveState => {
                                tracing::info!("Save state command received");
                            }
                            SystemCommand::LoadState => {
                                tracing::info!("Load state command received");
                            }
                        }
                    }
                    EventKind::McpPluginEvent(plugin_event) => {
                        tracing::info!("MCP plugin event received: {:?}", plugin_event);

                        // Forward event to plugins view for display
                        if let Some(plugins_view) = &mut self.plugins_view {
                            plugins_view.handle_mcp_event(&plugin_event);
                        }
                    }
                    EventKind::Test(msg) => {
                        tracing::info!("Test event received: {}", msg);
                    }
                }
            }
        }

        // Request immediate repaint if we processed any events
        // This ensures the event visualizer updates immediately
        if events_processed {
            ctx.request_repaint();
        }

        // Update spinner rotation when waiting
        if self.is_waiting {
            self.spinner_rotation += 0.1;
            ctx.request_repaint();
        }

        // Handle keyboard shortcuts
        ctx.input(|i| {
            // Cmd+R (macOS) or Ctrl+R (Windows/Linux) to reload configuration
            if i.modifiers.command && i.key_pressed(egui::Key::R) {
                self.reload_config();
            }
        });

        // Check for pending agent result (from non-blocking async task)
        if let Some(result_rx) = &mut self.pending_agent_result {
            match result_rx.try_recv() {
                Ok(result) => {
                    tracing::info!("Received agent processing result");
                    // Agent processing completed, handle result
                    match result {
                        Ok(rx) => {
                            tracing::info!("Agent processing succeeded, starting stream");
                            // Successfully got response stream, start receiving chunks
                            self.response_rx = Some(rx);
                            self.pending_agent_result = None; // Clear the pending result
                            ctx.request_repaint();
                        }
                        Err(e) => {
                            // Error occurred during agent processing
                            tracing::error!("Failed to process message through agent: {}", e);
                            self.is_waiting = false;

                            // Add error message visible to user
                            if let Some(last_msg) = self.messages.last_mut() {
                                last_msg.content = format!("⚠️ Error: {}\n\nPlease try again or check your connection.", e);
                            }

                            self.pending_agent_result = None; // Clear the pending result
                            ctx.request_repaint();
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // Still waiting for result, request repaint to check again
                    ctx.request_repaint();
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    tracing::error!("Agent result channel disconnected unexpectedly");
                    self.is_waiting = false;
                    self.pending_agent_result = None;

                    // Add error message
                    if let Some(last_msg) = self.messages.last_mut() {
                        last_msg.content = "⚠️ Error: Agent processing failed unexpectedly.\n\nPlease try again.".to_string();
                    }

                    ctx.request_repaint();
                }
            }
        }

        // Check for streaming responses
        if let Some(rx) = &mut self.response_rx {
            while let Ok(chunk) = rx.try_recv() {
                self.current_response.push_str(&chunk);

                // Update the last message (assistant response)
                if let Some(last_msg) = self.messages.last_mut() {
                    last_msg.content = self.current_response.clone();
                }

                ctx.request_repaint(); // Request repaint for each chunk
            }

            // Check if stream is done
            if rx.is_closed() && !self.current_response.is_empty() {
                // Calculate output tokens for the completed response
                let output_tokens = self.estimate_tokens(&self.current_response);
                self.token_stats.daily_output += output_tokens;
                self.token_stats.total_output += output_tokens;

                // Save stats after updating
                let _ = self.save_token_stats();

                // Update the last message with token count
                if let Some(last_msg) = self.messages.last_mut() {
                    last_msg.output_tokens = Some(output_tokens);
                }

                // Add assistant response to API's message history
                // This ensures the next message will have this response as context
                let api = Arc::clone(&self.api);
                let response = self.current_response.clone();
                self.runtime.spawn(async move {
                    let mut api_guard = api.lock().await;
                    api_guard.add_assistant_response(response);
                });

                self.response_rx = None;
                self.current_response.clear();
                self.is_waiting = false;
            }
        }

        // Set custom theme with larger fonts
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
        ].into();

        // Custom light color scheme
        let mut visuals = egui::Visuals::light();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(245, 245, 247);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(240, 240, 242);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(230, 230, 235);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 120, 220);
        visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(60, 120, 220, 80);
        visuals.extreme_bg_color = egui::Color32::from_rgb(250, 250, 252);
        visuals.panel_fill = egui::Color32::from_rgb(248, 248, 250);
        visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);

        style.visuals = visuals;
        ctx.set_style(style);

        // Sidebar panel
        if self.sidebar_open {
            egui::SidePanel::left("sidebar")
                .resizable(false)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Sidebar header with toggle
                        ui.horizontal(|ui| {
                            ui.heading("Menu");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(icons::CARET_LEFT).clicked() {
                                    self.sidebar_open = false;
                                }
                            });
                        });
                        ui.separator();

                        // Menu items (left-justified)
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let chat_button = ui.add(
                                egui::SelectableLabel::new(
                                    self.current_view == AppView::Chat,
                                    format!("{} Chat", icons::CHATS_CIRCLE)
                                )
                            );
                            if chat_button.clicked() {
                                self.current_view = AppView::Chat;
                            }
                        });

                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            let settings_button = ui.add(
                                egui::SelectableLabel::new(
                                    self.current_view == AppView::Settings,
                                    format!("{} Settings", icons::GEAR)
                                )
                            );
                            if settings_button.clicked() {
                                self.current_view = AppView::Settings;
                            }
                        });

                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            let plugins_button = ui.add(
                                egui::SelectableLabel::new(
                                    self.current_view == AppView::Plugins,
                                    format!("{} Plugins", icons::PUZZLE_PIECE)
                                )
                            );
                            if plugins_button.clicked() {
                                self.current_view = AppView::Plugins;
                            }
                        });

                        ui.add_space(5.0);

                        // Events button
                        ui.horizontal(|ui| {
                            let events_button = ui.add(
                                egui::SelectableLabel::new(
                                    self.current_view == AppView::Events,
                                    format!("{} Events", icons::LIST_BULLETS)
                                )
                            );
                            if events_button.clicked() {
                                self.current_view = AppView::Events;
                            }
                        });

                        ui.add_space(5.0);

                        // Reload configuration button
                        ui.horizontal(|ui| {
                            if ui.button(format!("{} Reload Config", icons::ARROW_CLOCKWISE)).clicked() {
                                self.reload_config();
                            }
                            ui.label(egui::RichText::new("⌘R")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(120, 120, 120)));
                        });

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Event Visualizer section
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Event Flow").strong().size(14.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button(if self.show_event_visualizer { "▼" } else { "▶" }).clicked() {
                                    self.show_event_visualizer = !self.show_event_visualizer;
                                }
                            });
                        });

                        if self.show_event_visualizer {
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    if self.event_history.is_empty() {
                                        ui.label(egui::RichText::new("No events yet")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(120, 120, 120)));
                                    } else {
                                        // Show most recent events first
                                        for event in self.event_history.iter().rev().take(10) {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());

                                                // Event kind with color coding
                                                let (color, icon) = match event.kind.as_str() {
                                                    "UserMessage" => (egui::Color32::from_rgb(100, 150, 255), "📤"),
                                                    "AgentMessage" => (egui::Color32::from_rgb(100, 255, 150), "📥"),
                                                    "StatusChange" => (egui::Color32::from_rgb(255, 200, 100), "🔄"),
                                                    "SystemCommand" => (egui::Color32::from_rgb(255, 100, 100), "⚙️"),
                                                    _ => (egui::Color32::from_rgb(150, 150, 150), "📨"),
                                                };

                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(icon).size(10.0));
                                                    ui.label(
                                                        egui::RichText::new(&event.kind)
                                                            .size(10.0)
                                                            .color(color)
                                                    );
                                                });

                                                // Source → Destination
                                                ui.label(
                                                    egui::RichText::new(format!("{} → {}", event.source, event.destination))
                                                        .size(9.0)
                                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                                );

                                                // Timestamp
                                                ui.label(
                                                    egui::RichText::new(event.timestamp.format("%H:%M:%S").to_string())
                                                        .size(8.0)
                                                        .color(egui::Color32::from_rgb(100, 100, 100))
                                                );
                                            });
                                            ui.add_space(3.0);
                                        }
                                    }
                                });
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header at top with toggle button and version info
                ui.horizontal(|ui| {
                    // Sidebar toggle button (hamburger menu)
                    if !self.sidebar_open {
                        if ui.button(icons::LIST).clicked() {
                            self.sidebar_open = true;
                        }
                    }

                    ui.heading("Rustbot - AI Assistant");
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(version::version_string())
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));
                });
                ui.separator();

                // Render different views based on current_view
                match self.current_view {
                    AppView::Chat => self.render_chat_view(ui, ctx),
                    AppView::Settings => self.render_settings_view(ui),
                    AppView::Plugins => self.render_plugins_view(ui, ctx),
                    AppView::Events => self.render_events_view(ui),
                }
            });
        });
    }
}
