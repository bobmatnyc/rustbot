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
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::mcp::config::McpConfig;
use crate::mcp::extensions::{ExtensionInstaller, ExtensionRegistry, InstalledExtension};
use crate::mcp::marketplace::{MarketplaceClient, McpRegistry, McpServerWrapper};

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

    /// Extension registry for tracking installations
    extension_registry: ExtensionRegistry,

    /// Extension installer
    extension_installer: ExtensionInstaller,

    /// Path to extension registry file
    registry_path: PathBuf,

    /// Path to MCP configuration file
    mcp_config_path: PathBuf,

    /// Installation status message
    install_message: Option<(String, bool)>, // (message, is_error)

    /// Selected agent for installation (None = global config for all agents)
    selected_agent: Option<String>,

    /// Available agent configurations (loaded from agent loader)
    agent_configs: Option<Vec<crate::agent::AgentConfig>>,
}

impl MarketplaceView {
    /// Create a new marketplace view
    ///
    /// # Arguments
    /// * `runtime` - Tokio runtime handle for spawning async tasks
    pub fn new(runtime: Handle) -> Self {
        let (fetch_tx, fetch_rx) = mpsc::unbounded_channel();

        // Setup extension paths
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let extensions_dir = home_dir.join(".rustbot").join("extensions");
        let registry_path = extensions_dir.join("registry.json");
        let install_dir = extensions_dir.join("bin");

        // MCP config path (use ~/.rustbot/ for consistency with registry)
        let mcp_config_path = extensions_dir
            .parent()
            .unwrap_or(&home_dir)
            .join("mcp_config.json");

        // Load or create extension registry
        let extension_registry = ExtensionRegistry::load(&registry_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load extension registry: {}. Creating new.", e);
            ExtensionRegistry::new()
        });

        let extension_installer = ExtensionInstaller::new(install_dir);

        // Load available agent configurations
        let agent_loader = crate::agent::AgentLoader::new();
        let agent_configs = agent_loader.load_all().ok();

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
            servers_per_page: 100, // Use API maximum for better deduplication coverage
            next_cursor: None,
            fetch_rx,
            fetch_tx,
            extension_registry,
            extension_installer,
            registry_path,
            mcp_config_path,
            install_message: None,
            selected_agent: None,
            agent_configs,
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

    /// Deduplicate servers to show only latest stable version of each service
    ///
    /// # Design Decision: Deduplication Strategy
    ///
    /// The MCP Registry API returns all versions of each server, including older versions.
    /// This creates a confusing UX with multiple "filesystem" entries, for example.
    ///
    /// **Strategy**: Use the `is_latest` field from API metadata
    /// - The registry already identifies the latest version via `meta.official.is_latest`
    /// - This is more reliable than custom version parsing (handles pre-releases, etc.)
    /// - Falls back to keeping duplicates if `is_latest` is false for all versions
    ///
    /// **Alternative Considered**: Custom version comparison
    /// - Rejected: Complex edge cases (pre-release versions, different naming schemes)
    /// - Registry API is authoritative source - trust its versioning logic
    ///
    /// # Algorithm
    ///
    /// 1. Group servers by base name (extract name before '@' if present)
    /// 2. For each group, prefer entries where `is_latest == true`
    /// 3. If no `is_latest` entry exists, keep first occurrence
    ///
    /// # Performance
    ///
    /// - Time Complexity: O(n) where n = number of servers
    /// - Space Complexity: O(n) for HashMap
    /// - Typical input: 20-50 servers per page, negligible overhead
    ///
    /// # Example
    ///
    /// Input:
    /// - filesystem@0.5.1 (is_latest: true)
    /// - filesystem@0.5.0 (is_latest: false)
    /// - filesystem@0.4.9 (is_latest: false)
    ///
    /// Output:
    /// - filesystem@0.5.1 (is_latest: true)
    fn deduplicate_servers(servers: Vec<McpServerWrapper>) -> Vec<McpServerWrapper> {
        use std::collections::HashMap;

        // Group by base name (name before '@' symbol)
        let mut latest_versions: HashMap<String, McpServerWrapper> = HashMap::new();

        for server_wrapper in servers {
            let name = &server_wrapper.server.name;

            // Extract base name (e.g., "filesystem@0.5.1" -> "filesystem")
            // Some servers may not have '@' (already base name)
            let base_name = name.split('@').next().unwrap_or(name).to_string();

            match latest_versions.get(&base_name) {
                None => {
                    // First occurrence of this service - keep it
                    latest_versions.insert(base_name, server_wrapper);
                }
                Some(existing) => {
                    // Prefer the one marked as latest by the registry
                    let candidate_is_latest = server_wrapper.meta.official.is_latest;
                    let existing_is_latest = existing.meta.official.is_latest;

                    if candidate_is_latest && !existing_is_latest {
                        // Replace with latest version
                        latest_versions.insert(base_name, server_wrapper);
                    }
                    // If both are latest or both are not latest, keep existing (arbitrary choice)
                    // If existing is latest and candidate is not, keep existing (do nothing)
                }
            }
        }

        // Convert HashMap back to Vec
        latest_versions.into_values().collect()
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
                    // Deduplicate servers to show only latest versions
                    let servers = Self::deduplicate_servers(registry.servers);
                    self.servers = servers;

                    // Store pagination cursor for next page
                    if let Some(metadata) = registry.metadata {
                        self.next_cursor = metadata.next_cursor;
                    } else {
                        self.next_cursor = None;
                    }

                    // FIXED: Use deduplicated count for accurate pagination
                    // Previously used raw API count which showed "20 servers" when only 8 unique existed
                    self.total_servers = self.servers.len();
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
            let changed = ui
                .checkbox(&mut self.show_official_only, "Official only")
                .changed();

            ui.label("Package type:");
            let combo_changed = egui::ComboBox::from_id_source("package_type_filter")
                .selected_text(self.package_type_filter.as_deref().unwrap_or("All"))
                .show_ui(ui, |ui| {
                    let mut changed = false;
                    changed |= ui
                        .selectable_value(&mut self.package_type_filter, None, "All")
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut self.package_type_filter,
                            Some("npm".to_string()),
                            "npm",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut self.package_type_filter,
                            Some("pypi".to_string()),
                            "PyPI",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut self.package_type_filter,
                            Some("docker".to_string()),
                            "Docker",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut self.package_type_filter,
                            Some("remote".to_string()),
                            "Remote",
                        )
                        .changed();
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
            ui.colored_label(
                egui::Color32::RED,
                format!("{} Error", icons::WARNING_CIRCLE),
            );
            ui.label(error);
            if ui.button("Retry").clicked() {
                self.refresh_servers();
            }
            return;
        }

        // Show result count with clarity about deduplication
        let filtered_count = self.get_filtered_count();
        ui.horizontal(|ui| {
            ui.label(format!("Showing {} unique servers", filtered_count));
            ui.label(
                egui::RichText::new("(latest versions only)")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            )
            .on_hover_text("Multiple versions of the same server are deduplicated. Only the latest stable release is shown.");
        });

        if self.total_servers > self.servers_per_page {
            ui.label(format!(
                "Page {} of {}",
                self.current_page + 1,
                self.total_pages()
            ));
        }

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
                    let package_type = server
                        .packages
                        .first()
                        .map(|p| p.registry_type.as_str())
                        .or_else(|| {
                            if !server.remotes.is_empty() {
                                Some("remote")
                            } else {
                                None
                            }
                        })
                        .unwrap_or("unknown");

                    if let Some(ref filter) = self.package_type_filter {
                        if package_type != filter.as_str() {
                            continue;
                        }
                    }

                    // Render server card
                    let is_selected = self.selected_server == Some(idx);

                    let response = ui.add(egui::SelectableLabel::new(is_selected, &server.name));

                    if response.clicked() {
                        self.selected_server = Some(idx);
                    }

                    ui.label(
                        egui::RichText::new(&server.description)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                    );

                    ui.horizontal(|ui| {
                        // Package type badge
                        ui.label(format!("{} {}", icons::PACKAGE, package_type));

                        // Official badge
                        if is_official {
                            ui.label(
                                egui::RichText::new(format!("{} Official", icons::SEAL_CHECK))
                                    .color(egui::Color32::from_rgb(60, 150, 60)),
                            );
                        }

                        // Version
                        if !server.version.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("v{}", server.version))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100)),
                            );
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
                if ui
                    .button(format!("{} Previous", icons::CARET_LEFT))
                    .clicked()
                    && self.current_page > 0
                {
                    self.current_page -= 1;
                    self.refresh_servers();
                }

                ui.label(format!(
                    "Page {} of {}",
                    self.current_page + 1,
                    self.total_pages()
                ));

                if ui.button(format!("{} Next", icons::CARET_RIGHT)).clicked()
                    && (self.current_page + 1) < self.total_pages()
                {
                    self.current_page += 1;
                    self.refresh_servers();
                }
            });
        }
    }

    /// Render server details (right column)
    fn render_server_details(&mut self, ui: &mut egui::Ui) {
        ui.heading("Server Details");

        if let Some(idx) = self.selected_server {
            // Clone wrapper to avoid borrowing issues when calling install_extension
            if let Some(wrapper_ref) = self.servers.get(idx) {
                let wrapper = wrapper_ref.clone();
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
                    ui.label(
                        egui::RichText::new(format!(
                            "{} Official Anthropic Server",
                            icons::SEAL_CHECK
                        ))
                        .color(egui::Color32::from_rgb(60, 150, 60)),
                    );
                }

                // Repository link
                if !server.repository.url.is_empty() {
                    ui.hyperlink_to(
                        format!("{} View Repository", icons::LINK),
                        &server.repository.url,
                    );
                }

                ui.add_space(10.0);

                // Package information
                if !server.packages.is_empty() {
                    ui.label(egui::RichText::new("Installation Packages:").strong());
                    for package in &server.packages {
                        ui.add_space(5.0);
                        ui.label(format!(
                            "{} Type: {}",
                            icons::PACKAGE,
                            package.registry_type
                        ));
                        ui.code(&package.identifier);

                        // Environment variables for this package
                        if !package.environment_variables.is_empty() {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("Environment Variables:").strong());
                            for env_var in &package.environment_variables {
                                let secret_marker =
                                    if env_var.is_secret { " (secret)" } else { "" };
                                ui.label(format!(
                                    "• {} = <required>{}",
                                    env_var.name, secret_marker
                                ));
                                if !env_var.description.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("  {}", env_var.description))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(120, 120, 120)),
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

                // Installation status message
                if let Some((message, is_error)) = &self.install_message {
                    let color = if *is_error {
                        egui::Color32::from_rgb(200, 80, 80)
                    } else {
                        egui::Color32::from_rgb(60, 150, 60)
                    };
                    ui.label(egui::RichText::new(message).color(color));
                    ui.add_space(10.0);
                }

                // Agent selection dropdown for installation target
                ui.horizontal(|ui| {
                    ui.label(format!("{} Install for:", icons::USER));
                    egui::ComboBox::from_id_source("install_target_agent")
                        .selected_text(
                            self.selected_agent
                                .as_ref()
                                .map(|id| {
                                    // Find agent name from config
                                    self.agent_configs
                                        .as_ref()
                                        .and_then(|configs| {
                                            configs
                                                .iter()
                                                .find(|a| &a.id == id)
                                                .map(|a| a.name.as_str())
                                        })
                                        .unwrap_or(id.as_str())
                                })
                                .unwrap_or("All Agents (Global)"),
                        )
                        .show_ui(ui, |ui| {
                            // Global option (default)
                            ui.selectable_value(
                                &mut self.selected_agent,
                                None,
                                "All Agents (Global)",
                            )
                            .on_hover_text("Install for all agents using global mcp_config.json");

                            ui.separator();

                            // List available agents
                            if let Some(ref agent_configs) = self.agent_configs {
                                for agent in agent_configs {
                                    ui.selectable_value(
                                        &mut self.selected_agent,
                                        Some(agent.id.clone()),
                                        &agent.name,
                                    )
                                    .on_hover_text(format!(
                                        "Install for {} agent only (creates agent-specific config)",
                                        agent.name
                                    ));
                                }
                            }
                        });
                });

                ui.add_space(10.0);

                // Install extension button
                let is_installed = self.extension_registry.get(&server.name).is_some();
                let install_button_text = if is_installed {
                    format!("{} Installed", icons::CHECK_CIRCLE)
                } else {
                    format!("{} Install Extension", icons::DOWNLOAD_SIMPLE)
                };

                if ui.button(install_button_text).clicked() {
                    if !is_installed {
                        self.install_extension(&wrapper);
                    }
                }

                ui.add_space(10.0);

                // Copy config button
                if ui
                    .button(format!(
                        "{} Copy Configuration Snippet",
                        icons::CLIPBOARD_TEXT
                    ))
                    .clicked()
                {
                    let config = self.generate_config_snippet(&wrapper);
                    ui.output_mut(|o| o.copied_text = config);
                }

                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Paste this into your mcp_config.json file")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
            }
        } else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} Select a server to view details",
                        icons::ARROW_LEFT
                    ))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(120, 120, 120)),
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
                (
                    "docker".to_string(),
                    vec!["run".to_string(), package.identifier.clone()],
                )
            } else {
                // For other packages, basic npx/uvx/etc
                ("npx".to_string(), vec![package.identifier.clone()])
            }
        } else if let Some(remote) = server.remotes.first() {
            ("mcp-remote".to_string(), vec![remote.url.clone()])
        } else {
            (
                "echo".to_string(),
                vec!["Configure this manually".to_string()],
            )
        };

        // Extract environment variables
        let env = server
            .packages
            .first()
            .map(|p| {
                p.environment_variables
                    .iter()
                    .map(|ev| (ev.name.clone(), "<set_value>".to_string()))
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .unwrap_or_default();

        serde_json::to_string_pretty(&serde_json::json!({
            "id": server.name.to_lowercase().replace('/', "-").replace(' ', "-"),
            "name": server.name,
            "description": server.description,
            "command": command,
            "args": args,
            "env": env,
            "enabled": true,
        }))
        .unwrap_or_default()
    }

    /// Get count of servers after filtering
    fn get_filtered_count(&self) -> usize {
        self.servers
            .iter()
            .filter(|wrapper| {
                let is_official = wrapper.meta.official.status == "active";
                if self.show_official_only && !is_official {
                    return false;
                }

                if let Some(ref filter) = self.package_type_filter {
                    let package_type = wrapper
                        .server
                        .packages
                        .first()
                        .map(|p| p.registry_type.as_str())
                        .or_else(|| {
                            if !wrapper.server.remotes.is_empty() {
                                Some("remote")
                            } else {
                                None
                            }
                        })
                        .unwrap_or("unknown");

                    if package_type != filter.as_str() {
                        return false;
                    }
                }
                true
            })
            .count()
    }

    /// Calculate total pages
    fn total_pages(&self) -> usize {
        (self.total_servers + self.servers_per_page - 1) / self.servers_per_page
    }

    /// Update MCP config for a specific agent
    ///
    /// Creates or updates the agent-specific MCP configuration file with the
    /// newly installed extension. Agent-specific configs are stored in:
    /// ~/.rustbot/mcp_configs/{agent_id}_mcp.json
    ///
    /// This also ensures the agent's JSON config file has the `mcpConfigFile` field set.
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    /// * `extension` - The installed extension to add
    ///
    /// # Returns
    /// Ok(()) if successfully updated, Err if loading, updating, or saving fails
    fn update_agent_mcp_config(
        &self,
        agent_id: &str,
        extension: &InstalledExtension,
    ) -> anyhow::Result<()> {
        // Load or create agent-specific config
        let mut config = McpConfig::load_or_create_for_agent(agent_id)?;

        // Add extension (this replaces if already exists)
        config.add_extension(extension.mcp_config.clone())?;

        // Save updated config
        let config_path = McpConfig::agent_config_path(agent_id)?;
        config.save_to_file(&config_path)?;

        tracing::info!(
            "Updated agent '{}' MCP config with extension: {}",
            agent_id,
            extension.id
        );

        // Ensure agent's JSON config file has mcpConfigFile field
        if let Err(e) = self.ensure_agent_has_mcp_config_file(agent_id) {
            tracing::warn!(
                "Failed to update agent '{}' JSON config with mcpConfigFile: {}",
                agent_id,
                e
            );
            // Don't fail the entire operation, just log the warning
        }

        Ok(())
    }

    /// Ensure agent's JSON config file has mcpConfigFile field set
    ///
    /// Searches for the agent's JSON config file and updates it if needed.
    /// This auto-fixes agent configs during extension installation.
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    ///
    /// # Returns
    /// Ok(()) if config was updated or already correct, Err if agent not found or update fails
    fn ensure_agent_has_mcp_config_file(&self, agent_id: &str) -> anyhow::Result<()> {
        // Find the agent config in our cached list
        let agent = self
            .agent_configs
            .as_ref()
            .and_then(|configs| configs.iter().find(|a| a.id == agent_id))
            .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found in loaded configs", agent_id))?;

        // Check if mcpConfigFile is already set
        if agent.mcp_config_file.is_some() {
            tracing::debug!(
                "Agent '{}' already has mcpConfigFile set: {:?}",
                agent_id,
                agent.mcp_config_file
            );
            return Ok(());
        }

        // Find the agent's JSON file path
        // Try agents/presets first, then agents/custom
        let preset_path = std::path::PathBuf::from("agents")
            .join("presets")
            .join(format!("{}.json", agent_id));
        let custom_path = std::path::PathBuf::from("agents")
            .join("custom")
            .join(format!("{}.json", agent_id));

        let agent_file_path = if preset_path.exists() {
            preset_path
        } else if custom_path.exists() {
            custom_path
        } else {
            anyhow::bail!(
                "Agent config file not found for '{}' (tried {:?} and {:?})",
                agent_id,
                preset_path,
                custom_path
            );
        };

        // Update the agent's JSON file to include mcpConfigFile
        use crate::agent::config::JsonAgentConfig;
        JsonAgentConfig::set_mcp_config_file(&agent_file_path, agent_id)?;

        tracing::info!(
            "Auto-fixed agent '{}' config file to include mcpConfigFile field",
            agent_id
        );

        Ok(())
    }

    /// Update global MCP config (backward compatibility)
    ///
    /// Updates the global mcp_config.json file, which is used by all agents
    /// that don't have agent-specific configurations.
    ///
    /// # Arguments
    /// * `extension` - The installed extension to add to mcp_config.json
    ///
    /// # Returns
    /// Ok(()) if successfully updated, Err if loading, updating, or saving fails
    fn update_global_mcp_config(&self, extension: &InstalledExtension) -> anyhow::Result<()> {
        // Load current config
        let mut config = McpConfig::load_from_file(&self.mcp_config_path)?;

        // Add extension (this replaces if already exists)
        config.add_extension(extension.mcp_config.clone())?;

        // Save updated config
        config.save_to_file(&self.mcp_config_path)?;

        tracing::info!(
            "Updated global mcp_config.json with extension: {}",
            extension.id
        );
        Ok(())
    }

    /// Install an extension from a marketplace server
    ///
    /// Creates an installed extension entry in the registry with MCP configuration.
    /// The extension is disabled by default and requires user configuration (env vars, etc.)
    ///
    /// Supports both agent-specific and global installation based on selected_agent.
    fn install_extension(&mut self, wrapper: &McpServerWrapper) {
        let server = &wrapper.server;

        // Try to install the extension
        match self.extension_installer.install_from_listing(server, None) {
            Ok(extension) => {
                // Clone for later use (after moving into registry)
                let extension_clone = extension.clone();

                // Add to registry
                self.extension_registry.install(extension);

                // Save registry
                match self.extension_registry.save(&self.registry_path) {
                    Ok(_) => {
                        // Update appropriate MCP config based on selected agent
                        let config_result = if let Some(ref agent_id) = self.selected_agent {
                            self.update_agent_mcp_config(agent_id, &extension_clone)
                        } else {
                            self.update_global_mcp_config(&extension_clone)
                        };

                        match config_result {
                            Ok(_) => {
                                let target = self
                                    .selected_agent
                                    .as_deref()
                                    .unwrap_or("all agents (global)");
                                self.install_message = Some((
                                    format!(
                                        "✓ Successfully installed '{}' for {}. Restart to activate.",
                                        server.name, target
                                    ),
                                    false,
                                ));
                                tracing::info!(
                                    "Installed extension '{}' for {}",
                                    server.name,
                                    target
                                );
                            }
                            Err(e) => {
                                self.install_message = Some((
                                    format!(
                                        "⚠ Extension '{}' installed but failed to update config: {}",
                                        server.name, e
                                    ),
                                    true,
                                ));
                                tracing::warn!(
                                    "Extension '{}' installed but config update failed: {}",
                                    server.name,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.install_message =
                            Some((format!("✗ Failed to save registry: {}", e), true));
                        tracing::error!("Failed to save extension registry: {}", e);
                    }
                }
            }
            Err(e) => {
                self.install_message = Some((format!("✗ Installation failed: {}", e), true));
                tracing::error!("Failed to install extension '{}': {}", server.name, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::marketplace::{
        McpServerListing, McpServerWrapper, OfficialMetadata, Repository, ServerMeta,
    };

    /// Helper to create a test server wrapper
    fn create_test_server(name: &str, is_latest: bool) -> McpServerWrapper {
        McpServerWrapper {
            server: McpServerListing {
                schema: None,
                name: name.to_string(),
                description: format!("Test server: {}", name),
                repository: Repository {
                    url: "https://github.com/test/test".to_string(),
                    source: "github".to_string(),
                },
                version: "1.0.0".to_string(),
                packages: vec![],
                remotes: vec![],
            },
            meta: ServerMeta {
                official: OfficialMetadata {
                    status: "active".to_string(),
                    published_at: "2025-01-01T00:00:00Z".to_string(),
                    updated_at: "2025-01-01T00:00:00Z".to_string(),
                    is_latest,
                },
            },
        }
    }

    #[test]
    fn test_deduplicate_servers_keeps_latest_version() {
        // Create test data with duplicate server names
        let servers = vec![
            create_test_server("filesystem@0.5.1", true), // Latest
            create_test_server("filesystem@0.5.0", false),
            create_test_server("filesystem@0.4.9", false),
            create_test_server("sqlite@1.2.3", true), // Latest
            create_test_server("sqlite@1.2.2", false),
        ];

        let deduplicated = MarketplaceView::deduplicate_servers(servers);

        // Should only have 2 servers (one filesystem, one sqlite)
        assert_eq!(deduplicated.len(), 2);

        // Verify we kept the latest versions
        let names: Vec<String> = deduplicated.iter().map(|w| w.server.name.clone()).collect();

        assert!(names.contains(&"filesystem@0.5.1".to_string()));
        assert!(names.contains(&"sqlite@1.2.3".to_string()));
    }

    #[test]
    fn test_deduplicate_servers_preserves_unique_servers() {
        let servers = vec![
            create_test_server("filesystem@0.5.1", true),
            create_test_server("sqlite@1.2.3", true),
            create_test_server("postgres@2.0.0", true),
        ];

        let deduplicated = MarketplaceView::deduplicate_servers(servers.clone());

        // All servers are unique, should keep all
        assert_eq!(deduplicated.len(), 3);
    }

    #[test]
    fn test_deduplicate_servers_handles_no_latest_flag() {
        // Edge case: All versions have is_latest=false
        let servers = vec![
            create_test_server("filesystem@0.5.1", false),
            create_test_server("filesystem@0.5.0", false),
        ];

        let deduplicated = MarketplaceView::deduplicate_servers(servers);

        // Should keep first occurrence when no latest version marked
        assert_eq!(deduplicated.len(), 1);
        assert_eq!(deduplicated[0].server.name, "filesystem@0.5.1");
    }

    #[test]
    fn test_deduplicate_servers_handles_names_without_version() {
        let servers = vec![
            create_test_server("filesystem", true), // No @ symbol
            create_test_server("sqlite@1.2.3", true),
        ];

        let deduplicated = MarketplaceView::deduplicate_servers(servers);

        // Both should be kept (different base names)
        assert_eq!(deduplicated.len(), 2);
    }

    #[test]
    fn test_deduplicate_servers_empty_input() {
        let servers = vec![];
        let deduplicated = MarketplaceView::deduplicate_servers(servers);
        assert_eq!(deduplicated.len(), 0);
    }
}
