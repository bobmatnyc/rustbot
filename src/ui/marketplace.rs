//! Marketplace View UI Component
//!
//! Browser for discovering and viewing available MCP servers from the
//! official Anthropic registry.
//!
//! # Design Rationale
//!
//! **Two-Column Layout**: Server list on left, details on right
//! - Matches common UI patterns (email clients, file browsers)
//! - Allows quick scanning of servers while viewing details
//!
//! **Search-First UX**: Prominent search bar at top
//! - Most users will search rather than browse all servers
//! - Filters are secondary controls below search
//!
//! **Async Data Fetching**: Background API calls via tokio::spawn
//! - Prevents UI blocking during network requests
//! - Uses channels to send results back to UI thread
//!
//! # State Management Strategy
//!
//! - `servers`: Cached server listings (refreshed on search/filter)
//! - `selected_server`: Index into servers vec for details panel
//! - `is_loading`: Shows spinner during API calls
//! - `error_message`: Displays user-facing error messages
//!
//! # Performance Considerations
//!
//! - Server list limited to 20-50 items per page (pagination reduces memory)
//! - No automatic refresh (user-triggered only)
//! - Details panel renders only selected server (not entire list)
//!
//! # Future Enhancements (Phase 2+)
//!
//! - One-click install integration with plugin manager
//! - Local caching of registry data (reduce API calls)
//! - Favorites/bookmarks for frequently used servers
//! - Installation status tracking (installed, not installed, update available)

use eframe::egui;
use egui_phosphor::regular as icons;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::runtime::Handle;

use crate::mcp::marketplace::{MarketplaceClient, McpServerWrapper, McpRegistry};

/// Async task result for server list fetch
enum FetchResult {
    Success(McpRegistry),
    Error(String),
}

/// Marketplace view state
///
/// Manages UI state and async data fetching for the marketplace browser.
///
/// # Thread Safety
///
/// All async operations spawn on the provided `runtime` handle.
/// Results are sent back via `fetch_rx` channel (mpsc receiver on UI thread).
pub struct MarketplaceView {
    /// Marketplace API client
    client: Arc<MarketplaceClient>,

    /// Tokio runtime handle for async operations
    runtime: Handle,

    /// Cached server listings
    servers: Vec<McpServerWrapper>,

    /// Selected server index for details view
    selected_server: Option<usize>,

    /// Search query string
    search_query: String,

    /// Filter by package type (None = show all)
    package_type_filter: Option<String>,

    /// Filter by official status
    show_official_only: bool,

    /// Loading state (true when API call in progress)
    is_loading: bool,

    /// Error message for display
    error_message: Option<String>,

    /// Current page number (for pagination)
    current_page: usize,

    /// Total servers available (from pagination metadata)
    total_servers: usize,

    /// Servers per page
    servers_per_page: usize,

    /// Next cursor for pagination (None = last page)
    next_cursor: Option<String>,

    /// Receiver for async fetch results
    fetch_rx: mpsc::UnboundedReceiver<FetchResult>,

    /// Sender for async fetch results (cloned for each async task)
    fetch_tx: mpsc::UnboundedSender<FetchResult>,
}

impl MarketplaceView {
    /// Create a new marketplace view
    ///
    /// # Arguments
    /// * `runtime` - Tokio runtime handle for spawning async tasks
    pub fn new(runtime: Handle) -> Self {
        let (fetch_tx, fetch_rx) = mpsc::unbounded_channel();

        let mut view = Self {
            client: Arc::new(MarketplaceClient::new()),
            runtime,
            servers: Vec::new(),
            selected_server: None,
            search_query: String::new(),
            package_type_filter: None,
            show_official_only: false,
            is_loading: false,
            error_message: None,
            current_page: 0,
            total_servers: 0,
            servers_per_page: 20,
            next_cursor: None,
            fetch_rx,
            fetch_tx,
        };

        // Trigger initial load
        view.refresh_servers();

        view
    }

    /// Trigger server list refresh
    ///
    /// Spawns async task to fetch servers from API based on current search/filter state.
    /// Results are sent back via `fetch_tx` channel and processed in `update()`.
    pub fn refresh_servers(&mut self) {
        self.is_loading = true;
        self.error_message = None;

        let client = Arc::clone(&self.client);
        let offset = self.current_page * self.servers_per_page;
        let limit = self.servers_per_page;
        let search = self.search_query.clone();
        let tx = self.fetch_tx.clone();

        // Spawn async task to fetch servers
        self.runtime.spawn(async move {
            let result = if search.is_empty() {
                client.list_servers(limit, offset).await
            } else {
                client.search_servers(&search, limit).await
            };

            let message = match result {
                Ok(registry) => FetchResult::Success(registry),
                Err(e) => FetchResult::Error(e.to_string()),
            };

            let _ = tx.send(message);
        });
    }

    /// Process async fetch results
    ///
    /// Called from `render()` to check for completed async tasks.
    fn update(&mut self) {
        // Process all pending fetch results
        while let Ok(result) = self.fetch_rx.try_recv() {
            self.is_loading = false;

            match result {
                FetchResult::Success(registry) => {
                    self.servers = registry.servers;
                    if let Some(metadata) = registry.metadata {
                        self.total_servers = metadata.count;
                        self.next_cursor = metadata.next_cursor;
                    } else {
                        self.total_servers = self.servers.len();
                        self.next_cursor = None;
                    }
                    self.error_message = None;

                    // Reset selection if out of bounds
                    if let Some(idx) = self.selected_server {
                        if idx >= self.servers.len() {
                            self.selected_server = None;
                        }
                    }
                }
                FetchResult::Error(error) => {
                    self.error_message = Some(error);
                }
            }
        }
    }

    /// Main render method
    ///
    /// Renders the complete marketplace UI: search bar, filters, server list, and details panel.
    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Process any pending async results
        self.update();

        ui.heading(format!("{} MCP Marketplace", icons::STOREFRONT));

        ui.add_space(10.0);

        // Search and filters
        self.render_search_bar(ui);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Two-column layout: Server list | Details
        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                self.render_server_list(ui);
            });

            columns[1].vertical(|ui| {
                self.render_server_details(ui);
            });
        });
    }

    /// Render search bar and filters
    fn render_search_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("{} Search:", icons::MAGNIFYING_GLASS));

            let response = ui.text_edit_singleline(&mut self.search_query);

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.current_page = 0; // Reset to first page on new search
                self.refresh_servers();
            }

            if ui.button("Search").clicked() {
                self.current_page = 0;
                self.refresh_servers();
            }

            if ui.button("Clear").clicked() {
                self.search_query.clear();
                self.current_page = 0;
                self.refresh_servers();
            }
        });

        ui.horizontal(|ui| {
            let changed = ui.checkbox(&mut self.show_official_only, "Official only").changed();

            ui.label("Package type:");
            let combo_changed = egui::ComboBox::from_id_source("package_type_filter")
                .selected_text(self.package_type_filter.as_deref().unwrap_or("All"))
                .show_ui(ui, |ui| {
                    let mut changed = false;
                    changed |= ui.selectable_value(&mut self.package_type_filter, None, "All").changed();
                    changed |= ui.selectable_value(&mut self.package_type_filter, Some("npm".to_string()), "npm").changed();
                    changed |= ui.selectable_value(&mut self.package_type_filter, Some("pypi".to_string()), "PyPI").changed();
                    changed |= ui.selectable_value(&mut self.package_type_filter, Some("docker".to_string()), "Docker").changed();
                    changed |= ui.selectable_value(&mut self.package_type_filter, Some("remote".to_string()), "Remote").changed();
                    changed
                })
                .inner
                .unwrap_or(false);

            // Refresh on filter change
            if changed || combo_changed {
                // Note: Filters are applied client-side, no need to refresh from API
            }
        });
    }

    /// Render server list (left column)
    fn render_server_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Available Servers");

        if self.is_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading servers...");
            });
            return;
        }

        if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, format!("{} Error", icons::WARNING_CIRCLE));
            ui.label(error);
            if ui.button("Retry").clicked() {
                self.refresh_servers();
            }
            return;
        }

        // Show result count
        ui.label(format!(
            "Showing {} servers {}",
            self.get_filtered_count(),
            if self.total_servers > self.servers_per_page {
                format!("(page {} of {})", self.current_page + 1, self.total_pages())
            } else {
                String::new()
            }
        ));

        ui.add_space(5.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (idx, wrapper) in self.servers.iter().enumerate() {
                    let server = &wrapper.server;
                    let is_official = wrapper.meta.official.status == "active";

                    // Apply filters
                    if self.show_official_only && !is_official {
                        continue;
                    }

                    // Get package type from first package (if available)
                    let package_type = server.packages.first()
                        .map(|p| p.registry_type.as_str())
                        .or_else(|| if !server.remotes.is_empty() { Some("remote") } else { None })
                        .unwrap_or("unknown");

                    if let Some(ref filter) = self.package_type_filter {
                        if package_type != filter.as_str() {
                            continue;
                        }
                    }

                    // Render server card
                    let is_selected = self.selected_server == Some(idx);

                    let response = ui.add(
                        egui::SelectableLabel::new(is_selected, &server.name)
                    );

                    if response.clicked() {
                        self.selected_server = Some(idx);
                    }

                    ui.label(
                        egui::RichText::new(&server.description)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 120, 120))
                    );

                    ui.horizontal(|ui| {
                        // Package type badge
                        ui.label(format!("{} {}", icons::PACKAGE, package_type));

                        // Official badge
                        if is_official {
                            ui.label(egui::RichText::new(format!("{} Official", icons::SEAL_CHECK))
                                .color(egui::Color32::from_rgb(60, 150, 60)));
                        }

                        // Version
                        if !server.version.is_empty() {
                            ui.label(egui::RichText::new(format!("v{}", server.version))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 100, 100)));
                        }
                    });

                    ui.add_space(5.0);
                    ui.separator();
                }
            });

        // Pagination controls
        if self.total_servers > self.servers_per_page {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(format!("{} Previous", icons::CARET_LEFT))
                    .clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                    self.refresh_servers();
                }

                ui.label(format!("Page {} of {}", self.current_page + 1, self.total_pages()));

                if ui.button(format!("{} Next", icons::CARET_RIGHT))
                    .clicked() && (self.current_page + 1) < self.total_pages() {
                    self.current_page += 1;
                    self.refresh_servers();
                }
            });
        }
    }

    /// Render server details (right column)
    fn render_server_details(&self, ui: &mut egui::Ui) {
        ui.heading("Server Details");

        if let Some(idx) = self.selected_server {
            if let Some(wrapper) = self.servers.get(idx) {
                let server = &wrapper.server;
                let is_official = wrapper.meta.official.status == "active";

                ui.add_space(10.0);

                ui.label(egui::RichText::new(&server.name).size(18.0).strong());
                ui.label(&server.description);

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Metadata
                if !server.version.is_empty() {
                    ui.label(format!("{} Version: {}", icons::GIT_BRANCH, server.version));
                }
                if is_official {
                    ui.label(egui::RichText::new(format!("{} Official Anthropic Server", icons::SEAL_CHECK))
                        .color(egui::Color32::from_rgb(60, 150, 60)));
                }

                // Repository link
                if !server.repository.url.is_empty() {
                    ui.hyperlink_to(format!("{} View Repository", icons::LINK), &server.repository.url);
                }

                ui.add_space(10.0);

                // Package information
                if !server.packages.is_empty() {
                    ui.label(egui::RichText::new("Installation Packages:").strong());
                    for package in &server.packages {
                        ui.add_space(5.0);
                        ui.label(format!("{} Type: {}", icons::PACKAGE, package.registry_type));
                        ui.code(&package.identifier);

                        // Environment variables for this package
                        if !package.environment_variables.is_empty() {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("Environment Variables:").strong());
                            for env_var in &package.environment_variables {
                                let secret_marker = if env_var.is_secret { " (secret)" } else { "" };
                                ui.label(format!("• {} = <required>{}", env_var.name, secret_marker));
                                if !env_var.description.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("  {}", env_var.description))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(120, 120, 120))
                                    );
                                }
                            }
                        }
                    }
                }

                // Remote endpoints
                if !server.remotes.is_empty() {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Remote Endpoints:").strong());
                    for remote in &server.remotes {
                        ui.add_space(5.0);
                        ui.label(format!("{} Type: {}", icons::GLOBE, remote.remote_type));
                        ui.code(&remote.url);
                    }
                }

                ui.add_space(20.0);

                // Copy config button
                if ui.button(format!("{} Copy Configuration Snippet", icons::CLIPBOARD_TEXT)).clicked() {
                    let config = self.generate_config_snippet(wrapper);
                    ui.output_mut(|o| o.copied_text = config);
                }

                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Paste this into your mcp_config.json file")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 120, 120))
                );
            }
        } else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} Select a server to view details", icons::ARROW_LEFT))
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 120, 120))
                );
            });
        }
    }

    /// Generate MCP configuration snippet for a server
    ///
    /// Creates a JSON object matching the Rustbot MCP config format.
    fn generate_config_snippet(&self, wrapper: &McpServerWrapper) -> String {
        let server = &wrapper.server;

        // Extract command and args from first package or use remote URL
        let (command, args) = if let Some(package) = server.packages.first() {
            // For OCI packages, use docker
            if package.registry_type == "oci" {
                ("docker".to_string(), vec!["run".to_string(), package.identifier.clone()])
            } else {
                // For other packages, basic npx/uvx/etc
                ("npx".to_string(), vec![package.identifier.clone()])
            }
        } else if let Some(remote) = server.remotes.first() {
            ("mcp-remote".to_string(), vec![remote.url.clone()])
        } else {
            ("echo".to_string(), vec!["Configure this manually".to_string()])
        };

        // Extract environment variables
        let env = server.packages.first()
            .map(|p| p.environment_variables.iter()
                .map(|ev| (ev.name.clone(), "<set_value>".to_string()))
                .collect::<std::collections::HashMap<_, _>>())
            .unwrap_or_default();

        serde_json::to_string_pretty(&serde_json::json!({
            "id": server.name.to_lowercase().replace('/', "-").replace(' ', "-"),
            "name": server.name,
            "description": server.description,
            "command": command,
            "args": args,
            "env": env,
            "enabled": true,
        })).unwrap_or_default()
    }

    /// Get count of servers after filtering
    fn get_filtered_count(&self) -> usize {
        self.servers.iter().filter(|wrapper| {
            let is_official = wrapper.meta.official.status == "active";
            if self.show_official_only && !is_official {
                return false;
            }

            if let Some(ref filter) = self.package_type_filter {
                let package_type = wrapper.server.packages.first()
                    .map(|p| p.registry_type.as_str())
                    .or_else(|| if !wrapper.server.remotes.is_empty() { Some("remote") } else { None })
                    .unwrap_or("unknown");

                if package_type != filter.as_str() {
                    return false;
                }
            }
            true
        }).count()
    }

    /// Calculate total pages
    fn total_pages(&self) -> usize {
        (self.total_servers + self.servers_per_page - 1) / self.servers_per_page
    }
}
