// UI view rendering methods for Rustbot
// Contains all the main view rendering functions extracted from RustbotApp

use crate::ui::{MessageRole, SettingsView};
use eframe::egui;
use egui_phosphor::regular as icons;

/// Extension trait to add view rendering methods to RustbotApp
/// This allows us to define methods on RustbotApp from a separate module
impl crate::RustbotApp {
    /// Render the main chat view with message history and input controls
    ///
    /// This method handles:
    /// - Scrollable message display area
    /// - Animated spinner for waiting states
    /// - Message input field with multi-line support
    /// - Token usage statistics display
    /// - Context window progress bar
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    /// * `ctx` - The egui Context for global state and repaints
    pub fn render_chat_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Calculate available height for messages
        // Account for all UI elements below the message area:
        // - Status indicator (if waiting): ~35px
        // - Spacing before input: 15px
        // - Input area: 80px
        // - Token tracker: ~25px
        // - Context bar: ~25px
        // Total bottom UI: ~180px
        let status_height = if self.is_waiting { 35.0 } else { 0.0 };
        let bottom_ui_height = status_height + 15.0 + 80.0 + 25.0 + 25.0;
        let available_height = ui.available_height() - bottom_ui_height - 20.0; // Extra margin

        // Scrollable message area
        egui::ScrollArea::vertical()
            .max_height(available_height.max(100.0)) // Minimum 100px for messages
            .stick_to_bottom(true)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if self.messages.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("Welcome! Type a message below to start chatting.")
                                .color(egui::Color32::from_rgb(100, 100, 100)),
                        );
                    });
                } else {
                    for msg in &self.messages {
                        let (label, color) = match msg.role {
                            MessageRole::User => ("You", egui::Color32::from_rgb(45, 100, 200)),
                            MessageRole::Assistant => {
                                ("Assistant", egui::Color32::from_rgb(60, 150, 60))
                            }
                        };

                        // Message header
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                color,
                                egui::RichText::new(format!("{}:", label)).strong(),
                            );

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
                                    painter.circle_filled(
                                        pos,
                                        1.5,
                                        egui::Color32::from_rgba_premultiplied(
                                            60, 120, 220, alpha,
                                        ),
                                    );
                                }

                                ui.label(
                                    egui::RichText::new("Thinking...")
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .italics(),
                                );
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
                                            .color(egui::Color32::from_rgb(40, 40, 40)),
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

                // Get agent status using API
                let status_text = if let Some(status) = self.api.current_agent_status() {
                    match status {
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
                        .color(egui::Color32::from_rgb(100, 150, 255)),
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

            let send_button =
                ui.add_sized([60.0, 80.0], egui::Button::new(if self.is_waiting {
                    "..."
                } else {
                    "Send"
                }));

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

            ui.label(
                egui::RichText::new(format!(
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
                .color(egui::Color32::from_rgb(120, 120, 120)),
            );

            // Add space before clear button
            ui.add_space(20.0);

            // Clear chat button
            if ui
                .button(egui::RichText::new("🗑 Clear Chat").size(11.0))
                .on_hover_text("Clear conversation history")
                .clicked()
            {
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
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(available_width, bar_height), egui::Sense::hover());

            // Background (gray)
            ui.painter()
                .rect_filled(rect, 2.0, egui::Color32::from_rgb(200, 200, 200));

            // Filled portion (color-coded)
            let filled_width = (available_width * percentage / 100.0)
                .max(0.0)
                .min(available_width);
            if filled_width > 0.0 {
                let filled_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, bar_height));
                ui.painter().rect_filled(filled_rect, 2.0, color);
            }

            // Label with percentage and token counts
            ui.label(
                egui::RichText::new(format!(
                    "{:.1}% ({}/{}k)",
                    percentage,
                    self.context_tracker.current_tokens / 1000,
                    self.context_tracker.max_tokens / 1000
                ))
                .size(11.0)
                .color(color),
            );
        });
    }

    /// Render the settings view with navigation tabs
    ///
    /// Provides a tabbed interface for:
    /// - AI Settings (model selection)
    /// - System Prompts (instructions and personality)
    /// - Agents (agent configuration)
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_settings_view(&mut self, ui: &mut egui::Ui) {
        // Secondary navigation bar under header
        ui.horizontal(|ui| {
            let ai_settings_button = ui.add(egui::SelectableLabel::new(
                self.settings_view == SettingsView::AiSettings,
                "AI Settings",
            ));
            if ai_settings_button.clicked() {
                self.settings_view = SettingsView::AiSettings;
            }

            ui.add_space(10.0);

            let system_prompts_button = ui.add(egui::SelectableLabel::new(
                self.settings_view == SettingsView::SystemPrompts,
                "System Prompts",
            ));
            if system_prompts_button.clicked() {
                self.settings_view = SettingsView::SystemPrompts;
            }

            ui.add_space(10.0);

            let agents_button = ui.add(egui::SelectableLabel::new(
                self.settings_view == SettingsView::Agents,
                "Agents",
            ));
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

    /// Render the AI settings view for model selection
    ///
    /// Allows users to select the LLM model to use for conversations.
    /// Currently supports Claude Sonnet 4.5, Claude Sonnet 4, Claude Opus 4, and GPT-4.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_ai_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.heading("AI Model Selection");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Select LLM Model:");
            egui::ComboBox::from_id_salt("model_selector")
                .selected_text(&self.selected_model)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_model,
                        "Claude Sonnet 4.5".to_string(),
                        "Claude Sonnet 4.5",
                    );
                    ui.selectable_value(
                        &mut self.selected_model,
                        "Claude Sonnet 4".to_string(),
                        "Claude Sonnet 4",
                    );
                    ui.selectable_value(
                        &mut self.selected_model,
                        "Claude Opus 4".to_string(),
                        "Claude Opus 4",
                    );
                    ui.selectable_value(
                        &mut self.selected_model,
                        "GPT-4".to_string(),
                        "GPT-4",
                    );
                });
        });

        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(format!("Currently using: {}", self.selected_model))
                .color(egui::Color32::from_rgb(80, 80, 80)),
        );
    }

    /// Render the system prompts configuration view
    ///
    /// Allows editing of:
    /// - System Instructions: Core behavioral instructions for the AI (shared across all agents)
    ///
    /// Note: Agent personality is now configured per-agent in the Agents settings.
    ///
    /// Changes can be saved and are backed up automatically.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_system_prompts(&mut self, ui: &mut egui::Ui) {
        // Use scroll area for system prompts
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("System Instructions");
                ui.add_space(10.0);

                ui.label("These instructions are sent with every chat session (shared across all agents):");
                ui.add_space(5.0);
                ui.label(egui::RichText::new("Note: Agent personality is configured per-agent in the Agents tab.")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(100, 100, 100)));
                ui.add_space(10.0);

                // System Instructions
                ui.label(egui::RichText::new("System Instructions:").strong());
                ui.add_space(5.0);
                let system_instructions_response = ui.add_sized(
                    [ui.available_width() - 20.0, 300.0],
                    egui::TextEdit::multiline(&mut self.system_prompts.system_instructions)
                        .hint_text("Enter system instructions for the AI...")
                        .margin(egui::vec2(8.0, 8.0)),
                );

                ui.add_space(15.0);

                // Save button
                if ui.button("Save Instructions").clicked() {
                    if let Err(e) = self.save_system_prompts() {
                        tracing::error!("Failed to save system prompts: {}", e);
                    }
                }

                // Show if any changes were detected
                if system_instructions_response.changed() {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("* Unsaved changes")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(220, 100, 60)),
                    );
                }

                ui.add_space(20.0); // Bottom padding
            });
    }

    /// Render the agents management view
    ///
    /// Displays all configured agents and allows:
    /// - Viewing agent details (name, ID, model, web search capability)
    /// - Setting the active agent
    /// - Editing agent configuration (instructions, personality, model)
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_agents_view(&mut self, ui: &mut egui::Ui) {
        // Use scroll area for agents
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("Agents");
                ui.add_space(10.0);

                ui.label("Manage AI agents with specialized capabilities and instructions:");
                ui.add_space(15.0);

                // Agent list - show in list view
                ui.label(egui::RichText::new("Available Agents:").strong());
                ui.add_space(10.0);

                // Display each agent in a compact list format
                for (index, config) in self.agent_configs.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Agent icon and name
                            let status_color = if config.is_primary {
                                egui::Color32::from_rgb(60, 150, 60) // Green for primary
                            } else if config.enabled {
                                egui::Color32::from_rgb(100, 150, 200) // Blue for enabled
                            } else {
                                egui::Color32::from_rgb(100, 100, 100) // Gray for disabled
                            };

                            // Icon based on agent type
                            let icon = if config.is_primary {
                                format!("{} {}", icons::STAR, config.name)
                            } else if config.web_search_enabled {
                                format!("{} {}", icons::MAGNIFYING_GLASS, config.name)
                            } else {
                                format!("{} {}", icons::ROBOT, config.name)
                            };

                            ui.label(
                                egui::RichText::new(&icon)
                                    .strong()
                                    .size(16.0),
                            );

                            // Status indicator
                            if config.is_primary {
                                ui.label(
                                    egui::RichText::new("● Primary")
                                        .size(12.0)
                                        .color(status_color),
                                );
                            } else if config.enabled {
                                ui.label(
                                    egui::RichText::new("✓ Enabled")
                                        .size(12.0)
                                        .color(status_color),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("○ Disabled")
                                        .size(12.0)
                                        .color(status_color),
                                );
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Edit button (for all agents)
                                    if ui.button(format!("{} Edit", icons::PENCIL_SIMPLE)).clicked() {
                                        self.selected_agent_index = Some(index);
                                    }

                                    // Enable/Disable toggle (only for non-primary agents)
                                    if !config.is_primary {
                                        let toggle_text = if config.enabled {
                                            format!("{} Disable", icons::TOGGLE_RIGHT)
                                        } else {
                                            format!("{} Enable", icons::TOGGLE_LEFT)
                                        };

                                        if ui.button(toggle_text).clicked() {
                                            config.enabled = !config.enabled;
                                            // TODO: Persist this change and update the agent in the API
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(3.0);

                        // Compact info line
                        ui.horizontal(|ui| {
                            ui.add_space(20.0); // Indent
                            let role = if config.is_primary { "Primary" } else { "Specialist" };
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} • Model: {} • Web Search: {}",
                                    role,
                                    config.model.split('/').last().unwrap_or(&config.model),
                                    if config.web_search_enabled { "✓" } else { "✗" }
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 100, 100)),
                            );
                        });
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
                                .margin(egui::vec2(8.0, 8.0)),
                        );

                        ui.add_space(15.0);

                        // Agent personality
                        ui.label(egui::RichText::new("Agent Personality (Optional):").strong());
                        ui.label("The agent's communication style and personality:");
                        ui.add_space(5.0);

                        // Convert Option<String> to String for editing
                        let mut personality_text = config.personality.clone().unwrap_or_default();
                        let personality_response = ui.add_sized(
                            [ui.available_width() - 20.0, 150.0],
                            egui::TextEdit::multiline(&mut personality_text)
                                .hint_text("Enter agent personality traits (leave empty for none)...")
                                .margin(egui::vec2(8.0, 8.0)),
                        );

                        // Update config if changed
                        if personality_response.changed() {
                            config.personality = if personality_text.trim().is_empty() {
                                None
                            } else {
                                Some(personality_text)
                            };
                        }

                        ui.add_space(15.0);

                        // Model selection
                        ui.label(egui::RichText::new("LLM Model:").strong());
                        ui.add_space(5.0);
                        egui::ComboBox::from_label("")
                            .selected_text(&config.model)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut config.model,
                                    "anthropic/claude-sonnet-4.5".to_string(),
                                    "Claude Sonnet 4.5",
                                );
                                ui.selectable_value(
                                    &mut config.model,
                                    "anthropic/claude-sonnet-4".to_string(),
                                    "Claude Sonnet 4",
                                );
                                ui.selectable_value(
                                    &mut config.model,
                                    "anthropic/claude-opus-4".to_string(),
                                    "Claude Opus 4",
                                );
                                ui.selectable_value(
                                    &mut config.model,
                                    "openai/gpt-4".to_string(),
                                    "GPT-4",
                                );
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
