//! Plugins View UI Component
//!
//! Comprehensive UI for managing MCP plugins in Rustbot.
//!
//! Design Decision: Two-column layout with real-time updates
//!
//! Rationale: Plugin list in left column provides overview, details panel
//! on right shows comprehensive information. Event-driven updates ensure
//! UI stays in sync with plugin state changes without manual refresh.
//!
//! Trade-offs:
//! - Auto-refresh vs Manual: Convenience vs control (using auto-refresh)
//! - Polling vs Events: Event bus for real-time updates (no polling)
//! - Mutable access: Async spawn for operations to avoid blocking UI
//!
//! UI Components:
//! 1. Plugin List (left): Status, name, tool count
//! 2. Plugin Details (right): Full info, control buttons
//! 3. Recent Events (bottom): Last 10 events with timestamps
//! 4. Global Controls (toolbar): Reload config

use eframe::egui;
use egui_phosphor::regular as icons;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::manager::McpPluginManager;
use crate::mcp::plugin::{PluginMetadata, PluginState};
use crate::events::{Event, EventBus, EventKind, McpPluginEvent, PluginHealthStatus};

/// Plugins management view
///
/// Displays all MCP plugins with:
/// - Real-time status indicators
/// - Start/Stop/Restart controls
/// - Tool listings
/// - Event history
pub struct PluginsView {
    /// MCP plugin manager (shared with main app)
    mcp_manager: Arc<Mutex<McpPluginManager>>,

    /// Cached plugin list (updated via refresh)
    plugins: Vec<PluginMetadata>,

    /// Currently selected plugin ID for detail view
    selected_plugin: Option<String>,

    /// Recent plugin events for display
    recent_events: VecDeque<String>,

    /// Last refresh timestamp
    last_refresh: std::time::Instant,

    /// Auto-refresh interval (seconds)
    refresh_interval: u64,
}

impl PluginsView {
    /// Create new plugins view
    ///
    /// # Arguments
    /// * `mcp_manager` - Shared reference to plugin manager
    pub fn new(mcp_manager: Arc<Mutex<McpPluginManager>>) -> Self {
        Self {
            mcp_manager,
            plugins: Vec::new(),
            selected_plugin: None,
            recent_events: VecDeque::with_capacity(50),
            last_refresh: std::time::Instant::now(),
            refresh_interval: 2, // 2 seconds
        }
    }

    /// Refresh plugin list from manager
    ///
    /// Fetches current plugin state asynchronously and updates cache.
    /// Called automatically on refresh interval or manually via button.
    pub async fn refresh_plugins(&mut self) {
        let manager = self.mcp_manager.lock().await;

        // Get all plugins using the public list_plugins method
        let plugin_infos = manager.list_plugins().await;

        // Convert PluginInfo to PluginMetadata by fetching full details
        self.plugins.clear();
        for info in plugin_infos {
            if let Some(metadata) = manager.get_plugin(&info.id).await {
                self.plugins.push(metadata);
            }
        }
    }

    /// Main render method
    ///
    /// Draws the complete plugins UI with:
    /// - Header with title
    /// - Two-column layout (list | details)
    /// - Recent events collapsible section
    /// - Global controls
    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Header
        ui.horizontal(|ui| {
            ui.heading(format!("{} MCP Plugins", icons::PUZZLE_PIECE));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Reload config button
                if ui.button(format!("{} Reload Config", icons::ARROW_CLOCKWISE))
                    .on_hover_text("Reload plugin configuration from disk")
                    .clicked()
                {
                    self.reload_config(ctx);
                }

                // Manual refresh button
                if ui.button(format!("{} Refresh", icons::ARROWS_CLOCKWISE))
                    .on_hover_text("Refresh plugin list now")
                    .clicked()
                {
                    self.trigger_refresh(ctx);
                }
            });
        });

        ui.separator();

        // Auto-refresh check
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(self.refresh_interval) {
            self.trigger_refresh(ctx);
            self.last_refresh = std::time::Instant::now();
        }

        // Main content area with scrolling
        let available_height = ui.available_height() - 150.0; // Reserve space for events

        egui::ScrollArea::vertical()
            .max_height(available_height.max(300.0))
            .show(ui, |ui| {
                // Two-column layout
                ui.columns(2, |columns| {
                    // Left: Plugin List
                    columns[0].vertical(|ui| {
                        self.render_plugin_list(ui, ctx);
                    });

                    // Right: Plugin Details
                    columns[1].vertical(|ui| {
                        self.render_plugin_details(ui, ctx);
                    });
                });
            });

        // Bottom: Recent Events
        ui.separator();
        ui.collapsing(format!("{} Recent Events", icons::LIST_BULLETS), |ui| {
            self.render_recent_events(ui);
        });
    }

    /// Render plugin list (left column)
    ///
    /// Shows all plugins as selectable cards with:
    /// - Status indicator (colored dot)
    /// - Plugin name
    /// - Tool count badge
    fn render_plugin_list(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Available Plugins");
        ui.add_space(5.0);

        if self.plugins.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No plugins configured")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(120, 120, 120))
                );
                ui.label(
                    egui::RichText::new("Create mcp_config.json to add plugins")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150))
                );
            });
            return;
        }

        // Display each plugin
        for plugin in &self.plugins {
            let is_selected = self.selected_plugin.as_ref() == Some(&plugin.id);

            // Plugin card
            let response = ui.group(|ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Status indicator (colored dot)
                    let (status_icon, color) = get_status_icon_and_color(&plugin.state);
                    ui.colored_label(color, status_icon);

                    // Plugin name
                    ui.label(
                        egui::RichText::new(&plugin.name)
                            .strong()
                            .size(14.0)
                    );

                    // Tool count badge
                    if !plugin.tools.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} tools", plugin.tools.len()))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100))
                            );
                        });
                    }
                });

                // State text
                ui.horizontal(|ui| {
                    ui.add_space(15.0); // Indent
                    let state_text = get_state_text(&plugin.state);
                    ui.label(
                        egui::RichText::new(state_text)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 120, 120))
                    );
                });
            });

            // Handle selection
            if response.response.clicked() {
                self.selected_plugin = Some(plugin.id.clone());
            }

            // Highlight selected
            if is_selected {
                ui.painter().rect_stroke(
                    response.response.rect,
                    egui::Rounding::same(2),
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 120, 220)),
                    egui::epaint::StrokeKind::Outside
                );
            }

            ui.add_space(5.0);
        }
    }

    /// Render plugin details (right column)
    ///
    /// Shows comprehensive information about selected plugin:
    /// - Plugin name and ID
    /// - Current state with visual indicator
    /// - Restart count (if any)
    /// - Tools list with descriptions
    /// - Control buttons (Start/Stop/Restart)
    fn render_plugin_details(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(plugin_id) = &self.selected_plugin {
            if let Some(plugin) = self.plugins.iter().find(|p| &p.id == plugin_id) {
                // Plugin header
                ui.heading(&plugin.name);
                ui.label(
                    egui::RichText::new(format!("ID: {}", plugin.id))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 120, 120))
                );

                ui.add_space(5.0);

                // Description (if available)
                if let Some(desc) = &plugin.description {
                    ui.label(
                        egui::RichText::new(desc)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(80, 80, 80))
                    );
                    ui.add_space(5.0);
                }

                ui.separator();

                // Status section
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Status:").strong());
                    let (status_icon, color) = get_status_icon_and_color(&plugin.state);
                    ui.colored_label(color, status_icon);
                    let state_text = get_state_text(&plugin.state);
                    ui.colored_label(color, state_text);
                });

                // Error message (if in error state)
                if let Some(error_msg) = plugin.error_message() {
                    ui.add_space(5.0);
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 60, 60),
                            format!("{} Error:", icons::WARNING)
                        );
                        ui.label(
                            egui::RichText::new(error_msg)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 50, 50))
                        );
                    });
                }

                // Restart info (if plugin has been restarted)
                if plugin.restart_count > 0 {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} Restarts: {}/{}",
                            icons::ARROW_CLOCKWISE,
                            plugin.restart_count,
                            plugin.max_retries
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(200, 150, 50))
                    );
                }

                ui.add_space(10.0);
                ui.separator();

                // Tools section
                ui.label(
                    egui::RichText::new(format!("{} Tools ({})", icons::WRENCH, plugin.tools.len()))
                        .strong()
                );
                ui.add_space(5.0);

                if plugin.tools.is_empty() {
                    ui.label(
                        egui::RichText::new("No tools available")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 120, 120))
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for tool in &plugin.tools {
                                ui.horizontal(|ui| {
                                    ui.label("•");
                                    ui.label(egui::RichText::new(&tool.name).strong().size(12.0));
                                });

                                if let Some(desc) = &tool.description {
                                    ui.horizontal(|ui| {
                                        ui.add_space(15.0);
                                        ui.label(
                                            egui::RichText::new(desc)
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(100, 100, 100))
                                        );
                                    });
                                }
                                ui.add_space(3.0);
                            }
                        });
                }

                ui.add_space(10.0);
                ui.separator();

                // Control buttons
                ui.horizontal(|ui| {
                    match &plugin.state {
                        PluginState::Running => {
                            if ui.button(format!("{} Stop", icons::STOP))
                                .on_hover_text("Stop this plugin")
                                .clicked()
                            {
                                self.stop_plugin(plugin_id, ctx);
                            }

                            if ui.button(format!("{} Restart", icons::ARROW_CLOCKWISE))
                                .on_hover_text("Restart this plugin")
                                .clicked()
                            {
                                self.restart_plugin(plugin_id, ctx);
                            }
                        }
                        PluginState::Stopped | PluginState::Disabled | PluginState::Error { .. } => {
                            if ui.button(format!("{} Start", icons::PLAY))
                                .on_hover_text("Start this plugin")
                                .clicked()
                            {
                                self.start_plugin(plugin_id, ctx);
                            }
                        }
                        PluginState::Starting | PluginState::Initializing | PluginState::Stopping => {
                            ui.label(
                                egui::RichText::new("Operation in progress...")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150))
                            );
                        }
                    }
                });

            } else {
                // Selected plugin not found (might have been removed)
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(
                        egui::RichText::new("Plugin not found")
                            .color(egui::Color32::from_rgb(150, 150, 150))
                    );
                });
                self.selected_plugin = None; // Clear invalid selection
            }
        } else {
            // No plugin selected
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("← Select a plugin to view details")
                        .color(egui::Color32::from_rgb(120, 120, 120))
                );
            });
        }
    }

    /// Render recent events (bottom section)
    ///
    /// Shows last 10 events with timestamps in reverse chronological order.
    fn render_recent_events(&self, ui: &mut egui::Ui) {
        if self.recent_events.is_empty() {
            ui.label(
                egui::RichText::new("No recent events")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            );
        } else {
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    // Show most recent first
                    for event in self.recent_events.iter().rev().take(10) {
                        ui.horizontal(|ui| {
                            ui.label("•");
                            ui.label(
                                egui::RichText::new(event)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(80, 80, 80))
                            );
                        });
                    }
                });
        }
    }

    /// Add an event to the recent events list
    ///
    /// Events are stored in a circular buffer (last 50 events).
    pub fn add_event(&mut self, event_text: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.recent_events.push_back(format!("[{}] {}", timestamp, event_text));

        // Keep only last 50 events
        if self.recent_events.len() > 50 {
            self.recent_events.pop_front();
        }
    }

    /// Handle MCP plugin events from event bus
    ///
    /// Updates recent events list based on plugin lifecycle events.
    /// Call this from the main app's event loop.
    pub fn handle_mcp_event(&mut self, event: &McpPluginEvent) {
        let event_text = format_plugin_event(event);
        self.add_event(event_text);
    }

    // ========================================================================
    // Plugin Control Actions (async spawned to avoid blocking UI)
    // ========================================================================

    /// Trigger manual refresh
    fn trigger_refresh(&self, ctx: &egui::Context) {
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            // Refresh will happen on next render
            ctx_clone.request_repaint();
        });
    }

    /// Start a plugin
    fn start_plugin(&self, plugin_id: &str, ctx: &egui::Context) {
        let manager = Arc::clone(&self.mcp_manager);
        let id = plugin_id.to_string();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let mut mgr = manager.lock().await;
            match mgr.start_plugin(&id).await {
                Ok(_) => {
                    tracing::info!("Plugin '{}' started successfully", id);
                }
                Err(e) => {
                    tracing::error!("Failed to start plugin '{}': {}", id, e);
                }
            }
            ctx_clone.request_repaint();
        });
    }

    /// Stop a plugin
    fn stop_plugin(&self, plugin_id: &str, ctx: &egui::Context) {
        let manager = Arc::clone(&self.mcp_manager);
        let id = plugin_id.to_string();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let mut mgr = manager.lock().await;
            match mgr.stop_plugin(&id).await {
                Ok(_) => {
                    tracing::info!("Plugin '{}' stopped successfully", id);
                }
                Err(e) => {
                    tracing::error!("Failed to stop plugin '{}': {}", id, e);
                }
            }
            ctx_clone.request_repaint();
        });
    }

    /// Restart a plugin (stop + start)
    fn restart_plugin(&self, plugin_id: &str, ctx: &egui::Context) {
        let id = plugin_id.to_string();

        // Stop first
        self.stop_plugin(&id, ctx);

        // Start after brief delay
        let manager = Arc::clone(&self.mcp_manager);
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let mut mgr = manager.lock().await;
            match mgr.start_plugin(&id).await {
                Ok(_) => {
                    tracing::info!("Plugin '{}' restarted successfully", id);
                }
                Err(e) => {
                    tracing::error!("Failed to restart plugin '{}': {}", id, e);
                }
            }
            ctx_clone.request_repaint();
        });
    }

    /// Reload configuration from disk
    fn reload_config(&self, ctx: &egui::Context) {
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            // TODO: Implement config hot-reload
            // For now, just log that it was requested
            tracing::info!("Config reload requested (not yet implemented)");

            ctx_clone.request_repaint();
        });
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get status icon and color for plugin state
///
/// Returns (icon_text, color) tuple for rendering.
fn get_status_icon_and_color(state: &PluginState) -> (&'static str, egui::Color32) {
    match state {
        PluginState::Running => ("●", egui::Color32::from_rgb(60, 150, 60)), // Green
        PluginState::Starting | PluginState::Initializing => {
            ("◐", egui::Color32::from_rgb(200, 180, 50)) // Yellow
        }
        PluginState::Stopping => ("◐", egui::Color32::from_rgb(200, 150, 100)), // Orange
        PluginState::Stopped => ("○", egui::Color32::from_rgb(120, 120, 120)), // Gray
        PluginState::Disabled => ("○", egui::Color32::from_rgb(150, 150, 150)), // Light gray
        PluginState::Error { .. } => ("✖", egui::Color32::from_rgb(200, 60, 60)), // Red
    }
}

/// Get human-readable state text
fn get_state_text(state: &PluginState) -> &'static str {
    match state {
        PluginState::Running => "Running",
        PluginState::Starting => "Starting...",
        PluginState::Initializing => "Initializing...",
        PluginState::Stopping => "Stopping...",
        PluginState::Stopped => "Stopped",
        PluginState::Disabled => "Disabled",
        PluginState::Error { .. } => "Error",
    }
}

/// Format MCP plugin event as human-readable text
fn format_plugin_event(event: &McpPluginEvent) -> String {
    match event {
        McpPluginEvent::Started { plugin_id, tool_count } => {
            format!("✓ {} started ({} tools)", plugin_id, tool_count)
        }
        McpPluginEvent::Stopped { plugin_id } => {
            format!("○ {} stopped", plugin_id)
        }
        McpPluginEvent::Error { plugin_id, message } => {
            format!("✖ {} error: {}", plugin_id, message)
        }
        McpPluginEvent::ToolsChanged { plugin_id, tool_count } => {
            format!("🔧 {} tools changed ({} tools)", plugin_id, tool_count)
        }
        McpPluginEvent::HealthStatus { plugin_id, status } => {
            let status_text = match status {
                PluginHealthStatus::Healthy => "healthy",
                PluginHealthStatus::Unresponsive => "unresponsive",
                PluginHealthStatus::Dead => "dead",
            };
            format!("🏥 {} is {}", plugin_id, status_text)
        }
        McpPluginEvent::RestartAttempt { plugin_id, attempt, max_retries } => {
            format!("↻ {} restarting ({}/{})", plugin_id, attempt, max_retries)
        }
        McpPluginEvent::ConfigReloaded { plugins_added, plugins_removed, plugins_updated } => {
            format!(
                "🔄 Config reloaded: +{} -{} ~{}",
                plugins_added.len(),
                plugins_removed.len(),
                plugins_updated.len()
            )
        }
    }
}
