// UI view rendering methods for Rustbot
// Contains all the main view rendering functions extracted from RustbotApp

use crate::ui::{ExtensionsView, MessageRole, SettingsView};
use eframe::egui;
use egui_commonmark::CommonMarkViewer;
use egui_phosphor::regular as icons;
use std::sync::Arc;

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

                            // Copy button for assistant messages (only if message has content)
                            if msg.role == MessageRole::Assistant && !msg.content.is_empty() {
                                if ui.button(icons::CLIPBOARD_TEXT)
                                    .on_hover_text("Copy message to clipboard")
                                    .clicked()
                                {
                                    ui.ctx().copy_text(msg.content.clone());
                                }
                            }

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

                                // Show current activity or default "Thinking..." message
                                let status_text = self.current_activity
                                    .as_ref()
                                    .map(|s| s.as_str())
                                    .unwrap_or("Thinking...");

                                ui.label(
                                    egui::RichText::new(status_text)
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .italics(),
                                );
                            }
                        });

                        // Display message content with proper wrapping and markdown rendering
                        if !msg.content.is_empty() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_space(20.0); // Indent message content
                                let available_width = ui.available_width() - 20.0;
                                ui.vertical(|ui| {
                                    ui.set_max_width(available_width);
                                    // Render markdown content (mermaid preprocessing happens when content is set)
                                    CommonMarkViewer::new()
                                        .show(ui, &mut self.markdown_cache, &msg.content);

                                    // Add copy buttons for embedded images (Mermaid diagrams)
                                    if !msg.embedded_images.is_empty() {
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.add_space(2.0);

                                            for (i, data_url) in msg.embedded_images.iter().enumerate() {
                                                let label = if msg.embedded_images.len() == 1 {
                                                    format!("{} Copy Diagram", icons::CLIPBOARD)
                                                } else {
                                                    format!("{} Copy Diagram {}", icons::CLIPBOARD, i + 1)
                                                };

                                                if ui.button(
                                                    egui::RichText::new(label)
                                                        .size(10.5)
                                                        .color(egui::Color32::from_rgb(80, 120, 180))
                                                )
                                                .on_hover_text("Copy diagram image to clipboard (as data URL)")
                                                .clicked() {
                                                    ui.ctx().copy_text(data_url.clone());
                                                    tracing::info!("📋 Copied diagram {} to clipboard", i + 1);
                                                }

                                                if i < msg.embedded_images.len() - 1 {
                                                    ui.add_space(8.0);
                                                }
                                            }
                                        });
                                    }
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

                // Simple status text (agent status requires async access, so just show generic message)
                ui.label(
                    egui::RichText::new("Processing your message...")
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
            let _response = ui.add_sized(
                [text_edit_width, 80.0],
                egui::TextEdit::multiline(&mut self.message_input)
                    .hint_text("Type your message here...\n\nPress Cmd+Enter to send")
                    .desired_width(text_edit_width),
            );

            let send_button = ui.add_sized(
                [60.0, 80.0],
                egui::Button::new(if self.is_waiting { "..." } else { "Send" }),
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
            let daily_cost =
                self.calculate_cost(self.token_stats.daily_input, self.token_stats.daily_output);
            let total_cost =
                self.calculate_cost(self.token_stats.total_input, self.token_stats.total_output);

            // Get current model from primary agent
            let model = self
                .agent_configs
                .iter()
                .find(|config| config.is_primary)
                .map(|config| {
                    // Extract just the model name (after the last slash)
                    config.model.split('/').last().unwrap_or(&config.model)
                })
                .unwrap_or("unknown");

            ui.label(
                egui::RichText::new(format!(
                    "{} {} • Daily: {}↑ {}↓ (${:.4})  •  Total: {}↑ {}↓ (${:.4})",
                    icons::CHART_LINE,
                    model,
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

            // Add space before buttons
            ui.add_space(20.0);

            // Copy full chat button
            if ui
                .button(
                    egui::RichText::new(format!("{} Copy Chat", icons::CLIPBOARD_TEXT)).size(11.0),
                )
                .on_hover_text("Copy full conversation to clipboard")
                .clicked()
            {
                // Build full conversation text
                let mut full_chat = String::new();
                for msg in &self.messages {
                    let role = match msg.role {
                        MessageRole::User => "You",
                        MessageRole::Assistant => "Assistant",
                    };
                    full_chat.push_str(&format!("{}:\n{}\n\n", role, msg.content));
                }

                // Copy to clipboard
                ui.ctx().copy_text(full_chat);
            }

            ui.add_space(10.0);

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
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(available_width, bar_height),
                egui::Sense::hover(),
            );

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

            ui.add_space(10.0);

            let preferences_button = ui.add(egui::SelectableLabel::new(
                self.settings_view == SettingsView::Preferences,
                "Preferences",
            ));
            if preferences_button.clicked() {
                self.settings_view = SettingsView::Preferences;
            }
        });
        ui.separator();

        // Render content based on selected settings view
        match self.settings_view {
            SettingsView::SystemPrompts => self.render_system_prompts(ui),
            SettingsView::Agents => self.render_agents_view(ui),
            SettingsView::Preferences => self.render_preferences_view(ui),
        }
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

    /// Render the events view showing recent MCP plugin events
    ///
    /// Displays a dedicated view for monitoring MCP plugin events including:
    /// - Plugin starts/stops
    /// - Errors and health status changes
    /// - Tool changes and configuration reloads
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_events_view(&self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.heading(format!("{} Recent Events", icons::LIST_BULLETS));
        ui.add_space(10.0);

        ui.label("Monitor MCP extension activity and events:");
        ui.add_space(15.0);

        if let Some(plugins_view) = &self.plugins_view {
            plugins_view.render_events_only(ui);
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("No event data available")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
            });
        }
    }

    /// Render the marketplace view
    ///
    /// Displays the MCP Marketplace browser for discovering and installing MCP servers.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    /// * `ctx` - The egui Context for global state and repaints
    pub fn render_marketplace_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(marketplace_view) = &mut self.extensions_marketplace_view {
            marketplace_view.render(ui, ctx);
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("Marketplace view not initialized")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
            });
        }
    }

    /// Render the extensions view with tabs for Marketplace and Installed
    ///
    /// This view provides a unified interface for managing MCP extensions:
    /// - Marketplace: Browse and discover available MCP servers
    /// - Installed: View and manage installed extensions (with filtering)
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    /// * `ctx` - The egui Context for global state and repaints
    pub fn render_extensions_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Secondary navigation bar (tabs) - similar to Settings view pattern
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.extensions_view == ExtensionsView::Marketplace,
                    format!("{} Marketplace", icons::STOREFRONT),
                )
                .clicked()
            {
                self.extensions_view = ExtensionsView::Marketplace;
            }

            ui.add_space(10.0);

            if ui
                .selectable_label(
                    self.extensions_view == ExtensionsView::Installed,
                    format!("{} Installed", icons::PACKAGE),
                )
                .clicked()
            {
                self.extensions_view = ExtensionsView::Installed;
            }
        });
        ui.separator();

        // Render active subview
        match self.extensions_view {
            ExtensionsView::Marketplace => {
                // Reuse existing marketplace view
                self.render_marketplace_view(ui, ctx);
            }
            ExtensionsView::Installed => {
                self.render_installed_extensions(ui);
            }
        }
    }

    /// Render installed extensions view with filtering
    ///
    /// Shows all installed extensions with the ability to filter by type:
    /// - All: Show all installed extensions
    /// - Remote: Show only remote/cloud extensions
    /// - Local: Show only local extensions
    ///
    /// Each extension displays a badge indicating its type (Remote/Local).
    fn render_installed_extensions(&mut self, ui: &mut egui::Ui) {
        // Check if we're showing a configuration dialog
        if let Some(ext_id) = &self.configuring_extension_id.clone() {
            self.render_extension_config_dialog(ui, ext_id);
            return; // Show only the dialog when configuring
        }

        // Show uninstall confirmation dialog if present
        if let Some((ext_id, ext_name)) = &self.uninstall_confirmation.clone() {
            self.render_uninstall_confirmation_dialog(ui, ext_id, ext_name);
            return; // Show only the dialog when confirming
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("Installed Extensions");
                ui.add_space(10.0);

                // Filter dropdown
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    egui::ComboBox::from_id_source("install_type_filter")
                        .selected_text(self.installed_extensions_filter.label())
                        .show_ui(ui, |ui| {
                            use crate::ui::InstallTypeFilter;
                            ui.selectable_value(
                                &mut self.installed_extensions_filter,
                                InstallTypeFilter::All,
                                "All",
                            );
                            ui.selectable_value(
                                &mut self.installed_extensions_filter,
                                InstallTypeFilter::Remote,
                                "Remote",
                            );
                            ui.selectable_value(
                                &mut self.installed_extensions_filter,
                                InstallTypeFilter::Local,
                                "Local",
                            );
                        });
                });

                ui.add_space(15.0);

                // Load extension registry
                use crate::mcp::extensions::ExtensionRegistry;
                use std::path::PathBuf;

                let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let registry_path = home_dir
                    .join(".rustbot")
                    .join("extensions")
                    .join("registry.json");

                match ExtensionRegistry::load(&registry_path) {
                    Ok(registry) => {
                        let all_extensions = registry.list();

                        // Apply filter
                        let filtered_extensions: Vec<_> = all_extensions
                            .iter()
                            .filter(|ext| {
                                use crate::mcp::extensions::InstallationType;
                                use crate::ui::InstallTypeFilter;

                                match self.installed_extensions_filter {
                                    InstallTypeFilter::All => true,
                                    InstallTypeFilter::Remote => {
                                        matches!(ext.install_type, InstallationType::Remote)
                                    }
                                    InstallTypeFilter::Local => {
                                        matches!(ext.install_type, InstallationType::Local)
                                    }
                                }
                            })
                            .collect();

                        if filtered_extensions.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                if all_extensions.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} No extensions installed",
                                            icons::PACKAGE
                                        ))
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(120, 120, 120)),
                                    );
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Visit the Marketplace to discover and install MCP servers",
                                        )
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} No {} extensions installed",
                                            icons::PACKAGE,
                                            self.installed_extensions_filter.label().to_lowercase()
                                        ))
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(120, 120, 120)),
                                    );
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new("Try selecting a different filter")
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(150, 150, 150)),
                                    );
                                }
                            });
                        } else {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} extension(s) shown ({} total installed)",
                                    filtered_extensions.len(),
                                    all_extensions.len()
                                ))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                            ui.add_space(10.0);

                            // Display each filtered extension
                            for ext in filtered_extensions {
                                ui.group(|ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.add_space(5.0);

                                    // Extension header with type badge
                                    ui.horizontal(|ui| {
                                        // Type badge (Remote or Local)
                                        use crate::mcp::extensions::InstallationType;
                                        let (badge_icon, badge_text, badge_color) = match ext.install_type {
                                            InstallationType::Remote => (
                                                icons::CLOUD,
                                                "Remote",
                                                egui::Color32::from_rgb(80, 150, 220),
                                            ),
                                            InstallationType::Local => (
                                                icons::DESKTOP,
                                                "Local",
                                                egui::Color32::from_rgb(100, 180, 100),
                                            ),
                                        };

                                        // Draw badge with background
                                        let badge_label = format!("{} {}", badge_icon, badge_text);
                                        ui.label(
                                            egui::RichText::new(badge_label)
                                                .size(11.0)
                                                .color(badge_color)
                                                .strong(),
                                        );

                                        ui.add_space(10.0);

                                        ui.label(
                                            egui::RichText::new(&ext.name)
                                                .size(16.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(60, 120, 180)),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} v{}",
                                                    icons::TAG,
                                                    ext.metadata.version
                                                ))
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(120, 120, 120)),
                                            );
                                        });
                                    });

                                    ui.add_space(5.0);

                                    // Description
                                    if !ext.description.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&ext.description)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(100, 100, 100)),
                                        );
                                        ui.add_space(5.0);
                                    }

                                    // Installation date
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} Installed: {}",
                                                icons::CALENDAR,
                                                ext.metadata.installed_at.split('T').next().unwrap_or("Unknown")
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(120, 120, 120)),
                                        );
                                    });

                                    ui.add_space(5.0);

                                    // Action buttons
                                    ui.horizontal(|ui| {
                                        if !ext.metadata.repository_url.is_empty() {
                                            if ui.button(format!("{} Repository", icons::GITHUB_LOGO)).clicked() {
                                                ui.output_mut(|o| o.copied_text = ext.metadata.repository_url.clone());
                                                tracing::info!("Repository URL copied to clipboard: {}", ext.metadata.repository_url);
                                            }
                                        }

                                        if ui.button(format!("{} Configure", icons::GEAR)).clicked() {
                                            self.configuring_extension_id = Some(ext.id.clone());
                                        }

                                        if ui
                                            .button(
                                                egui::RichText::new(format!("{} Uninstall", icons::TRASH))
                                                    .color(egui::Color32::from_rgb(180, 60, 60)),
                                            )
                                            .clicked()
                                        {
                                            // Show confirmation dialog
                                            self.uninstall_confirmation = Some((ext.id.clone(), ext.name.clone()));
                                        }
                                    });

                                    ui.add_space(5.0);
                                });

                                ui.add_space(10.0);
                            }
                        }
                    }
                    Err(e) => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                egui::RichText::new(format!("{} Failed to load extensions", icons::WARNING))
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(200, 80, 80)),
                            );
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(format!("Error: {}", e))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                            );
                        });
                    }
                }
            });
    }

    /// Render extension configuration dialog
    fn render_extension_config_dialog(&mut self, ui: &mut egui::Ui, ext_id: &str) {
        use crate::mcp::extensions::ExtensionRegistry;
        use std::path::PathBuf;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading(format!("{} Configure Extension", icons::GEAR));
                ui.add_space(10.0);

                // Back button
                if ui.button(format!("{} Back to Extensions", icons::ARROW_LEFT)).clicked() {
                    self.configuring_extension_id = None;
                    self.extension_config_message = None;
                    return;
                }

                ui.add_space(15.0);

                // Load extension info
                let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let registry_path = home_dir.join(".rustbot").join("extensions").join("registry.json");

                let extension = match ExtensionRegistry::load(&registry_path) {
                    Ok(registry) => registry.get(ext_id).cloned(),
                    Err(_) => None,
                };

                if let Some(ext) = extension {
                    // Extension header
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(&ext.name).size(18.0).strong());
                        ui.label(&ext.description);
                        ui.add_space(10.0);
                    });

                    ui.add_space(15.0);

                    // Show configuration message if any
                    if let Some((message, is_error)) = &self.extension_config_message {
                        let color = if *is_error {
                            egui::Color32::from_rgb(200, 80, 80)
                        } else {
                            egui::Color32::from_rgb(60, 150, 60)
                        };
                        ui.label(egui::RichText::new(message).color(color));
                        ui.add_space(10.0);
                    }

                    // Agent configuration section
                    ui.label(egui::RichText::new("Enable for Agents:").size(16.0).strong());
                    ui.add_space(10.0);

                    // Iterate through agents and show checkboxes
                    let mut changes_made = false;
                    for (idx, agent_config) in self.agent_configs.iter_mut().enumerate() {
                        let mut is_enabled = agent_config.mcp_extensions.contains(&ext.id);

                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut is_enabled, "").changed() {
                                if is_enabled {
                                    // Add extension to agent
                                    if !agent_config.mcp_extensions.contains(&ext.id) {
                                        agent_config.mcp_extensions.push(ext.id.clone());
                                        changes_made = true;
                                    }
                                } else {
                                    // Remove extension from agent
                                    agent_config.mcp_extensions.retain(|id| id != &ext.id);
                                    changes_made = true;
                                }
                            }

                            ui.label(egui::RichText::new(&agent_config.name).size(14.0));
                        });

                        ui.add_space(5.0);
                    }

                    // Save button
                    ui.add_space(15.0);
                    if ui.button(format!("{} Save Configuration", icons::FLOPPY_DISK)).clicked() {
                        // Save all modified agent configs
                        let mut save_errors = Vec::new();
                        let mut saved_count = 0;

                        for agent_config in &self.agent_configs {
                            // Build path to agent JSON file
                            let agent_path = PathBuf::from("agents")
                                .join("presets")
                                .join(format!("{}.json", agent_config.name));

                            // Save the updated config
                            match serde_json::to_string_pretty(&agent_config) {
                                Ok(json) => {
                                    if let Err(e) = std::fs::write(&agent_path, json) {
                                        save_errors.push(format!("{}: {}", agent_config.name, e));
                                    } else {
                                        saved_count += 1;
                                    }
                                }
                                Err(e) => {
                                    save_errors.push(format!("{}: {}", agent_config.name, e));
                                }
                            }
                        }

                        if save_errors.is_empty() {
                            self.extension_config_message = Some((
                                format!("✓ Configuration saved! {} agent(s) updated. Tools will be available instantly.", saved_count),
                                false,
                            ));
                            // Reload agent configs to pick up changes
                            // This will trigger tool reload in the next update cycle
                        } else {
                            self.extension_config_message = Some((
                                format!("✗ Failed to save some configs: {}", save_errors.join(", ")),
                                true,
                            ));
                        }
                    }

                } else {
                    ui.label(
                        egui::RichText::new("Extension not found")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(200, 80, 80)),
                    );
                }
            });
    }

    /// Render uninstall confirmation dialog
    fn render_uninstall_confirmation_dialog(
        &mut self,
        ui: &mut egui::Ui,
        ext_id: &str,
        ext_name: &str,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading(format!("{} Uninstall Extension", icons::WARNING));
                ui.add_space(20.0);

                // Show uninstall message if any
                if let Some((message, is_error)) = &self.uninstall_message {
                    let color = if *is_error {
                        egui::Color32::from_rgb(200, 80, 80)
                    } else {
                        egui::Color32::from_rgb(60, 150, 60)
                    };
                    ui.label(egui::RichText::new(message).size(14.0).color(color));
                    ui.add_space(15.0);

                    // If successful uninstall, show back button
                    if !is_error {
                        if ui
                            .button(format!("{} Back to Extensions", icons::ARROW_LEFT))
                            .clicked()
                        {
                            self.uninstall_confirmation = None;
                            self.uninstall_message = None;
                        }
                        return;
                    }
                }

                // Warning box
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new(format!(
                            "{} Warning: This action cannot be undone",
                            icons::WARNING
                        ))
                        .size(16.0)
                        .strong()
                        .color(egui::Color32::from_rgb(200, 120, 40)),
                    );

                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new(format!("Extension: {}", ext_name))
                            .size(14.0)
                            .strong(),
                    );
                    ui.label(egui::RichText::new(format!("ID: {}", ext_id)).size(12.0));

                    ui.add_space(10.0);

                    ui.label("This will:");
                    ui.label("  • Remove the extension from your system");
                    ui.label("  • Remove it from all agent configurations");
                    ui.label("  • Remove MCP configuration entries");

                    ui.add_space(10.0);
                });

                ui.add_space(20.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Cancel", icons::X)).clicked() {
                        self.uninstall_confirmation = None;
                        self.uninstall_message = None;
                    }

                    ui.add_space(10.0);

                    if ui
                        .button(
                            egui::RichText::new(format!("{} Uninstall", icons::TRASH))
                                .color(egui::Color32::from_rgb(180, 60, 60)),
                        )
                        .clicked()
                    {
                        // Perform uninstall
                        match self.perform_uninstall(ext_id) {
                            Ok(()) => {
                                self.uninstall_message = Some((
                                    format!("✓ Extension '{}' uninstalled successfully", ext_name),
                                    false,
                                ));
                            }
                            Err(e) => {
                                self.uninstall_message =
                                    Some((format!("✗ Failed to uninstall extension: {}", e), true));
                            }
                        }
                    }
                });
            });
    }

    /// Perform extension uninstall
    ///
    /// Removes the extension from:
    /// 1. Extension registry (~/.rustbot/extensions/registry.json)
    /// 2. Global MCP config (~/.rustbot/mcp_config.json) if present
    /// 3. All agent-specific MCP configs (~/.rustbot/mcp_configs/*.json)
    /// 4. All agent presets (agents/presets/*.json) mcp_extensions field
    fn perform_uninstall(&mut self, extension_id: &str) -> anyhow::Result<()> {
        use crate::mcp::extensions::ExtensionRegistry;
        use std::path::PathBuf;

        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        // 1. Remove from extension registry
        let registry_path = home_dir
            .join(".rustbot")
            .join("extensions")
            .join("registry.json");

        let mut registry = ExtensionRegistry::load(&registry_path)?;
        if registry.uninstall(extension_id).is_none() {
            return Err(anyhow::anyhow!(
                "Extension '{}' not found in registry",
                extension_id
            ));
        }
        registry.save(&registry_path)?;
        tracing::info!("✓ Removed extension '{}' from registry", extension_id);

        // 2. Remove from global MCP config if it exists
        let global_config_path = home_dir.join(".rustbot").join("mcp_config.json");
        if global_config_path.exists() {
            use crate::mcp::config::McpConfig;
            let mut config = McpConfig::load_from_file(&global_config_path)?;
            if config.remove_extension(extension_id) {
                config.save_to_file(&global_config_path)?;
                tracing::info!("✓ Removed extension from global MCP config");
            }
        }

        // 3. Remove from all agent-specific MCP configs
        let mcp_configs_dir = home_dir.join(".rustbot").join("mcp_configs");
        if mcp_configs_dir.exists() {
            use crate::mcp::config::McpConfig;
            for entry in std::fs::read_dir(&mcp_configs_dir)? {
                let path = entry?.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let mut config = McpConfig::load_from_file(&path)?;
                    if config.remove_extension(extension_id) {
                        config.save_to_file(&path)?;
                        tracing::info!("✓ Removed extension from {:?}", path.file_name());
                    }
                }
            }
        }

        // 4. Remove from all agent preset configs (agents/presets/*.json)
        for agent_config in &mut self.agent_configs {
            if agent_config
                .mcp_extensions
                .contains(&extension_id.to_string())
            {
                agent_config.mcp_extensions.retain(|id| id != extension_id);

                // Save the updated agent config
                let agent_path = PathBuf::from("agents")
                    .join("presets")
                    .join(format!("{}.json", agent_config.name));

                let json = serde_json::to_string_pretty(&agent_config)?;
                std::fs::write(&agent_path, json)?;
                tracing::info!("✓ Removed extension from agent '{}'", agent_config.name);
            }
        }

        Ok(())
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

                            ui.label(egui::RichText::new(&icon).strong().size(16.0));

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
                                    if ui
                                        .button(format!("{} Edit", icons::PENCIL_SIMPLE))
                                        .clicked()
                                    {
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
                            let role = if config.is_primary {
                                "Primary"
                            } else {
                                "Specialist"
                            };
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} • Model: {} • Web Search: {}",
                                    role,
                                    config.model.split('/').last().unwrap_or(&config.model),
                                    if config.web_search_enabled {
                                        "✓"
                                    } else {
                                        "✗"
                                    }
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
                                .hint_text(
                                    "Enter agent personality traits (leave empty for none)...",
                                )
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
                                    "openai/gpt-5.1-turbo".to_string(),
                                    "GPT-5.1 Turbo",
                                );
                                ui.selectable_value(
                                    &mut config.model,
                                    "openai/gpt-4o".to_string(),
                                    "GPT-4o",
                                );
                                ui.selectable_value(
                                    &mut config.model,
                                    "anthropic/claude-opus-4".to_string(),
                                    "Claude Opus 4",
                                );
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

    /// Render the preferences view for UI customization
    ///
    /// Allows configuration of:
    /// - Theme (light/dark mode)
    /// - Other UI preferences (future: font size, etc.)
    ///
    /// Changes are saved immediately to user profile
    ///
    /// # Arguments
    /// * `ui` - The egui UI context for rendering
    pub fn render_preferences_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.heading("Preferences");
                ui.add_space(10.0);

                ui.label("Customize the application appearance and behavior:");
                ui.add_space(15.0);

                // Theme selection
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Theme").strong().size(16.0));
                    ui.add_space(5.0);
                    ui.label("Choose between light and dark mode:");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        let theme_changed = if ui
                            .selectable_label(!self.dark_mode, format!("{} Light", icons::SUN))
                            .clicked()
                        {
                            self.dark_mode = false;
                            true
                        } else if ui
                            .selectable_label(self.dark_mode, format!("{} Dark", icons::MOON))
                            .clicked()
                        {
                            self.dark_mode = true;
                            true
                        } else {
                            false
                        };

                        // Save theme preference to user profile
                        if theme_changed {
                            let storage = Arc::clone(&self.deps.storage);
                            let theme = if self.dark_mode { "dark" } else { "light" }.to_string();
                            let runtime = self
                                .deps
                                .runtime
                                .as_ref()
                                .expect("Runtime is required for RustbotApp");

                            runtime.spawn(async move {
                                // Load current profile
                                if let Ok(mut profile) = storage.load_user_profile().await {
                                    // Update theme
                                    profile.theme = theme.clone();

                                    // Save updated profile
                                    if let Err(e) = storage.save_user_profile(&profile).await {
                                        tracing::error!("Failed to save theme preference: {}", e);
                                    } else {
                                        tracing::info!("Theme preference saved: {}", theme);
                                    }
                                }
                            });
                        }
                    });

                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(if self.dark_mode {
                            "Currently using Dark theme"
                        } else {
                            "Currently using Light theme"
                        })
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                    );
                });

                ui.add_space(20.0);

                // Future preferences can be added here
                // Example: Font size, animations, etc.
            });
    }
}
