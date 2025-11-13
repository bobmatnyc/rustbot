mod agent;
mod events;
mod llm;
mod version;

use agent::{Agent, AgentConfig};
use eframe::egui;
use events::{Event, EventBus, EventKind, SystemCommand};
use llm::{create_adapter, AdapterType, LlmAdapter, LlmRequest, Message as LlmMessage};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use egui_phosphor::regular as icons;

fn create_window_icon() -> egui::IconData {
    // Create a 32x32 RGBA icon with an "R" for Rustbot
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];

    // Background color: Rust orange (#CE422B)
    let bg_color = [206, 66, 43, 255];

    // Fill background
    for i in 0..size * size {
        let offset = i * 4;
        rgba[offset..offset + 4].copy_from_slice(&bg_color);
    }

    // Helper function to set a pixel
    let set_pixel = |rgba: &mut [u8], x: usize, y: usize, color: [u8; 4]| {
        if x < size && y < size {
            let offset = (y * size + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    };

    // Draw a bold "R" in white (3 pixels wide for visibility)
    let white = [255, 255, 255, 255];

    // Vertical line (left side of R) - 3 pixels wide
    for y in 8..24 {
        for dx in 0..3 {
            set_pixel(&mut rgba, 10 + dx, y, white);
        }
    }

    // Top horizontal line - 3 pixels tall
    for x in 10..20 {
        for dy in 0..3 {
            set_pixel(&mut rgba, x, 8 + dy, white);
        }
    }

    // Top right curve/vertical
    for y in 8..16 {
        for dx in 0..3 {
            set_pixel(&mut rgba, 19 + dx, y, white);
        }
    }

    // Middle horizontal line - 3 pixels tall
    for x in 10..20 {
        for dy in 0..3 {
            set_pixel(&mut rgba, x, 15 + dy, white);
        }
    }

    // Diagonal leg (bottom right) - 3 pixels wide
    for i in 0..8 {
        for dx in 0..3 {
            for dy in 0..3 {
                set_pixel(&mut rgba, 16 + i + dx, 16 + i + dy, white);
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Load .env.local file
    dotenvy::from_filename(".env.local").ok();

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
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/Roboto-Regular.ttf"
                )),
            );

            // Load Roboto Bold
            fonts.font_data.insert(
                "Roboto-Bold".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/Roboto-Bold.ttf"
                )),
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

            // Apply fonts
            cc.egui_ctx.set_fonts(fonts);

            let api_key = std::env::var("OPENROUTER_API_KEY")
                .expect("OPENROUTER_API_KEY not found in .env.local");
            Ok(Box::new(RustbotApp::new(api_key)))
        }),
    )
}

struct RustbotApp {
    message_input: String,
    messages: Vec<ChatMessage>,
    llm_adapter: Arc<dyn LlmAdapter>,
    response_rx: Option<mpsc::UnboundedReceiver<String>>,
    current_response: String,
    is_waiting: bool,
    runtime: Arc<tokio::runtime::Runtime>,
    spinner_rotation: f32,
    token_stats: TokenStats,
    context_tracker: ContextTracker,
    sidebar_open: bool,
    current_view: AppView,
    settings_view: SettingsView,
    system_prompts: SystemPrompts,
    selected_model: String,
    // Event system
    event_bus: Arc<EventBus>,
    event_rx: broadcast::Receiver<Event>,
    // Agent system
    agents: Vec<Agent>,
    active_agent_id: String,
    // Agent UI state
    agent_configs: Vec<AgentConfig>,
    selected_agent_index: Option<usize>,
    // Event visualization
    event_history: Vec<VisualEvent>,
    show_event_visualizer: bool,
}

/// Visual representation of an event for the event visualizer
#[derive(Clone, Debug)]
struct VisualEvent {
    source: String,
    destination: String,
    kind: String,
    timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(PartialEq)]
enum AppView {
    Chat,
    Settings,
}

#[derive(PartialEq, Clone)]
enum SettingsView {
    AiSettings,
    SystemPrompts,
    Agents,
}

#[derive(Serialize, Deserialize, Clone)]
struct SystemPrompts {
    system_instructions: String,
    personality_instructions: String,
}

impl Default for SystemPrompts {
    fn default() -> Self {
        Self {
            system_instructions: "You are a helpful AI assistant.".to_string(),
            personality_instructions: "Be concise, friendly, and professional.".to_string(),
        }
    }
}

struct ChatMessage {
    role: MessageRole,
    content: String,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct TokenStats {
    daily_input: u32,
    daily_output: u32,
    total_input: u32,
    total_output: u32,
    #[serde(default)]
    last_reset_date: String, // Track when daily stats were last reset
}

#[derive(Clone)]
struct ContextTracker {
    max_tokens: u32,              // Model's context window (200k for Claude Sonnet 4.5)
    current_tokens: u32,          // Current estimated token usage
    system_tokens: u32,           // Tokens used by system context (dynamic)
    conversation_tokens: u32,     // Tokens used by conversation history
    compaction_threshold: f32,    // Trigger compaction (default: 0.50 = 50%)
    warning_threshold: f32,       // Show warning (default: 0.75 = 75%)
}

impl Default for ContextTracker {
    fn default() -> Self {
        Self {
            max_tokens: 200_000,      // Claude Sonnet 4.5 context window
            current_tokens: 0,
            system_tokens: 0,
            conversation_tokens: 0,
            compaction_threshold: 0.50,
            warning_threshold: 0.75,
        }
    }
}

impl ContextTracker {
    fn usage_percentage(&self) -> f32 {
        if self.max_tokens == 0 {
            0.0
        } else {
            (self.current_tokens as f32 / self.max_tokens as f32) * 100.0
        }
    }

    fn get_color(&self) -> egui::Color32 {
        let percentage = self.usage_percentage();
        if percentage < 50.0 {
            egui::Color32::from_rgb(60, 150, 60)   // Green
        } else if percentage < 75.0 {
            egui::Color32::from_rgb(200, 180, 50)  // Yellow
        } else if percentage < 90.0 {
            egui::Color32::from_rgb(220, 120, 40)  // Orange
        } else {
            egui::Color32::from_rgb(200, 60, 60)   // Red
        }
    }

    fn update_counts(&mut self, system_tokens: u32, conversation_tokens: u32) {
        self.system_tokens = system_tokens;
        self.conversation_tokens = conversation_tokens;
        self.current_tokens = system_tokens + conversation_tokens;
    }

    fn should_compact(&self) -> bool {
        self.usage_percentage() >= (self.compaction_threshold * 100.0)
    }

    fn should_warn(&self) -> bool {
        self.usage_percentage() >= (self.warning_threshold * 100.0)
    }
}

enum MessageRole {
    User,
    Assistant,
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
        let llm_adapter = Arc::from(create_adapter(AdapterType::OpenRouter, api_key.clone()));

        // Create default assistant agent configuration
        let mut assistant_config = AgentConfig::default_assistant();
        assistant_config.personality = system_prompts.personality_instructions.clone();

        // Create agent from config
        let assistant_agent = Agent::new(
            assistant_config.clone(),
            Arc::clone(&llm_adapter),
            Arc::clone(&event_bus),
            Arc::clone(&runtime),
            system_prompts.system_instructions.clone(),
        );

        Self {
            message_input: String::new(),
            messages: Vec::new(),
            llm_adapter,
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            runtime,
            spinner_rotation: 0.0,
            token_stats: Self::check_and_reset_daily_stats(token_stats),
            context_tracker: ContextTracker::default(),
            sidebar_open: true, // Start with sidebar open
            current_view: AppView::Chat,
            settings_view: SettingsView::AiSettings,
            system_prompts,
            selected_model: "Claude Sonnet 4.5".to_string(),
            event_bus,
            event_rx,
            agents: vec![assistant_agent],
            active_agent_id: "assistant".to_string(),
            agent_configs: vec![assistant_config],
            selected_agent_index: None,
            event_history: Vec::new(),
            show_event_visualizer: true, // Start with visualizer open for debugging
        }
    }

    fn get_instructions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|e| format!("Could not determine home directory: {}", e))?;

        let mut dir = PathBuf::from(home_dir);
        dir.push(".rustbot");
        dir.push("instructions");

        // Create directory if it doesn't exist
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    fn load_system_prompts() -> Result<SystemPrompts, Box<dyn std::error::Error>> {
        let mut dir = Self::get_instructions_dir()?;

        // Load system instructions
        dir.push("system");
        dir.push("current");
        let system_instructions = if dir.exists() {
            std::fs::read_to_string(&dir)?
        } else {
            String::new()
        };
        dir.pop();
        dir.pop();

        // Load personality instructions
        dir.push("personality");
        dir.push("current");
        let personality_instructions = if dir.exists() {
            std::fs::read_to_string(&dir)?
        } else {
            String::new()
        };

        Ok(SystemPrompts {
            system_instructions,
            personality_instructions,
        })
    }

    fn save_system_prompts(&self) -> Result<(), Box<dyn std::error::Error>> {
        let base_dir = Self::get_instructions_dir()?;

        // Save system instructions
        let mut system_dir = base_dir.clone();
        system_dir.push("system");
        std::fs::create_dir_all(&system_dir)?;

        let system_current = system_dir.join("current");

        // Create backup if current exists
        if system_current.exists() {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = system_dir.join(format!("backup_{}", timestamp));
            std::fs::copy(&system_current, &backup_path)?;
        }

        // Write new current
        std::fs::write(&system_current, &self.system_prompts.system_instructions)?;

        // Save personality instructions
        let mut personality_dir = base_dir.clone();
        personality_dir.push("personality");
        std::fs::create_dir_all(&personality_dir)?;

        let personality_current = personality_dir.join("current");

        // Create backup if current exists
        if personality_current.exists() {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = personality_dir.join(format!("backup_{}", timestamp));
            std::fs::copy(&personality_current, &backup_path)?;
        }

        // Write new current
        std::fs::write(&personality_current, &self.system_prompts.personality_instructions)?;

        Ok(())
    }

    fn get_stats_file_path() -> PathBuf {
        let mut path = PathBuf::from(".");
        path.push("rustbot_stats.json");
        path
    }

    fn load_token_stats() -> Result<TokenStats, Box<dyn std::error::Error>> {
        let path = Self::get_stats_file_path();
        if !path.exists() {
            return Ok(TokenStats::default());
        }

        let content = std::fs::read_to_string(path)?;
        let stats: TokenStats = serde_json::from_str(&content)?;
        Ok(stats)
    }

    fn save_token_stats(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_stats_file_path();
        let content = serde_json::to_string_pretty(&self.token_stats)?;
        std::fs::write(path, content)?;
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
        self.messages.clear();
        self.current_response.clear();
        self.context_tracker.update_counts(0, 0);
    }

    fn send_message(&mut self, ctx: &egui::Context) {
        if self.message_input.trim().is_empty() || self.is_waiting {
            return;
        }

        // Publish UserMessage event instead of directly calling LLM
        let event = Event::new(
            "user".to_string(),
            "assistant".to_string(),  // Default agent for now
            EventKind::UserMessage(self.message_input.clone()),
        );

        if let Err(e) = self.event_bus.publish(event) {
            tracing::error!("Failed to publish user message event: {}", e);
            return;
        }

        // Clear input immediately after publishing event
        self.message_input.clear();
    }

    fn handle_user_message_event(&mut self, ctx: &egui::Context, content: String) {
        // Calculate input tokens
        let input_tokens = self.estimate_tokens(&content);
        self.token_stats.daily_input += input_tokens;
        self.token_stats.total_input += input_tokens;

        // Save stats after updating
        let _ = self.save_token_stats();

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.clone(),
            input_tokens: Some(input_tokens),
            output_tokens: None,
        });

        // Build conversation history for agent
        let mut context_messages = Vec::new();

        // Add dynamic system context
        context_messages.push(LlmMessage {
            role: "system".to_string(),
            content: self.generate_system_context(),
        });

        // Add conversation history (excluding the message we just added)
        for msg in &self.messages[..self.messages.len() - 1] {
            context_messages.push(LlmMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            });
        }

        // Update context tracker
        let system_content_tokens = self.estimate_tokens(&self.generate_system_context());
        let conversation_total_tokens: u32 = self.messages.iter()
            .map(|msg| self.estimate_tokens(&msg.content))
            .sum();
        self.context_tracker.update_counts(system_content_tokens, conversation_total_tokens);

        // Find the active agent and process message
        if let Some(agent) = self.agents.iter_mut().find(|a| a.id() == self.active_agent_id) {
            self.is_waiting = true;
            self.current_response.clear();

            // Process message through agent
            let content_clone = content.clone();
            let rt = Arc::clone(&self.runtime);

            match rt.block_on(agent.process_message(content_clone, context_messages)) {
                Ok(rx) => {
                    self.response_rx = Some(rx);

                    // Add placeholder for assistant response
                    self.messages.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: String::new(),
                        input_tokens: None,
                        output_tokens: None,
                    });

                    ctx.request_repaint();
                }
                Err(e) => {
                    tracing::error!("Failed to process message through agent: {}", e);
                    self.is_waiting = false;
                }
            }
        } else {
            tracing::error!("Active agent '{}' not found", self.active_agent_id);
            self.is_waiting = false;
        }
    }

    fn render_chat_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Calculate available height for messages
        let input_area_height = 60.0;
        let available_height = ui.available_height() - input_area_height;

        // Scrollable message area
        egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .stick_to_bottom(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if self.messages.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(egui::RichText::new("Welcome! Type a message below to start chatting.")
                                    .color(egui::Color32::from_rgb(100, 100, 100)));
                            });
                        } else {
                            for msg in &self.messages {
                                let (label, color) = match msg.role {
                                    MessageRole::User => ("You", egui::Color32::from_rgb(45, 100, 200)),
                                    MessageRole::Assistant => ("Assistant", egui::Color32::from_rgb(60, 150, 60)),
                                };

                                // Message header
                                ui.horizontal(|ui| {
                                    ui.colored_label(color, egui::RichText::new(format!("{}:", label)).strong());

                                    if msg.content.is_empty() && self.is_waiting {
                                        // Draw spinner
                                        let spinner_size = 12.0;
                                        let (response, painter) = ui.allocate_painter(
                                            egui::vec2(spinner_size, spinner_size),
                                            egui::Sense::hover(),
                                        );
                                        let center = response.rect.center();
                                        let radius = spinner_size / 2.0;

                                        painter.circle_stroke(
                                            center,
                                            radius,
                                            egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 150, 150)),
                                        );

                                        // Draw rotating arc
                                        let arc_length = std::f32::consts::PI * 1.5;
                                        let start_angle = self.spinner_rotation;
                                        for i in 0..20 {
                                            let t = i as f32 / 20.0;
                                            let angle = start_angle + arc_length * t;
                                            let pos = center + egui::vec2(angle.cos(), angle.sin()) * radius;
                                            let alpha = (t * 255.0) as u8;
                                            painter.circle_filled(pos, 1.5, egui::Color32::from_rgba_premultiplied(60, 120, 220, alpha));
                                        }

                                        ui.label(egui::RichText::new("Thinking...")
                                            .color(egui::Color32::from_rgb(150, 150, 150))
                                            .italics());
                                    }
                                });

                                // Display message content with proper wrapping
                                if !msg.content.is_empty() {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0); // Indent message content
                                        let available_width = ui.available_width() - 20.0;
                                        ui.vertical(|ui| {
                                            ui.set_max_width(available_width);
                                            ui.label(
                                                egui::RichText::new(&msg.content)
                                                    .color(egui::Color32::from_rgb(40, 40, 40))
                                            );
                                        });
                                    });
                                }
                                ui.add_space(8.0);
                            }
                        }
                    });

                ui.separator();

                // Status indicator when processing
                if self.is_waiting {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);

                        // Animated spinner
                        let spinner_rect = egui::Rect::from_center_size(
                            egui::pos2(ui.cursor().left() + 8.0, ui.cursor().top() + 8.0),
                            egui::vec2(12.0, 12.0),
                        );
                        ui.painter().circle_stroke(
                            spinner_rect.center(),
                            5.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                        );
                        ui.painter().circle_filled(
                            egui::pos2(
                                spinner_rect.center().x + 5.0 * self.spinner_rotation.cos(),
                                spinner_rect.center().y + 5.0 * self.spinner_rotation.sin(),
                            ),
                            2.0,
                            egui::Color32::from_rgb(100, 150, 255),
                        );

                        ui.add_space(20.0);

                        // Get agent status
                        let status_text = if let Some(agent) = self.agents.iter().find(|a| a.id() == self.active_agent_id) {
                            match agent.status() {
                                crate::events::AgentStatus::Thinking => "Assistant is thinking...",
                                crate::events::AgentStatus::Responding => "Assistant is responding...",
                                _ => "Processing your message...",
                            }
                        } else {
                            "Processing..."
                        };

                        ui.label(
                            egui::RichText::new(status_text)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(100, 150, 255))
                        );
                    });
                    ui.add_space(5.0);
                }

                // Add spacing before input area
                ui.add_space(15.0);

                // Input area with multi-line text box
                ui.horizontal(|ui| {
                    let text_edit_width = ui.available_width() - 70.0;
                    let response = ui.add_sized(
                        [text_edit_width, 80.0],
                        egui::TextEdit::multiline(&mut self.message_input)
                            .hint_text("Type your message here...\n\nPress Cmd+Enter to send")
                            .desired_width(text_edit_width),
                    );

                    let send_button = ui.add_sized(
                        [60.0, 80.0],
                        egui::Button::new(if self.is_waiting { "..." } else { "Send" })
                    );

                    // Send on Cmd+Enter or button click
                    let cmd_enter = ui.input(|i| {
                        i.key_pressed(egui::Key::Enter) && (i.modifiers.command || i.modifiers.ctrl)
                    });

                    if (send_button.clicked() || cmd_enter) && !self.is_waiting {
                        self.send_message(ctx);
                    }
                });

                // Compact token tracker under input box
                ui.horizontal(|ui| {
                    let daily_cost = self.calculate_cost(
                        self.token_stats.daily_input,
                        self.token_stats.daily_output,
                    );
                    let total_cost = self.calculate_cost(
                        self.token_stats.total_input,
                        self.token_stats.total_output,
                    );

                    ui.label(egui::RichText::new(format!(
                        "{} Daily: {}↑ {}↓ (${:.4})  •  Total: {}↑ {}↓ (${:.4})",
                        icons::CHART_LINE,
                        self.token_stats.daily_input,
                        self.token_stats.daily_output,
                        daily_cost,
                        self.token_stats.total_input,
                        self.token_stats.total_output,
                        total_cost
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)));

                    // Add space before clear button
                    ui.add_space(20.0);

                    // Clear chat button
                    if ui.button(egui::RichText::new("🗑 Clear Chat")
                        .size(11.0))
                        .on_hover_text("Clear conversation history")
                        .clicked() {
                        self.clear_conversation();
                    }
                });

                // Context window progress bar
                ui.horizontal(|ui| {
                    let percentage = self.context_tracker.usage_percentage();
                    let color = self.context_tracker.get_color();

                    // Draw progress bar
                    let available_width = ui.available_width() - 150.0;
                    let bar_height = 8.0;
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::vec2(available_width, bar_height),
                        egui::Sense::hover(),
                    );

                    // Background (gray)
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        egui::Color32::from_rgb(200, 200, 200),
                    );

                    // Filled portion (color-coded)
                    let filled_width = (available_width * percentage / 100.0).max(0.0).min(available_width);
                    if filled_width > 0.0 {
                        let filled_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(filled_width, bar_height),
                        );
                        ui.painter().rect_filled(filled_rect, 2.0, color);
                    }

                    // Label with percentage and token counts
                    ui.label(egui::RichText::new(format!(
                        "{:.1}% ({}/{}k)",
                        percentage,
                        self.context_tracker.current_tokens / 1000,
                        self.context_tracker.max_tokens / 1000
                    ))
                    .size(11.0)
                    .color(color));
                });
    }

    fn render_settings_view(&mut self, ui: &mut egui::Ui) {
        // Secondary navigation bar under header
        ui.horizontal(|ui| {
            let ai_settings_button = ui.add(
                egui::SelectableLabel::new(
                    self.settings_view == SettingsView::AiSettings,
                    "AI Settings"
                )
            );
            if ai_settings_button.clicked() {
                self.settings_view = SettingsView::AiSettings;
            }

            ui.add_space(10.0);

            let system_prompts_button = ui.add(
                egui::SelectableLabel::new(
                    self.settings_view == SettingsView::SystemPrompts,
                    "System Prompts"
                )
            );
            if system_prompts_button.clicked() {
                self.settings_view = SettingsView::SystemPrompts;
            }

            ui.add_space(10.0);

            let agents_button = ui.add(
                egui::SelectableLabel::new(
                    self.settings_view == SettingsView::Agents,
                    "Agents"
                )
            );
            if agents_button.clicked() {
                self.settings_view = SettingsView::Agents;
            }
        });
        ui.separator();

        // Render content based on selected settings view
        match self.settings_view {
            SettingsView::AiSettings => self.render_ai_settings(ui),
            SettingsView::SystemPrompts => self.render_system_prompts(ui),
            SettingsView::Agents => self.render_agents_view(ui),
        }
    }

    fn render_ai_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.heading("AI Model Selection");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Select LLM Model:");
            egui::ComboBox::from_id_salt("model_selector")
                .selected_text(&self.selected_model)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_model, "Claude Sonnet 4.5".to_string(), "Claude Sonnet 4.5");
                    ui.selectable_value(&mut self.selected_model, "Claude Sonnet 4".to_string(), "Claude Sonnet 4");
                    ui.selectable_value(&mut self.selected_model, "Claude Opus 4".to_string(), "Claude Opus 4");
                    ui.selectable_value(&mut self.selected_model, "GPT-4".to_string(), "GPT-4");
                });
        });

        ui.add_space(20.0);
        ui.label(egui::RichText::new(format!("Currently using: {}", self.selected_model))
            .color(egui::Color32::from_rgb(80, 80, 80)));
    }

    fn render_system_prompts(&mut self, ui: &mut egui::Ui) {
        // Use scroll area for system prompts
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("System Prompts");
                ui.add_space(10.0);

                ui.label("These instructions are sent with every chat session:");
                ui.add_space(10.0);

                // System Instructions
                ui.label(egui::RichText::new("System Instructions:").strong());
                ui.add_space(5.0);
                let system_instructions_response = ui.add_sized(
                    [ui.available_width() - 20.0, 200.0],
                    egui::TextEdit::multiline(&mut self.system_prompts.system_instructions)
                        .hint_text("Enter system instructions for the AI...")
                        .margin(egui::vec2(8.0, 8.0))
                );

                ui.add_space(15.0);

                // Personality Instructions
                ui.label(egui::RichText::new("Personality Instructions:").strong());
                ui.add_space(5.0);
                let personality_response = ui.add_sized(
                    [ui.available_width() - 20.0, 200.0],
                    egui::TextEdit::multiline(&mut self.system_prompts.personality_instructions)
                        .hint_text("Enter personality instructions for the AI...")
                        .margin(egui::vec2(8.0, 8.0))
                );

                ui.add_space(15.0);

                // Save button
                if ui.button("Save Prompts").clicked() {
                    if let Err(e) = self.save_system_prompts() {
                        tracing::error!("Failed to save system prompts: {}", e);
                    }
                }

                // Show if any changes were detected
                if system_instructions_response.changed() || personality_response.changed() {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("* Unsaved changes")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(220, 100, 60)));
                }

                ui.add_space(20.0); // Bottom padding
            });
    }

    fn render_agents_view(&mut self, ui: &mut egui::Ui) {
        // Use scroll area for agents
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("Agents");
                ui.add_space(10.0);

                ui.label("Configure AI agents with their own instructions and personality:");
                ui.add_space(15.0);

                // Agent list
                ui.label(egui::RichText::new("Available Agents:").strong());
                ui.add_space(10.0);

                // Display each agent
                for (index, config) in self.agent_configs.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Agent name and status
                            let is_active = config.id == self.active_agent_id;
                            let status_color = if is_active {
                                egui::Color32::from_rgb(60, 150, 60)
                            } else {
                                egui::Color32::from_rgb(100, 100, 100)
                            };

                            ui.label(
                                egui::RichText::new(format!("🤖 {}", config.name))
                                    .strong()
                                    .size(16.0)
                            );

                            if is_active {
                                ui.label(
                                    egui::RichText::new("(Active)")
                                        .size(12.0)
                                        .color(status_color)
                                );
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Edit button
                                if ui.button("Edit").clicked() {
                                    self.selected_agent_index = Some(index);
                                }

                                // Set Active button
                                if !is_active && ui.button("Set Active").clicked() {
                                    self.active_agent_id = config.id.clone();
                                }
                            });
                        });

                        ui.add_space(5.0);
                        ui.label(format!("ID: {}", config.id));
                        ui.label(format!("Model: {}", config.model));
                    });

                    ui.add_space(10.0);
                }

                ui.add_space(15.0);

                // Agent editing section
                if let Some(index) = self.selected_agent_index {
                    if let Some(config) = self.agent_configs.get_mut(index) {
                        ui.separator();
                        ui.add_space(15.0);

                        ui.heading(format!("Edit Agent: {}", config.name));
                        ui.add_space(10.0);

                        // Agent name
                        ui.label(egui::RichText::new("Agent Name:").strong());
                        ui.add_space(5.0);
                        ui.text_edit_singleline(&mut config.name);
                        ui.add_space(10.0);

                        // Agent instructions
                        ui.label(egui::RichText::new("Agent Instructions:").strong());
                        ui.label("What this agent does and how it should behave:");
                        ui.add_space(5.0);
                        ui.add_sized(
                            [ui.available_width() - 20.0, 150.0],
                            egui::TextEdit::multiline(&mut config.instructions)
                                .hint_text("Enter agent-specific instructions...")
                                .margin(egui::vec2(8.0, 8.0))
                        );

                        ui.add_space(15.0);

                        // Agent personality
                        ui.label(egui::RichText::new("Agent Personality:").strong());
                        ui.label("The agent's communication style and personality:");
                        ui.add_space(5.0);
                        ui.add_sized(
                            [ui.available_width() - 20.0, 150.0],
                            egui::TextEdit::multiline(&mut config.personality)
                                .hint_text("Enter agent personality traits...")
                                .margin(egui::vec2(8.0, 8.0))
                        );

                        ui.add_space(15.0);

                        // Model selection
                        ui.label(egui::RichText::new("LLM Model:").strong());
                        ui.add_space(5.0);
                        egui::ComboBox::from_label("")
                            .selected_text(&config.model)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut config.model, "anthropic/claude-sonnet-4.5".to_string(), "Claude Sonnet 4.5");
                                ui.selectable_value(&mut config.model, "anthropic/claude-sonnet-4".to_string(), "Claude Sonnet 4");
                                ui.selectable_value(&mut config.model, "anthropic/claude-opus-4".to_string(), "Claude Opus 4");
                                ui.selectable_value(&mut config.model, "openai/gpt-4".to_string(), "GPT-4");
                            });

                        ui.add_space(15.0);

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.button("Save Changes").clicked() {
                                // Apply changes to agent (will implement recreation later)
                                self.selected_agent_index = None;
                            }

                            if ui.button("Cancel").clicked() {
                                self.selected_agent_index = None;
                            }
                        });
                    }
                }

                ui.add_space(20.0); // Bottom padding
            });
    }
}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process events from the event bus
        while let Ok(event) = self.event_rx.try_recv() {
            // Track event for visualization (keep last 50 events)
            let event_kind_str = match &event.kind {
                EventKind::UserMessage(_) => "UserMessage".to_string(),
                EventKind::AgentMessage { .. } => "AgentMessage".to_string(),
                EventKind::AgentStatusChange { .. } => "StatusChange".to_string(),
                EventKind::SystemCommand(_) => "SystemCommand".to_string(),
                EventKind::Test(_) => "Test".to_string(),
            };

            self.event_history.push(VisualEvent {
                source: event.source.clone(),
                destination: event.destination.clone(),
                kind: event_kind_str,
                timestamp: event.timestamp,
            });

            // Keep only last 50 events
            if self.event_history.len() > 50 {
                self.event_history.remove(0);
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
                    }
                    EventKind::SystemCommand(cmd) => {
                        match cmd {
                            SystemCommand::ClearConversation => {
                                self.clear_conversation();
                            }
                            SystemCommand::SaveState => {
                                tracing::info!("Save state command received");
                            }
                            SystemCommand::LoadState => {
                                tracing::info!("Load state command received");
                            }
                        }
                    }
                    EventKind::Test(msg) => {
                        tracing::info!("Test event received: {}", msg);
                    }
                }
            }
        }

        // Update spinner rotation when waiting
        if self.is_waiting {
            self.spinner_rotation += 0.1;
            ctx.request_repaint();
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
                }
            });
        });
    }
}
