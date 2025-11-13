mod llm;

use eframe::egui;
use llm::{LlmClient, Message as LlmMessage};
use std::sync::Arc;
use tokio::sync::mpsc;

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
    llm_client: Arc<LlmClient>,
    response_rx: Option<mpsc::UnboundedReceiver<String>>,
    current_response: String,
    is_waiting: bool,
    runtime: tokio::runtime::Runtime,
}

struct ChatMessage {
    role: MessageRole,
    content: String,
}

enum MessageRole {
    User,
    Assistant,
}

impl RustbotApp {
    fn new(api_key: String) -> Self {
        Self {
            message_input: String::new(),
            messages: Vec::new(),
            llm_client: Arc::new(LlmClient::new(api_key)),
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            runtime: tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }

    fn send_message(&mut self, ctx: &egui::Context) {
        if self.message_input.trim().is_empty() || self.is_waiting {
            return;
        }

        // Add user message
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: self.message_input.clone(),
        });

        // Prepare conversation history for API
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

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();
        self.response_rx = Some(rx);
        self.current_response.clear();
        self.is_waiting = true;

        // Add placeholder for assistant response
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
        });

        // Spawn async task to call LLM
        let client = Arc::clone(&self.llm_client);
        let ctx_clone = ctx.clone();
        self.runtime.spawn(async move {
            if let Err(e) = client.stream_chat(api_messages, tx).await {
                tracing::error!("LLM stream error: {}", e);
            }
            ctx_clone.request_repaint(); // Final repaint when done
        });

        self.message_input.clear();
    }
}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
        ].into();
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header at top
                ui.heading("Rustbot - AI Assistant");
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

                                ui.horizontal(|ui| {
                                    ui.colored_label(color, egui::RichText::new(format!("{}:", label)).strong());

                                    if msg.content.is_empty() && self.is_waiting {
                                        ui.label(egui::RichText::new("Thinking...")
                                            .color(egui::Color32::from_rgb(150, 150, 150))
                                            .italics());
                                    } else {
                                        ui.label(egui::RichText::new(&msg.content).color(egui::Color32::from_rgb(30, 30, 30)));
                                    }
                                });
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
            });
        });
    }
}
