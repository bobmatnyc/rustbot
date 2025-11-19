mod agent;
mod agents;
mod api;
mod app_builder;
mod error;
mod events;
mod llm;
mod mcp;
mod mermaid;
mod services;
mod tool_executor;
mod ui;
mod version;

use agent::AgentConfig;
use api::RustbotApi;
use app_builder::{AppBuilder, AppDependencies};
use eframe::egui;
use egui_commonmark::CommonMarkCache;
use egui_phosphor::regular as icons;
use error::{Result, RustbotError};
use events::{Event, EventBus, EventKind, SystemCommand};
use llm::{create_adapter, AdapterType, LlmAdapter};
use mcp::manager::McpPluginManager;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use ui::icon::create_window_icon;
use ui::{
    AppView, ChatMessage, ContextTracker, ExtensionsView, MessageRole, PluginsView, SettingsView,
    SystemPrompts, TokenStats, VisualEvent,
};

/// Read a secret from 1Password using the CLI
///
/// # Arguments
/// * `reference` - 1Password secret reference (format: `op://vault/item/field`)
///
/// # Returns
/// * `Ok(String)` - The secret value
/// * `Err(anyhow::Error)` - If reading fails
///
/// # Errors
/// - 1Password CLI not installed
/// - Not signed in to 1Password
/// - Secret reference not found
/// - Invalid reference format
fn read_1password_secret(reference: &str) -> anyhow::Result<String> {
    use anyhow::Context;

    // Validate reference format
    if !reference.starts_with("op://") {
        anyhow::bail!(
            "Invalid 1Password reference format: '{}'. Must start with 'op://'",
            reference
        );
    }

    // Execute `op read` command
    let output = Command::new("op")
        .arg("read")
        .arg(reference)
        .output()
        .with_context(|| {
            format!(
                "Failed to execute 1Password CLI. Is it installed?\n\
                 Install: brew install 1password-cli\n\
                 Reference: {}",
                reference
            )
        })?;

    // Check if command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Provide helpful error messages based on common failures
        if stderr.contains("not currently signed in") || stderr.contains("signed out") {
            anyhow::bail!(
                "Not signed in to 1Password. Run: op signin\n\
                 Reference: {}",
                reference
            );
        } else if stderr.contains("isn't an item") || stderr.contains("not found") {
            anyhow::bail!(
                "1Password secret not found: {}\n\
                 Error: {}",
                reference,
                stderr.trim()
            );
        } else {
            anyhow::bail!(
                "Failed to read 1Password secret: {}\n\
                 Error: {}",
                reference,
                stderr.trim()
            );
        }
    }

    // Parse output
    let secret = String::from_utf8(output.stdout)
        .with_context(|| format!("1Password returned invalid UTF-8 for: {}", reference))?
        .trim()
        .to_string();

    // Ensure secret is not empty
    if secret.is_empty() {
        anyhow::bail!("1Password secret is empty: {}", reference);
    }

    Ok(secret)
}

/// Resolve API key from environment variable or 1Password reference
///
/// Supports two formats:
/// 1. `op://vault/item/field` - 1Password secret reference
/// 2. Plain API key - Returned as-is
///
/// # Arguments
/// * `value` - The environment variable value to resolve
///
/// # Returns
/// * `Ok(String)` - The resolved API key
/// * `Err(anyhow::Error)` - If resolution fails
fn resolve_api_key(value: &str) -> anyhow::Result<String> {
    // If it's a 1Password reference, resolve it
    if value.starts_with("op://") {
        return read_1password_secret(value);
    }

    // Otherwise return as-is (plain API key)
    Ok(value.to_string())
}

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

    // Get API key with proper error handling to avoid panic in FFI boundary
    // If not found, we'll show setup wizard instead of exiting
    // Also resolve 1Password references (op://...) if present
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(key_ref) => {
            // Try to resolve the key (handles both plain keys and 1Password references)
            match resolve_api_key(&key_ref) {
                Ok(resolved_key) => {
                    tracing::info!("✓ API key loaded successfully");
                    resolved_key
                }
                Err(e) => {
                    // Log error but don't exit - we'll show setup wizard instead
                    tracing::error!("Failed to resolve API key: {}", e);
                    eprintln!("\n❌ ERROR: Failed to resolve OPENROUTER_API_KEY");
                    eprintln!("\nError details: {}", e);
                    eprintln!("\nPossible solutions:");
                    eprintln!("  - If using 1Password: Ensure 1Password CLI is installed (brew install 1password-cli)");
                    eprintln!("  - If using 1Password: Sign in with 'op signin'");
                    eprintln!("  - If using 1Password: Verify the reference is correct (op://vault/item/field)");
                    eprintln!("  - Or set a plain API key in .env.local");
                    eprintln!("\nWill show setup wizard to configure API key...\n");
                    String::new() // Empty string triggers setup wizard
                }
            }
        }
        Err(_) => {
            tracing::warn!("OPENROUTER_API_KEY not found - will show setup wizard");
            String::new()
        }
    };

    // Build dependencies using AppBuilder
    let deps = tokio::runtime::Runtime::new()
        .expect("Failed to create runtime")
        .block_on(async {
            AppBuilder::new()
                .with_api_key(api_key.clone())
                .with_base_path(std::path::PathBuf::from("."))
                .with_production_deps()
                .await
                .expect("Failed to build dependencies")
                .build()
                .expect("Failed to finalize dependencies")
        });

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
        Box::new(move |cc| {
            // Install SVG image loader for rendering Mermaid diagrams
            egui_extras::install_image_loaders(&cc.egui_ctx);

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

            Ok(Box::new(RustbotApp::new(deps, api_key)))
        }),
    )
}

struct RustbotApp {
    // Injected dependencies (service layer)
    deps: AppDependencies,

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
    current_activity: Option<String>, // Track current agent activity

    // Event visualization
    event_rx: broadcast::Receiver<Event>,
    event_history: VecDeque<VisualEvent>,
    show_event_visualizer: bool,

    // Agent UI state
    agent_configs: Vec<AgentConfig>,
    selected_agent_index: Option<usize>,

    // Pending agent result receiver
    pending_agent_result:
        Option<mpsc::UnboundedReceiver<anyhow::Result<mpsc::UnboundedReceiver<String>>>>,

    // MCP Plugin Manager and UI
    mcp_manager: Arc<Mutex<McpPluginManager>>,
    plugins_view: Option<PluginsView>,
    extensions_marketplace_view: Option<ui::MarketplaceView>,
    extensions_view: ExtensionsView,

    // Extension configuration state
    configuring_extension_id: Option<String>,
    extension_config_message: Option<(String, bool)>, // (message, is_error)
    installed_extensions_filter: ui::InstallTypeFilter, // Filter for installed extensions view

    // Extension uninstall state
    uninstall_confirmation: Option<(String, String)>, // (extension_id, extension_name)
    uninstall_message: Option<(String, bool)>,        // (message, is_error)

    // Markdown rendering
    markdown_cache: CommonMarkCache,

    // Mermaid diagram rendering
    mermaid_renderer: Arc<Mutex<mermaid::MermaidRenderer>>,

    // Splash screen state
    show_splash: bool,
    splash_start_time: Option<std::time::Instant>,

    // Setup wizard state
    setup_wizard_active: bool,
    setup_wizard_step: SetupWizardStep,
    setup_name: String,
    setup_email: String,
    setup_api_key: String,
}

/// Setup wizard flow steps
#[derive(Debug, Clone, PartialEq)]
enum SetupWizardStep {
    Welcome,
    EnterName,
    EnterEmail,
    EnterApiKey,
    Complete,
}

impl RustbotApp {
    fn new(deps: AppDependencies, api_key: String) -> Self {
        // Get runtime from dependencies (required)
        let runtime = deps
            .runtime
            .as_ref()
            .expect("Runtime is required for RustbotApp");

        // Load persisted state (UI-specific types, not from service layer)
        // Note: TokenStats and SystemPrompts are UI-specific types with different
        // structure from the service layer types, so we handle them directly
        let token_stats = Self::load_token_stats().unwrap_or_default();
        let system_prompts = Self::load_system_prompts().unwrap_or_default();

        // Subscribe to event bus
        let event_rx = deps.event_bus.subscribe();

        // Get LLM adapter from dependencies (required)
        let llm_adapter = deps
            .llm_adapter
            .as_ref()
            .expect("LLM adapter is required for RustbotApp");

        // Load agents from config service
        let mut agent_configs = runtime.block_on(async {
            deps.config.load_agent_configs().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to load agents from config service: {}", e);
                vec![]
            })
        });

        // If no agents loaded, fall back to default assistant
        if agent_configs.is_empty() {
            tracing::info!("No agents loaded, using default assistant");
            agent_configs.push(AgentConfig::default_assistant());
        }

        // Build the API using RustbotApiBuilder with all loaded agents
        let mut api_builder = api::RustbotApiBuilder::new()
            .event_bus(Arc::clone(&deps.event_bus))
            .runtime(Arc::clone(runtime))
            .llm_adapter(Arc::clone(llm_adapter))
            .max_history_size(20)
            .system_instructions(system_prompts.system_instructions.clone());

        // Add all loaded agents
        for agent_config in &agent_configs {
            api_builder = api_builder.add_agent(agent_config.clone());
        }

        let api = api_builder.build().expect("Failed to build RustbotApi");

        // Initialize MCP plugin manager with event bus
        let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
            Arc::clone(&deps.event_bus),
        ))));

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
            runtime.handle().clone(),
        ));

        // Create marketplace view with runtime handle
        let extensions_marketplace_view = Some(ui::MarketplaceView::new(runtime.handle().clone()));

        // Create mermaid renderer
        let mermaid_renderer = Arc::new(Mutex::new(mermaid::MermaidRenderer::new()));

        // Check if this is first run (no profile exists and/or no API key in env)
        let profile_exists = runtime.block_on(async {
            let profile = deps.storage.load_user_profile().await.unwrap_or_default();
            !profile.name.is_empty() || !profile.email.is_empty()
        });

        let setup_wizard_active = !profile_exists || api_key.is_empty();

        Self {
            deps,
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
            current_activity: None,
            event_rx,
            agent_configs: agent_configs.clone(),
            selected_agent_index: None,
            event_history: VecDeque::with_capacity(50),
            show_event_visualizer: true, // Start with visualizer open for debugging
            pending_agent_result: None,
            mcp_manager,
            plugins_view,
            extensions_marketplace_view,
            extensions_view: ExtensionsView::default(),
            configuring_extension_id: None,
            extension_config_message: None,
            installed_extensions_filter: ui::InstallTypeFilter::default(),
            uninstall_confirmation: None,
            uninstall_message: None,
            markdown_cache: CommonMarkCache::default(),
            mermaid_renderer,
            show_splash: true,
            splash_start_time: Some(std::time::Instant::now()),
            setup_wizard_active,
            setup_wizard_step: SetupWizardStep::Welcome,
            setup_name: String::new(),
            setup_email: String::new(),
            setup_api_key: api_key.clone(),
        }
    }

    fn get_instructions_dir() -> Result<PathBuf> {
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                RustbotError::EnvError(
                    "Could not determine home directory: HOME or USERPROFILE not set".to_string(),
                )
            })?;

        let mut dir = PathBuf::from(home_dir);
        dir.push(".rustbot");
        dir.push("instructions");

        // Create directory if it doesn't exist
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                RustbotError::StorageError(format!(
                    "Failed to create instructions directory: {}",
                    e
                ))
            })?;
        }

        Ok(dir)
    }

    fn load_system_prompts() -> Result<SystemPrompts> {
        let mut dir = Self::get_instructions_dir()?;

        // Load system instructions
        dir.push("system");
        dir.push("current");
        let system_instructions = if dir.exists() {
            std::fs::read_to_string(&dir).map_err(|e| {
                RustbotError::StorageError(format!(
                    "Failed to read system instructions from {:?}: {}",
                    dir, e
                ))
            })?
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
        std::fs::create_dir_all(&system_dir).map_err(|e| {
            RustbotError::StorageError(format!("Failed to create system directory: {}", e))
        })?;

        let system_current = system_dir.join("current");

        // Create backup if current exists
        if system_current.exists() {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = system_dir.join(format!("backup_{}", timestamp));
            std::fs::copy(&system_current, &backup_path).map_err(|e| {
                RustbotError::StorageError(format!(
                    "Failed to create backup of system instructions: {}",
                    e
                ))
            })?;
        }

        // Write new current
        std::fs::write(&system_current, &self.system_prompts.system_instructions).map_err(|e| {
            RustbotError::StorageError(format!("Failed to write system instructions: {}", e))
        })?;

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

        let content = std::fs::read_to_string(&path).map_err(|e| {
            RustbotError::StorageError(format!("Failed to read token stats from {:?}: {}", path, e))
        })?;

        let stats: TokenStats = serde_json::from_str(&content).map_err(|e| {
            RustbotError::StorageError(format!("Failed to parse token stats JSON: {}", e))
        })?;

        Ok(stats)
    }

    fn save_token_stats(&self) -> Result<()> {
        let path = Self::get_stats_file_path();
        let content = serde_json::to_string_pretty(&self.token_stats).map_err(|e| {
            RustbotError::StorageError(format!("Failed to serialize token stats: {}", e))
        })?;

        std::fs::write(&path, content).map_err(|e| {
            RustbotError::StorageError(format!("Failed to write token stats to {:?}: {}", path, e))
        })?;

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

        // Get the primary agent's model
        let model = self
            .agent_configs
            .iter()
            .find(|config| config.is_primary)
            .map(|config| config.model.as_str())
            .unwrap_or("unknown");

        // Load user profile synchronously (blocking on async)
        let profile = if let Some(runtime) = &self.deps.runtime {
            runtime.block_on(async {
                self.deps
                    .storage
                    .load_user_profile()
                    .await
                    .unwrap_or_default()
            })
        } else {
            services::traits::UserProfile::default()
        };

        // Build user profile section if available
        let user_profile_section = if !profile.name.is_empty() || !profile.email.is_empty() {
            let mut section = String::new();
            if !profile.name.is_empty() {
                section.push_str(&format!("\n**User Name**: {}", profile.name));
            }
            if !profile.email.is_empty() {
                section.push_str(&format!("\n**User Email**: {}", profile.email));
            }
            if let Some(ref timezone) = profile.timezone {
                section.push_str(&format!("\n**User Timezone**: {}", timezone));
            }
            if let Some(ref location) = profile.location {
                section.push_str(&format!("\n**User Location**: {}", location));
            }
            section
        } else {
            String::new()
        };

        // Build the system context
        format!(
            r#"## System Context

**Current Date & Time**: {} ({})
**LLM Model**: {}
**Application**: Rustbot v{}
**Operating System**: {} ({})
**Hostname**: {}
**User**: {}{}

This information is provided automatically to give you context about the current system environment."#,
            datetime,
            day_of_week,
            model,
            version::version_string(),
            os,
            arch,
            hostname,
            user,
            user_profile_section
        )
    }

    fn clear_conversation(&mut self) {
        tracing::info!(
            "🗑️  Clearing conversation - UI messages: {}, Event history: {}",
            self.messages.len(),
            self.event_history.len()
        );

        // Clear UI state
        self.messages.clear();
        self.current_response.clear();
        self.context_tracker.update_counts(0, 0);

        // Clear event flow display
        self.event_history.clear();

        // Clear API conversation history and publish event
        let api = Arc::clone(&self.api);
        let runtime = self
            .deps
            .runtime
            .as_ref()
            .expect("Runtime is required for RustbotApp");
        runtime.spawn(async move {
            let mut api_guard = api.lock().await;
            api_guard.clear_history();
        });
    }

    fn reload_config(&mut self) {
        tracing::info!("🔄 Reloading Rustbot configuration...");

        // Get runtime from dependencies
        let runtime = self
            .deps
            .runtime
            .as_ref()
            .expect("Runtime is required for RustbotApp");

        // Reload agents from config service
        let agent_configs = runtime.block_on(async {
            self.deps
                .config
                .load_agent_configs()
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load agents from config service: {}", e);
                    vec![AgentConfig::default_assistant()]
                })
        });

        tracing::info!("📋 Reloaded {} agents", agent_configs.len());
        for config in &agent_configs {
            tracing::info!(
                "   - {} (primary: {}, enabled: {})",
                config.id,
                config.is_primary,
                config.enabled
            );
        }

        // Get LLM adapter from dependencies
        let llm_adapter = self
            .deps
            .llm_adapter
            .as_ref()
            .expect("LLM adapter is required for RustbotApp");

        // Subscribe to fresh event bus events
        let event_rx = self.deps.event_bus.subscribe();

        // Rebuild the API with reloaded agents
        let mut api_builder = api::RustbotApiBuilder::new()
            .event_bus(Arc::clone(&self.deps.event_bus))
            .runtime(Arc::clone(runtime))
            .llm_adapter(Arc::clone(llm_adapter))
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

    /// Extract all base64 image data URLs from markdown content
    ///
    /// This helper function finds all embedded images in the format:
    /// ![alt](data:image/jpeg;base64,...)
    ///
    /// # Arguments
    /// * `markdown` - The markdown content to search
    ///
    /// # Returns
    /// Vector of base64-encoded image data URLs
    fn extract_image_data_urls(markdown: &str) -> Vec<String> {
        use regex::Regex;
        let mut images = Vec::new();

        // Match data URL images: ![...](data:image/...;base64,...)
        let pattern = Regex::new(r#"!\[[^\]]*\]\((data:image/[^;]+;base64,[A-Za-z0-9+/=]+)\)"#)
            .expect("Invalid regex pattern");

        for cap in pattern.captures_iter(markdown) {
            if let Some(data_url) = cap.get(1) {
                images.push(data_url.as_str().to_string());
            }
        }

        images
    }

    /// Preprocess markdown content to render mermaid diagrams
    ///
    /// This method detects mermaid code blocks and replaces them with embedded SVG data.
    /// The replacement happens inline in the markdown, converting:
    /// ```mermaid
    /// graph TD
    ///   A-->B
    /// ```
    ///
    /// Into an embedded SVG image that egui_commonmark can display.
    ///
    /// # Arguments
    /// * `markdown` - The original markdown content with mermaid blocks
    ///
    /// # Returns
    /// Preprocessed markdown with mermaid diagrams replaced by SVG embeds
    fn preprocess_mermaid(&self, markdown: &str) -> String {
        // First, validate any existing base64 SVG images in the markdown
        // This handles cases where the LLM already generated base64 SVG images
        let mut result = markdown.to_string();

        // Look for base64 SVG images: ![...](data:image/svg+xml;base64,...)
        use regex::Regex;
        let base64_img_pattern =
            Regex::new(r#"!\[([^\]]*)\]\(data:image/svg\+xml;base64,([A-Za-z0-9+/=]+)\)"#).unwrap();

        // Validate and fix any base64 SVG images
        for cap in base64_img_pattern.captures_iter(markdown) {
            if let (Some(alt_text), Some(base64_data)) = (cap.get(1), cap.get(2)) {
                use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                // Try to decode the base64 to verify it's valid
                if let Ok(svg_bytes) = BASE64.decode(base64_data.as_str()) {
                    // Verify it's actually SVG
                    if let Ok(svg_str) = std::str::from_utf8(&svg_bytes[..svg_bytes.len().min(100)])
                    {
                        if svg_str.trim_start().starts_with("<?xml")
                            || svg_str.trim_start().starts_with("<svg")
                        {
                            tracing::debug!(
                                "✓ Found valid base64 SVG image in markdown ({} bytes)",
                                svg_bytes.len()
                            );
                            // It's valid, keep it as-is
                            continue;
                        }
                    }
                }
                tracing::warn!("Found invalid base64 SVG image, will remove it");
            }
        }

        // Then, extract and render mermaid code blocks
        let blocks = mermaid::extract_mermaid_blocks(&result);

        if blocks.is_empty() {
            return result;
        }

        let renderer = Arc::clone(&self.mermaid_renderer);
        let runtime = Arc::clone(
            self.deps
                .runtime
                .as_ref()
                .expect("Runtime is required for RustbotApp"),
        );

        // Process blocks in reverse order to maintain correct indices
        for (start, end, code) in blocks.iter().rev() {
            // Try to render the diagram as PNG (better compatibility with egui_commonmark)
            let png_result = runtime.block_on(async {
                if let Ok(mut r) = renderer.try_lock() {
                    r.render_to_png(code).await
                } else {
                    Err(mermaid::MermaidError::EncodingError(
                        "Renderer locked".to_string(),
                    ))
                }
            });

            match png_result {
                Ok(image_bytes) => {
                    // Convert image bytes to base64 data URL
                    // Note: mermaid.ink/img/ returns JPEG (with labels) instead of PNG (transparent)
                    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                    let img_base64 = BASE64.encode(&image_bytes);
                    let data_url = format!("data:image/jpeg;base64,{}", img_base64);

                    // Replace mermaid block with image markdown
                    // Note: egui_commonmark supports image syntax: ![alt](url)
                    let replacement = format!("![Mermaid Diagram]({})", data_url);

                    result.replace_range(*start..*end, &replacement);

                    tracing::debug!(
                        "✓ Rendered mermaid diagram ({} bytes JPEG)",
                        image_bytes.len()
                    );
                }
                Err(e) => {
                    // On error, leave the code block as-is (graceful degradation)
                    tracing::warn!("Failed to render mermaid diagram: {}", e);
                    // Optionally, we could add an error message:
                    // let error_msg = format!("```\nError rendering diagram: {}\n```", e);
                    // result.replace_range(*start..*end, &error_msg);
                }
            }
        }

        result
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
            embedded_images: Vec::new(), // User messages don't have embedded images
        });

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            input_tokens: None,
            output_tokens: None,
            embedded_images: Vec::new(), // Will be populated when content is set
        });

        self.is_waiting = true;
        self.current_response.clear();

        // Update context tracker
        let system_content_tokens = self.estimate_tokens(&self.generate_system_context());
        let conversation_total_tokens: u32 = self
            .messages
            .iter()
            .map(|msg| self.estimate_tokens(&msg.content))
            .sum();
        self.context_tracker
            .update_counts(system_content_tokens, conversation_total_tokens);

        // Call send_message - we use a channel to communicate the result back
        let message = self.message_input.clone();
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending_agent_result = Some(rx);

        // Spawn async task using tokio runtime
        // This is the proper way to call async code from sync UI thread
        let api = Arc::clone(&self.api);
        let runtime = self
            .deps
            .runtime
            .as_ref()
            .expect("Runtime is required for RustbotApp");
        runtime.spawn(async move {
            // Lock the API, call send_message, then release lock
            let mut api_guard = api.lock().await;
            let result = api_guard.send_message(&message).await;
            let _ = tx.send(result);
        });

        // Clear input after processing
        self.message_input.clear();
    }

    /// Render fullscreen splash screen with logo
    fn render_splash_screen(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.35);

                // Logo
                let logo = egui::Image::new(egui::include_image!("../assets/rustbot-icon.png"));
                ui.add(logo.fit_to_exact_size(egui::vec2(200.0, 200.0)));

                ui.add_space(30.0);

                // Title
                ui.heading(egui::RichText::new("Rustbot").size(48.0).strong());

                ui.add_space(10.0);
                ui.label(egui::RichText::new("AI Assistant").size(24.0));

                ui.add_space(40.0);

                // Loading animation
                ui.spinner();
            });
        });
    }

    /// Render setup wizard dialog
    fn render_setup_wizard(&mut self, ctx: &egui::Context) {
        egui::Window::new("Welcome to Rustbot!")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.add_space(20.0);

                match self.setup_wizard_step {
                    SetupWizardStep::Welcome => {
                        ui.vertical_centered(|ui| {
                            let logo =
                                egui::Image::new(egui::include_image!("../assets/rustbot-icon.png"));
                            ui.add(logo.fit_to_exact_size(egui::vec2(100.0, 100.0)));

                            ui.add_space(20.0);
                            ui.heading("Welcome to Rustbot!");
                            ui.add_space(10.0);
                            ui.label("Let's get you set up in just a few steps.");

                            ui.add_space(30.0);
                            if ui.button("Get Started").clicked() {
                                self.setup_wizard_step = SetupWizardStep::EnterName;
                            }
                        });
                    }

                    SetupWizardStep::EnterName => {
                        ui.heading("What's your name?");
                        ui.add_space(10.0);
                        ui.label("This helps personalize your experience.");
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.setup_name);
                        });

                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui.button("Next").clicked() && !self.setup_name.trim().is_empty() {
                                self.setup_wizard_step = SetupWizardStep::EnterEmail;
                            }
                        });
                    }

                    SetupWizardStep::EnterEmail => {
                        ui.heading("What's your email?");
                        ui.add_space(10.0);
                        ui.label("Optional, but helps with context.");
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.label("Email:");
                            ui.text_edit_singleline(&mut self.setup_email);
                        });

                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui.button("Back").clicked() {
                                self.setup_wizard_step = SetupWizardStep::EnterName;
                            }
                            if ui.button("Next").clicked() {
                                self.setup_wizard_step = SetupWizardStep::EnterApiKey;
                            }
                        });
                    }

                    SetupWizardStep::EnterApiKey => {
                        ui.heading("OpenRouter API Key");
                        ui.add_space(10.0);
                        ui.label("Get your free API key from:");
                        ui.hyperlink("https://openrouter.ai/keys");
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.label("API Key:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.setup_api_key).password(true),
                            );
                        });

                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui.button("Back").clicked() {
                                self.setup_wizard_step = SetupWizardStep::EnterEmail;
                            }
                            if ui.button("Finish Setup").clicked()
                                && !self.setup_api_key.trim().is_empty()
                            {
                                self.save_setup_wizard_results();
                                self.setup_wizard_step = SetupWizardStep::Complete;
                                self.setup_wizard_active = false;
                            }
                        });
                    }

                    SetupWizardStep::Complete => {
                        ui.vertical_centered(|ui| {
                            ui.heading("All Set!");
                            ui.add_space(10.0);
                            ui.label("Rustbot is ready to use.");
                        });
                    }
                }

                ui.add_space(20.0);
            });
    }

    /// Save setup wizard results to storage
    fn save_setup_wizard_results(&self) {
        // Save user profile
        let profile = services::traits::UserProfile {
            name: self.setup_name.clone(),
            email: self.setup_email.clone(),
            timezone: None,
            location: None,
        };

        let storage = Arc::clone(&self.deps.storage);
        if let Some(runtime) = &self.deps.runtime {
            runtime.spawn(async move {
                let _ = storage.save_user_profile(&profile).await;
            });
        }

        // Save API key to .env.local
        let env_path = std::path::PathBuf::from(".env.local");
        let env_content = format!("OPENROUTER_API_KEY={}", self.setup_api_key);
        let _ = std::fs::write(&env_path, env_content);
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
            embedded_images: Vec::new(), // User messages don't have embedded images
        });

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            input_tokens: None,
            output_tokens: None,
            embedded_images: Vec::new(), // Will be populated when content is set
        });

        self.is_waiting = true;
        self.current_response.clear();

        // Update context tracker
        let system_content_tokens = self.estimate_tokens(&self.generate_system_context());
        let conversation_total_tokens: u32 = self
            .messages
            .iter()
            .map(|msg| self.estimate_tokens(&msg.content))
            .sum();
        self.context_tracker
            .update_counts(system_content_tokens, conversation_total_tokens);

        // Spawn async task to send message
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending_agent_result = Some(rx);

        // Spawn async task using tokio runtime
        let api = Arc::clone(&self.api);
        let runtime = self
            .deps
            .runtime
            .as_ref()
            .expect("Runtime is required for RustbotApp");
        runtime.spawn(async move {
            // Lock the API, call send_message, then release lock
            let mut api_guard = api.lock().await;
            let result = api_guard.send_message(&content).await;
            let _ = tx.send(result);
        });
    }
}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check splash screen timer (show for 2 seconds)
        if self.show_splash {
            if let Some(start) = self.splash_start_time {
                if start.elapsed().as_secs() < 2 {
                    self.render_splash_screen(ctx);
                    ctx.request_repaint();
                    return;
                } else {
                    self.show_splash = false;
                    self.splash_start_time = None;
                }
            }
        }

        // Setup wizard check - show after splash
        if self.setup_wizard_active {
            self.render_setup_wizard(ctx);
            return;
        }

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
                            AgentStatus::Thinking => Some("🤔 Thinking...".to_string()),
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
                                tracing::debug!(
                                    "ClearConversation event received (already handled)"
                                );
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
            // Cmd+Q (macOS) or Ctrl+Q (Windows/Linux) to quit application
            if i.modifiers.command && i.key_pressed(egui::Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

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
                                last_msg.content = format!(
                                    "⚠️ Error: {}\n\nPlease try again or check your connection.",
                                    e
                                );
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
                        last_msg.content =
                            "⚠️ Error: Agent processing failed unexpectedly.\n\nPlease try again."
                                .to_string();
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

                // Preprocess mermaid diagrams in the response once when content is finalized
                let preprocessed_content = self.preprocess_mermaid(&self.current_response);

                // Update the last message with token count and preprocessed content
                if let Some(last_msg) = self.messages.last_mut() {
                    last_msg.output_tokens = Some(output_tokens);
                    last_msg.content = preprocessed_content.clone();
                    // Extract embedded image data URLs for easy access
                    last_msg.embedded_images = Self::extract_image_data_urls(&preprocessed_content);
                }

                // Add assistant response to API's message history
                // This ensures the next message will have this response as context
                let api = Arc::clone(&self.api);
                let response = self.current_response.clone();
                let runtime = self
                    .deps
                    .runtime
                    .as_ref()
                    .expect("Runtime is required for RustbotApp");
                runtime.spawn(async move {
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
            (
                egui::TextStyle::Heading,
                egui::FontId::new(24.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();

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
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button(icons::CARET_LEFT).clicked() {
                                        self.sidebar_open = false;
                                    }
                                },
                            );
                        });
                        ui.separator();

                        // Menu items (left-justified)
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let chat_button = ui.add(egui::SelectableLabel::new(
                                self.current_view == AppView::Chat,
                                format!("{} Chat", icons::CHATS_CIRCLE),
                            ));
                            if chat_button.clicked() {
                                self.current_view = AppView::Chat;
                            }
                        });

                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            let settings_button = ui.add(egui::SelectableLabel::new(
                                self.current_view == AppView::Settings,
                                format!("{} Settings", icons::GEAR),
                            ));
                            if settings_button.clicked() {
                                self.current_view = AppView::Settings;
                            }
                        });

                        ui.add_space(5.0);

                        // Events button
                        ui.horizontal(|ui| {
                            let events_button = ui.add(egui::SelectableLabel::new(
                                self.current_view == AppView::Events,
                                format!("{} Events", icons::LIST_BULLETS),
                            ));
                            if events_button.clicked() {
                                self.current_view = AppView::Events;
                            }
                        });

                        ui.add_space(5.0);

                        // Extensions button (was Marketplace)
                        ui.horizontal(|ui| {
                            let extensions_button = ui.add(egui::SelectableLabel::new(
                                self.current_view == AppView::Extensions,
                                format!("{} Extensions", icons::PUZZLE_PIECE),
                            ));
                            if extensions_button.clicked() {
                                self.current_view = AppView::Extensions;
                            }
                        });

                        ui.add_space(5.0);

                        // Reload configuration button
                        ui.horizontal(|ui| {
                            if ui
                                .button(format!("{} Reload Config", icons::ARROW_CLOCKWISE))
                                .clicked()
                            {
                                self.reload_config();
                            }
                            ui.label(
                                egui::RichText::new("⌘R")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                        });

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Event Visualizer section
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Event Flow").strong().size(14.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(if self.show_event_visualizer {
                                            "▼"
                                        } else {
                                            "▶"
                                        })
                                        .clicked()
                                    {
                                        self.show_event_visualizer = !self.show_event_visualizer;
                                    }
                                },
                            );
                        });

                        if self.show_event_visualizer {
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    if self.event_history.is_empty() {
                                        ui.label(
                                            egui::RichText::new("No events yet")
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(120, 120, 120)),
                                        );
                                    } else {
                                        // Show most recent events first
                                        for event in self.event_history.iter().rev().take(10) {
                                            ui.group(|ui| {
                                                ui.set_width(ui.available_width());

                                                // Event kind with color coding
                                                let (color, icon) = match event.kind.as_str() {
                                                    "UserMessage" => (
                                                        egui::Color32::from_rgb(100, 150, 255),
                                                        "📤",
                                                    ),
                                                    "AgentMessage" => (
                                                        egui::Color32::from_rgb(100, 255, 150),
                                                        "📥",
                                                    ),
                                                    "StatusChange" => (
                                                        egui::Color32::from_rgb(255, 200, 100),
                                                        "🔄",
                                                    ),
                                                    "SystemCommand" => (
                                                        egui::Color32::from_rgb(255, 100, 100),
                                                        "⚙️",
                                                    ),
                                                    _ => (
                                                        egui::Color32::from_rgb(150, 150, 150),
                                                        "📨",
                                                    ),
                                                };

                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(icon).size(10.0));
                                                    ui.label(
                                                        egui::RichText::new(&event.kind)
                                                            .size(10.0)
                                                            .color(color),
                                                    );
                                                });

                                                // Source → Destination
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} → {}",
                                                        event.source, event.destination
                                                    ))
                                                    .size(9.0)
                                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                                                );

                                                // Timestamp
                                                ui.label(
                                                    egui::RichText::new(
                                                        event
                                                            .timestamp
                                                            .format("%H:%M:%S")
                                                            .to_string(),
                                                    )
                                                    .size(8.0)
                                                    .color(egui::Color32::from_rgb(100, 100, 100)),
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
                    ui.label(
                        egui::RichText::new(version::version_string())
                            .size(14.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                    );
                });
                ui.separator();

                // Render different views based on current_view
                match self.current_view {
                    AppView::Chat => self.render_chat_view(ui, ctx),
                    AppView::Settings => self.render_settings_view(ui),
                    AppView::Events => self.render_events_view(ui),
                    AppView::Extensions => self.render_extensions_view(ui, ctx),
                }
            });
        });
    }
}
