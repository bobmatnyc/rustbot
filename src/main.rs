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
            // Use vertical layout with explicit spacing
            ui.vertical(|ui| {
                // Header at top
                ui.heading("Rustbot - AI Assistant");
                ui.separator();

                // Calculate available height for messages (leave room for input area)
                let input_area_height = 60.0; // Height for input box + spacing + separator
                let available_height = ui.available_height() - input_area_height;

                // Scrollable message area that fills remaining space
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
                                    ui.label(egui::RichText::new(&msg.content).color(egui::Color32::from_rgb(30, 30, 30)));
                                });
                                ui.add_space(8.0);
                            }
                        }
                    });

                // Push input to the very bottom
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

                    let send_button = ui.add_sized([60.0, 30.0], egui::Button::new("Send"));

                    if send_button.clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
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
        });
    }
}
