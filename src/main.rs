mod llm;
mod version;

use eframe::egui;
use llm::{create_adapter, AdapterType, LlmAdapter, LlmRequest, Message as LlmMessage};
use std::sync::Arc;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Load .env.local file
    dotenvy::from_filename(".env.local").ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Rustbot - AI Assistant"),
        ..Default::default()
    };

    eframe::run_native(
        "rustbot",
        options,
        Box::new(|_cc| {
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
    runtime: tokio::runtime::Runtime,
    spinner_rotation: f32,
    token_stats: TokenStats,
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

enum MessageRole {
    User,
    Assistant,
}

impl RustbotApp {
    fn new(api_key: String) -> Self {
        let token_stats = Self::load_token_stats().unwrap_or_default();

        Self {
            message_input: String::new(),
            messages: Vec::new(),
            llm_adapter: Arc::from(create_adapter(AdapterType::OpenRouter, api_key)),
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            runtime: tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
            spinner_rotation: 0.0,
            token_stats: Self::check_and_reset_daily_stats(token_stats),
        }
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
        // Claude Sonnet 4 pricing via OpenRouter
        // Input: $3.00 per million tokens
        // Output: $15.00 per million tokens
        const INPUT_COST_PER_MILLION: f64 = 3.0;
        const OUTPUT_COST_PER_MILLION: f64 = 15.0;

        let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;

        input_cost + output_cost
    }

    fn send_message(&mut self, ctx: &egui::Context) {
        if self.message_input.trim().is_empty() || self.is_waiting {
            return;
        }

        // Calculate input tokens
        let input_tokens = self.estimate_tokens(&self.message_input);
        self.token_stats.daily_input += input_tokens;
        self.token_stats.total_input += input_tokens;

        // Save stats after updating
        let _ = self.save_token_stats();

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: self.message_input.clone(),
            input_tokens: Some(input_tokens),
            output_tokens: None,
        });

        // Prepare conversation history for API using unified format
        let mut api_messages = Vec::new();
        for msg in &self.messages {
            api_messages.push(LlmMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            });
        }

        // Create unified LLM request
        let request = LlmRequest::new(api_messages);

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();
        self.response_rx = Some(rx);
        self.current_response.clear();
        self.is_waiting = true;

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            input_tokens: None,
            output_tokens: None,
        });

        // Spawn async task to call LLM using the unified interface
        let adapter = Arc::clone(&self.llm_adapter);
        let ctx_clone = ctx.clone();
        self.runtime.spawn(async move {
            if let Err(e) = adapter.stream_chat(request, tx).await {
                tracing::error!("LLM stream error: {}", e);
            }
            ctx_clone.request_repaint(); // Final repaint when done
        });

        self.message_input.clear();
    }
}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // Set larger default font sizes
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
        ].into();
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header at top with version info
                ui.horizontal(|ui| {
                    ui.heading("Rustbot - AI Assistant");
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(version::version_string())
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));
                });
                ui.separator();

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
                                    MessageRole::User => ("You", egui::Color32::from_rgb(60, 120, 220)),
                                    MessageRole::Assistant => ("Assistant", egui::Color32::from_rgb(80, 180, 80)),
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
                                                    .color(egui::Color32::from_rgb(30, 30, 30))
                                            );
                                        });
                                    });
                                }
                                ui.add_space(8.0);
                            }
                        }
                    });

                ui.separator();

                // Input area pinned at bottom
                ui.horizontal(|ui| {
                    let text_edit_width = ui.available_width() - 70.0;
                    let response = ui.add_sized(
                        [text_edit_width, 30.0],
                        egui::TextEdit::singleline(&mut self.message_input)
                            .hint_text("Type your message here...")
                            .desired_width(text_edit_width),
                    );

                    let send_button = ui.add_sized(
                        [60.0, 30.0],
                        egui::Button::new(if self.is_waiting { "..." } else { "Send" })
                    );

                    if (send_button.clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                        && !self.is_waiting {
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
                        "📊 Daily: {}↑ {}↓ (${:.4})  •  Total: {}↑ {}↓ (${:.4})",
                        self.token_stats.daily_input,
                        self.token_stats.daily_output,
                        daily_cost,
                        self.token_stats.total_input,
                        self.token_stats.total_output,
                        total_cost
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)));
                });
            });
        });
    }
}
