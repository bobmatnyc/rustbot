use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Rustbot - AI Assistant"),
        ..Default::default()
    };

    eframe::run_native(
        "rustbot",
        options,
        Box::new(|_cc| Ok(Box::new(RustbotApp::default()))),
    )
}

#[derive(Default)]
struct RustbotApp {
    message_input: String,
    messages: Vec<ChatMessage>,
}

struct ChatMessage {
    role: MessageRole,
    content: String,
}

enum MessageRole {
    User,
    Assistant,
}

impl eframe::App for RustbotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rustbot - AI Assistant");
            ui.separator();

            // Chat messages area (scrollable)
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &self.messages {
                        let (label, color) = match msg.role {
                            MessageRole::User => ("You", egui::Color32::from_rgb(100, 150, 255)),
                            MessageRole::Assistant => ("Assistant", egui::Color32::from_rgb(150, 255, 150)),
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(color, format!("{}:", label));
                            ui.label(&msg.content);
                        });
                        ui.add_space(8.0);
                    }
                });

            ui.separator();

            // Input area at bottom
            ui.horizontal(|ui| {
                let response = ui.add_sized(
                    [ui.available_width() - 60.0, 30.0],
                    egui::TextEdit::singleline(&mut self.message_input)
                        .hint_text("Type your message here..."),
                );

                if ui.button("Send").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    if !self.message_input.trim().is_empty() {
                        // Add user message
                        self.messages.push(ChatMessage {
                            role: MessageRole::User,
                            content: self.message_input.clone(),
                        });

                        // Add mock assistant response (will be replaced with real API)
                        self.messages.push(ChatMessage {
                            role: MessageRole::Assistant,
                            content: format!("Echo: {}", self.message_input),
                        });

                        self.message_input.clear();
                        response.request_focus();
                    }
                }
            });
        });
    }
}
