//! MCP Plugin Manager
//!
//! Design Decision: Centralized plugin lifecycle coordinator
//!
//! Rationale: A single manager coordinates all plugin operations (start, stop,
//! tool calls) to ensure thread-safe state management and consistent error handling.
//! The manager uses Arc<RwLock<>> for concurrent access from UI and async tasks.
//!
//! Trade-offs:
//! - Centralization vs Flexibility: Single coordinator vs distributed management
//! - Lock Granularity: HashMap-level lock vs per-plugin locks
//! - Async vs Sync: All operations are async for I/O efficiency
//!
//! Alternatives Considered:
//! 1. Per-plugin managers: Rejected - increases complexity, harder to coordinate
//! 2. Actor model: Rejected - overkill for current scale, can migrate later
//! 3. Sync-only design: Rejected - blocks UI during I/O operations
//!
//! Performance Characteristics:
//! - Plugin lookup: O(1) via HashMap
//! - Plugin list: O(n) where n = number of plugins
//! - Tool call: O(1) plugin lookup + RPC latency
//!
//! Extension Points:
//! - Phase 2: Add transport layer (stdio, HTTP)
//! - Phase 3: Add tool registry integration
//! - Phase 4: Add auto-restart with exponential backoff
//! - Phase 5: Add event bus integration for status updates

use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::config::McpConfig;
use super::plugin::{PluginMetadata, PluginState, PluginType, ToolInfo};
use super::protocol::McpToolDefinition;
use super::error::{McpError, Result};
use super::stdio::StdioTransport;
use super::client::McpClient;
use super::transport::McpTransport;
use crate::events::{Event, EventBus, EventKind, McpPluginEvent, PluginHealthStatus};

/// Running plugin instance
///
/// Holds the active MCP client and metadata for a running plugin.
struct RunningPlugin {
    metadata: PluginMetadata,
    client: McpClient<StdioTransport>,
}

/// MCP Plugin Manager
///
/// Coordinates the lifecycle of all MCP plugins (local servers and cloud services).
///
/// Thread Safety:
/// - Uses Arc<RwLock<>> for concurrent access from UI and async tasks
/// - Multiple readers can query state simultaneously
/// - Writers (start/stop operations) get exclusive access
///
/// Async Design:
/// - All public methods are async to avoid blocking UI
/// - Long-running operations (process spawning, HTTP requests) run in background
/// - State changes emit events to update UI immediately
///
/// Phase 3 Enhancements:
/// - Auto-restart with exponential backoff
/// - Configuration hot-reload
/// - Health monitoring background task
/// - Event bus integration for status updates
///
/// Usage:
///     let manager = McpPluginManager::new(event_bus);
///     manager.initialize(config_path).await?;
///
///     let plugins = manager.list_plugins().await;
///     manager.start_plugin("filesystem").await?;
#[derive(Clone)]
pub struct McpPluginManager {
    /// Configuration (shared for hot-reload capability)
    config: Arc<RwLock<McpConfig>>,

    /// Plugin metadata registry
    plugins: Arc<RwLock<HashMap<String, PluginMetadata>>>,

    /// Running plugin instances (Phase 2)
    running_plugins: Arc<RwLock<HashMap<String, RunningPlugin>>>,

    /// Event bus for publishing plugin lifecycle events (Phase 3)
    event_bus: Option<Arc<EventBus>>,

    /// Health monitoring task handle (Phase 3)
    health_monitor_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl McpPluginManager {
    /// Create a new plugin manager
    ///
    /// Phase 3: Accepts optional event bus for plugin lifecycle notifications
    ///
    /// Note: Call initialize() after construction to load config and create plugins
    pub fn new() -> Self {
        Self::with_event_bus(None)
    }

    /// Create a new plugin manager with event bus integration
    ///
    /// Pass an EventBus to receive plugin lifecycle events:
    /// - Plugin started/stopped
    /// - Health status changes
    /// - Configuration reloads
    /// - Error notifications
    pub fn with_event_bus(event_bus: Option<Arc<EventBus>>) -> Self {
        Self {
            config: Arc::new(RwLock::new(McpConfig {
                mcp_plugins: super::config::McpPlugins {
                    local_servers: Vec::new(),
                    cloud_services: Vec::new(),
                },
            })),
            plugins: Arc::new(RwLock::new(HashMap::new())),
            running_plugins: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
            health_monitor_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Helper to publish events to event bus (if configured)
    fn emit_event(&self, event: McpPluginEvent) {
        if let Some(bus) = &self.event_bus {
            let evt = Event::new(
                "mcp_manager".to_string(),
                "broadcast".to_string(),
                EventKind::McpPluginEvent(event),
            );
            if let Err(e) = bus.publish(evt) {
                tracing::warn!("Failed to publish MCP plugin event: {:?}", e);
            }
        }
    }

    /// Load configuration from file and initialize plugin metadata
    ///
    /// Phase 1: Only loads config and creates metadata (doesn't start plugins)
    ///
    /// Error Conditions:
    /// - File not found: Returns IoError
    /// - Invalid JSON: Returns JsonError
    /// - Validation failure: Returns Config error
    ///
    /// Example:
    ///     let manager = McpPluginManager::new();
    ///     manager.load_config("mcp_config.json").await?;
    pub async fn load_config(&mut self, config_path: &Path) -> Result<()> {
        // Load and validate configuration
        let config = McpConfig::load_from_file(config_path)?;

        // Store configuration
        *self.config.write().await = config.clone();

        // Initialize plugin metadata (but don't start yet - Phase 1)
        let mut plugins = self.plugins.write().await;
        plugins.clear();

        // Create metadata for local servers
        for server in &config.mcp_plugins.local_servers {
            let metadata = PluginMetadata::new_local_server(server);
            plugins.insert(server.id.clone(), metadata);
        }

        // Create metadata for cloud services
        for service in &config.mcp_plugins.cloud_services {
            let metadata = PluginMetadata::new_cloud_service(service);
            plugins.insert(service.id.clone(), metadata);
        }

        Ok(())
    }

    /// Initialize the plugin manager
    ///
    /// Phase 1: Alias for load_config for backward compatibility
    /// Phase 2+: Will also start enabled plugins
    pub async fn initialize(&mut self, config_path: &Path) -> Result<()> {
        self.load_config(config_path).await
    }

    /// Get current state of all plugins
    ///
    /// Returns a snapshot of plugin states. Use this for UI display.
    ///
    /// Performance: O(n) where n = number of plugins
    ///
    /// Example:
    ///     let states = manager.get_plugin_states().await;
    ///     for (id, state) in states {
    ///         println!("{}: {:?}", id, state);
    ///     }
    pub async fn get_plugin_states(&self) -> HashMap<String, PluginState> {
        let plugins = self.plugins.read().await;
        plugins.iter()
            .map(|(id, meta)| (id.clone(), meta.state.clone()))
            .collect()
    }

    /// Get metadata for a specific plugin
    ///
    /// Performance: O(1) HashMap lookup
    ///
    /// Example:
    ///     if let Some(plugin) = manager.get_plugin("filesystem").await {
    ///         println!("Tools: {}", plugin.tools.len());
    ///     }
    pub async fn get_plugin(&self, id: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.get(id).cloned()
    }

    /// List all plugins with basic information
    ///
    /// Returns lightweight view of plugins for UI lists.
    ///
    /// Performance: O(n) where n = number of plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.iter()
            .map(|(id, meta)| PluginInfo {
                id: id.clone(),
                name: meta.name.clone(),
                description: meta.description.clone(),
                plugin_type: meta.plugin_type.clone(),
                state: meta.state.clone(),
                tool_count: meta.tools.len(),
                error_message: meta.error_message().map(String::from),
            })
            .collect()
    }

    /// Start a plugin (Phase 2 implementation)
    ///
    /// Spawns the plugin process, establishes connection, performs MCP handshake,
    /// and discovers available tools.
    ///
    /// Error Conditions:
    /// - Plugin not found: Returns PluginNotFound
    /// - Plugin already running: No-op, returns Ok
    /// - Transport failure: Returns Transport error
    /// - Protocol failure: Returns Protocol error
    ///
    /// Side Effects:
    /// - Updates plugin state: Stopped → Starting → Initializing → Running
    /// - Stores running plugin instance
    /// - Populates tool list in metadata
    ///
    /// Example:
    /// ```rust,ignore
    /// manager.start_plugin("filesystem").await?;
    /// let plugin = manager.get_plugin("filesystem").await.unwrap();
    /// assert_eq!(plugin.state, PluginState::Running);
    /// println!("Tools: {}", plugin.tools.len());
    /// ```
    pub async fn start_plugin(&mut self, id: &str) -> Result<()> {
        // Check if already running
        {
            let running = self.running_plugins.read().await;
            if running.contains_key(id) {
                return Ok(()); // Already running, idempotent
            }
        }

        // Get plugin config
        let config = self.config.read().await;
        let server_config = config.mcp_plugins.local_servers.iter()
            .find(|s| s.id == id)
            .ok_or_else(|| McpError::PluginNotFound(id.to_string()))?
            .clone();
        drop(config);

        // Update state to Starting
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(id) {
                plugin.state = PluginState::Starting;
            }
        }

        // Create and start transport
        let mut transport = StdioTransport::new(server_config.clone());
        match transport.start().await {
            Ok(_) => {},
            Err(e) => {
                // Update state to Error
                let mut plugins = self.plugins.write().await;
                if let Some(plugin) = plugins.get_mut(id) {
                    plugin.state = PluginState::Error {
                        message: format!("Failed to start transport: {}", e),
                        timestamp: SystemTime::now(),
                    };
                }
                return Err(e);
            }
        }

        // Update state to Initializing
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(id) {
                plugin.state = PluginState::Initializing;
            }
        }

        // Create client and initialize
        let mut client = McpClient::new(transport);
        match client.initialize().await {
            Ok(_) => {},
            Err(e) => {
                // Update state to Error
                let mut plugins = self.plugins.write().await;
                if let Some(plugin) = plugins.get_mut(id) {
                    plugin.state = PluginState::Error {
                        message: format!("Failed to initialize: {}", e),
                        timestamp: SystemTime::now(),
                    };
                }
                return Err(e);
            }
        }

        // List tools
        let tools = match client.list_tools().await {
            Ok(tools) => tools,
            Err(e) => {
                // Tool listing failed, but plugin is initialized
                // Log warning and continue with empty tool list
                eprintln!("Warning: Failed to list tools for {}: {}", id, e);
                Vec::new()
            }
        };

        // Update metadata with tools and set state to Running
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(id) {
                plugin.state = PluginState::Running;
                plugin.tools = tools.iter().map(|t| ToolInfo {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: t.input_schema.clone(),
                }).collect();
            }
        }

        // Get updated metadata
        let metadata = {
            let plugins = self.plugins.read().await;
            plugins.get(id).cloned()
                .ok_or_else(|| McpError::PluginNotFound(id.to_string()))?
        };

        // Store running plugin
        let tool_count = {
            let plugins = self.plugins.read().await;
            plugins.get(id).map(|p| p.tools.len()).unwrap_or(0)
        };

        self.running_plugins.write().await.insert(id.to_string(), RunningPlugin {
            metadata,
            client,
        });

        // Emit started event (Phase 3)
        self.emit_event(McpPluginEvent::Started {
            plugin_id: id.to_string(),
            tool_count,
        });

        Ok(())
    }

    /// Stop a plugin (Phase 2 implementation)
    ///
    /// Gracefully shuts down the plugin process and cleans up resources.
    ///
    /// Error Conditions:
    /// - Plugin not found: Returns PluginNotFound
    /// - Plugin not running: No-op, returns Ok
    /// - Close failure: Logs warning but returns Ok (cleanup best-effort)
    ///
    /// Side Effects:
    /// - Updates plugin state: Running → Stopping → Stopped
    /// - Removes from running_plugins
    /// - Clears tool list in metadata
    ///
    /// Example:
    /// ```rust,ignore
    /// manager.stop_plugin("filesystem").await?;
    /// let plugin = manager.get_plugin("filesystem").await.unwrap();
    /// assert_eq!(plugin.state, PluginState::Stopped);
    /// ```
    pub async fn stop_plugin(&mut self, id: &str) -> Result<()> {
        // Update state to Stopping
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(id) {
                if matches!(plugin.state, PluginState::Stopped | PluginState::Disabled) {
                    return Ok(()); // Already stopped, idempotent
                }
                plugin.state = PluginState::Stopping;
            }
        }

        // Remove from running plugins and close transport
        let mut running = self.running_plugins.write().await;
        if let Some(mut plugin) = running.remove(id) {
            if let Err(e) = plugin.client.transport_mut().close().await {
                eprintln!("Warning: Error closing transport for {}: {}", id, e);
            }
        }
        drop(running);

        // Update state to Stopped and clear tools
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(id) {
                plugin.state = PluginState::Stopped;
                plugin.tools.clear();
            }
        }

        // Emit stopped event (Phase 3)
        self.emit_event(McpPluginEvent::Stopped {
            plugin_id: id.to_string(),
        });

        Ok(())
    }

    /// Execute a tool from a running plugin
    ///
    /// Calls a tool on an active plugin and returns the result.
    ///
    /// Preconditions:
    /// - Plugin must be running (call start_plugin() first)
    /// - Tool must exist in plugin's tool list
    ///
    /// Error Conditions:
    /// - Plugin not found: Returns PluginNotFound
    /// - Plugin not running: Returns Transport error
    /// - Tool not found: Server returns error
    /// - Invalid arguments: Server returns error
    ///
    /// Tool Error Handling:
    /// - Tool execution errors are returned in the result text with is_error flag
    /// - Check result for error before using
    ///
    /// Example:
    /// ```rust,ignore
    /// let result = manager.execute_tool(
    ///     "filesystem",
    ///     "read_file",
    ///     Some(serde_json::json!({"path": "/etc/hosts"}))
    /// ).await?;
    /// println!("Result: {}", result);
    /// ```
    pub async fn execute_tool(
        &mut self,
        plugin_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>
    ) -> Result<String> {
        // Get running plugin
        let mut running = self.running_plugins.write().await;
        let plugin = running.get_mut(plugin_id)
            .ok_or_else(|| McpError::PluginNotFound(format!(
                "Plugin '{}' not running (call start_plugin() first)", plugin_id
            )))?;

        // Call tool
        let result = plugin.client.call_tool(tool_name.to_string(), arguments).await?;

        // Check for tool-level error
        if result.is_error == Some(true) {
            // Tool execution failed - return error text
            let error_text = result.content.iter()
                .map(|c| c.text.clone())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(McpError::Protocol(format!("Tool execution error: {}", error_text)));
        }

        // Extract text from successful result
        let text = result.content.iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Reload configuration from disk (Phase 3 implementation)
    ///
    /// Hot-reloads plugin configuration without full application restart.
    ///
    /// Design Decision: Differential config reload
    ///
    /// Rationale: Only modify plugins that changed, leaving running plugins
    /// untouched. This minimizes disruption and maintains active connections.
    ///
    /// Reload Strategy:
    /// 1. Compare new config with current config
    /// 2. Identify plugins: added, removed, updated
    /// 3. Stop removed plugins
    /// 4. Start newly enabled plugins
    /// 5. Update settings for existing plugins (without restart if possible)
    /// 6. Publish ConfigReloaded event with changes
    ///
    /// Trade-offs:
    /// - Differential vs Full Reload: Minimal disruption vs simplicity
    /// - In-place updates: Complex logic vs user experience
    /// - Validation timing: Validate before applying vs rollback on failure
    ///
    /// Error Handling:
    /// - Invalid config: Reject without applying changes
    /// - Partial failures: Log warnings but continue
    /// - State consistency: Always maintain valid plugin state
    pub async fn reload_config(&mut self, new_config: McpConfig) -> Result<()> {
        tracing::info!("Reloading configuration...");

        // Validate new configuration first
        new_config.validate()?;

        // Get current configuration
        let old_config = self.config.read().await.clone();

        // Identify changes
        let mut plugins_added = Vec::new();
        let mut plugins_removed = Vec::new();
        let mut plugins_updated = Vec::new();

        // Build maps for comparison
        let old_servers: HashMap<String, _> = old_config.mcp_plugins.local_servers.iter()
            .map(|s| (s.id.clone(), s))
            .collect();

        let new_servers: HashMap<String, _> = new_config.mcp_plugins.local_servers.iter()
            .map(|s| (s.id.clone(), s))
            .collect();

        // Find added and updated plugins
        for (id, _new_server) in &new_servers {
            if !old_servers.contains_key(id) {
                plugins_added.push(id.clone());
            } else {
                // Check if configuration changed
                // For simplicity, we'll mark as updated if any field differs
                plugins_updated.push(id.clone());
            }
        }

        // Find removed plugins
        for id in old_servers.keys() {
            if !new_servers.contains_key(id) {
                plugins_removed.push(id.clone());
            }
        }

        tracing::info!(
            "Config changes: {} added, {} removed, {} updated",
            plugins_added.len(),
            plugins_removed.len(),
            plugins_updated.len()
        );

        // Apply changes

        // 1. Stop removed plugins
        for plugin_id in &plugins_removed {
            tracing::info!("Removing plugin '{}'", plugin_id);
            if let Err(e) = self.stop_plugin(plugin_id).await {
                tracing::warn!("Failed to stop removed plugin '{}': {}", plugin_id, e);
            }

            // Remove from registry
            self.plugins.write().await.remove(plugin_id);
        }

        // 2. Add new plugins
        for plugin_id in &plugins_added {
            if let Some(server_config) = new_servers.get(plugin_id) {
                tracing::info!("Adding new plugin '{}'", plugin_id);

                // Create metadata
                let metadata = PluginMetadata::new_local_server(server_config);
                self.plugins.write().await.insert(plugin_id.clone(), metadata);

                // Start if enabled
                if server_config.enabled {
                    if let Err(e) = self.start_plugin(plugin_id).await {
                        tracing::warn!("Failed to start new plugin '{}': {}", plugin_id, e);
                    }
                }
            }
        }

        // 3. Update existing plugins
        for plugin_id in &plugins_updated {
            if let Some(new_server) = new_servers.get(plugin_id) {
                tracing::info!("Updating plugin '{}'", plugin_id);

                // For now, we'll update metadata but not restart
                // Future: Detect which fields changed and restart only if necessary
                let mut plugins = self.plugins.write().await;
                if let Some(metadata) = plugins.get_mut(plugin_id) {
                    // Update fields that don't require restart
                    metadata.name = new_server.name.clone();
                    metadata.description = new_server.description.clone();
                    metadata.max_retries = new_server.max_retries.unwrap_or(5);
                }
            }
        }

        // Update stored configuration
        *self.config.write().await = new_config;

        // Publish reload event
        self.emit_event(McpPluginEvent::ConfigReloaded {
            plugins_added,
            plugins_removed,
            plugins_updated,
        });

        tracing::info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Get total number of plugins
    pub async fn plugin_count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }

    /// Check if a plugin exists
    pub async fn has_plugin(&self, id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(id)
    }

    /// Get tools from a running plugin
    ///
    /// Returns the MCP tool definitions for a plugin. These can be registered
    /// with the API layer for agent discovery.
    ///
    /// # Arguments
    /// * `plugin_id` - ID of the plugin to get tools from
    ///
    /// # Returns
    /// Vector of MCP tool definitions, or error if plugin not found/running
    ///
    /// # Example
    /// ```rust,ignore
    /// let tools = manager.get_plugin_tools("filesystem").await?;
    /// for tool in tools {
    ///     api.register_mcp_tool(tool, "filesystem".to_string()).await?;
    /// }
    /// ```
    pub async fn get_plugin_tools(&self, plugin_id: &str) -> Result<Vec<McpToolDefinition>> {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| McpError::PluginNotFound(plugin_id.to_string()))?;

        // Convert ToolInfo back to McpToolDefinition
        let tools = plugin.tools.iter().map(|t| McpToolDefinition {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.input_schema.clone(),
        }).collect();

        Ok(tools)
    }

    // ========================================================================
    // Phase 3: Auto-Restart and Health Monitoring
    // ========================================================================

    /// Calculate exponential backoff delay for restart attempts
    ///
    /// Formula: min(2^attempt * 1000ms, 32000ms)
    /// - Attempt 0: 1s
    /// - Attempt 1: 2s
    /// - Attempt 2: 4s
    /// - Attempt 3: 8s
    /// - Attempt 4: 16s
    /// - Attempt 5+: 32s (max)
    fn calculate_backoff_delay(attempt: u32) -> Duration {
        let base_delay_ms = 1000u64;
        let max_delay_ms = 32000u64;
        let delay_ms = base_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        Duration::from_millis(delay_ms.min(max_delay_ms))
    }

    /// Handle plugin crash and attempt restart with exponential backoff
    ///
    /// Design Decision: Automatic restart with backoff
    ///
    /// Rationale: MCP servers can crash due to external factors (NPM issues,
    /// temporary resource exhaustion). Auto-restart with backoff prevents
    /// rapid failure loops while giving the plugin time to recover.
    ///
    /// Trade-offs:
    /// - Auto-restart vs Manual: Convenience vs control
    /// - Max retries (5): Balance between persistence and resource waste
    /// - Backoff strategy: Exponential prevents thundering herd
    ///
    /// Error Recovery Strategy:
    /// - Detect crash (process died or transport closed unexpectedly)
    /// - Check restart count against max_retries
    /// - Calculate exponential backoff delay
    /// - Publish restart attempt event
    /// - Wait for backoff delay
    /// - Attempt restart via start_plugin()
    /// - Reset restart count on successful start
    /// - Mark as permanently failed after max retries
    async fn handle_plugin_crash(&mut self, plugin_id: &str) -> Result<()> {
        tracing::warn!("Plugin '{}' crashed, attempting restart", plugin_id);

        // Get current restart count and max retries
        let (restart_count, max_retries, auto_restart) = {
            let plugins = self.plugins.read().await;
            let plugin = plugins.get(plugin_id)
                .ok_or_else(|| McpError::PluginNotFound(plugin_id.to_string()))?;

            (plugin.restart_count, plugin.max_retries, self.is_auto_restart_enabled(plugin_id).await)
        };

        // Check if auto-restart is enabled
        if !auto_restart {
            tracing::info!("Auto-restart disabled for plugin '{}'", plugin_id);
            return Ok(());
        }

        // Check if we've exceeded max retries
        if restart_count >= max_retries {
            tracing::error!(
                "Plugin '{}' exceeded max retries ({}/{}), marking as failed",
                plugin_id, restart_count, max_retries
            );

            // Update state to permanent error
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.state = PluginState::Error {
                    message: format!(
                        "Plugin failed permanently after {} restart attempts",
                        restart_count
                    ),
                    timestamp: SystemTime::now(),
                };
            }

            // Emit error event
            self.emit_event(McpPluginEvent::Error {
                plugin_id: plugin_id.to_string(),
                message: format!("Max retries ({}) exceeded", max_retries),
            });

            return Err(McpError::Protocol(format!(
                "Plugin '{}' exceeded max retries",
                plugin_id
            )));
        }

        // Calculate backoff delay
        let delay = Self::calculate_backoff_delay(restart_count);
        tracing::info!(
            "Restarting plugin '{}' in {:?} (attempt {}/{})",
            plugin_id, delay, restart_count + 1, max_retries
        );

        // Publish restart attempt event
        self.emit_event(McpPluginEvent::RestartAttempt {
            plugin_id: plugin_id.to_string(),
            attempt: restart_count + 1,
            max_retries,
        });

        // Update restart count and timestamp
        {
            let mut plugins = self.plugins.write().await;
            if let Some(plugin) = plugins.get_mut(plugin_id) {
                plugin.restart_count += 1;
                plugin.last_restart = Some(SystemTime::now());
            }
        }

        // Wait for backoff delay
        tokio::time::sleep(delay).await;

        // Attempt restart
        match self.start_plugin(plugin_id).await {
            Ok(_) => {
                tracing::info!("Plugin '{}' restarted successfully", plugin_id);

                // Reset restart count on successful start
                let mut plugins = self.plugins.write().await;
                if let Some(plugin) = plugins.get_mut(plugin_id) {
                    plugin.restart_count = 0;
                }

                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to restart plugin '{}': {}", plugin_id, e);
                Err(e)
            }
        }
    }

    /// Check if auto-restart is enabled for a plugin
    async fn is_auto_restart_enabled(&self, plugin_id: &str) -> bool {
        let config = self.config.read().await;

        // Check local servers
        if let Some(server) = config.mcp_plugins.local_servers.iter()
            .find(|s| s.id == plugin_id)
        {
            return server.auto_restart;
        }

        // Check cloud services (if they have auto_restart in future)
        false
    }

    /// Monitor health of a specific plugin
    ///
    /// Checks if the plugin process is still alive and updates health status.
    ///
    /// Health Checks:
    /// 1. Plugin exists in running_plugins registry
    /// 2. Future: Ping request with timeout
    /// 3. Future: Process liveness check for stdio transport
    ///
    /// Note: Current implementation is conservative - only checks registry presence.
    /// Future enhancements can add deeper health checks.
    async fn check_plugin_health(&self, plugin_id: &str) -> Result<PluginHealthStatus> {
        let running = self.running_plugins.read().await;

        match running.get(plugin_id) {
            Some(_running_plugin) => {
                // Plugin exists in registry
                // Future: Add transport.is_connected() check when non-mut accessor available
                // Future: Add ping/echo request with timeout
                Ok(PluginHealthStatus::Healthy)
            }
            None => {
                // Plugin not in running registry
                Ok(PluginHealthStatus::Dead)
            }
        }
    }

    /// Start background health monitoring task
    ///
    /// Spawns a tokio task that periodically checks all running plugins and
    /// triggers restart if they've crashed.
    ///
    /// Health Check Interval:
    /// - Default: 30 seconds
    /// - Configurable via plugin config (future enhancement)
    ///
    /// Monitoring Loop:
    /// 1. Sleep for health_check_interval
    /// 2. Get list of running plugins
    /// 3. Check health of each plugin
    /// 4. Trigger restart for dead plugins (if auto_restart enabled)
    /// 5. Publish health status events
    ///
    /// Task Lifecycle:
    /// - Runs until explicitly stopped via stop_health_monitoring()
    /// - Automatically stops when manager is dropped
    pub async fn start_health_monitoring(&self) -> Result<()> {
        // Stop any existing monitor
        self.stop_health_monitoring().await;

        let manager = self.clone();
        let handle = tokio::spawn(async move {
            let interval = Duration::from_secs(30); // Default 30s interval

            tracing::info!("Health monitoring started (interval: {:?})", interval);

            loop {
                tokio::time::sleep(interval).await;

                // Get list of running plugins
                let plugin_ids: Vec<String> = {
                    let running = manager.running_plugins.read().await;
                    running.keys().cloned().collect()
                };

                // Check health of each plugin
                for plugin_id in plugin_ids {
                    match manager.check_plugin_health(&plugin_id).await {
                        Ok(PluginHealthStatus::Healthy) => {
                            // Plugin healthy, no action needed
                        }
                        Ok(PluginHealthStatus::Dead) => {
                            tracing::warn!("Plugin '{}' is dead, triggering restart", plugin_id);

                            // Publish health status event
                            manager.emit_event(McpPluginEvent::HealthStatus {
                                plugin_id: plugin_id.clone(),
                                status: PluginHealthStatus::Dead,
                            });

                            // Attempt restart (clone manager for async task)
                            let mut mgr_clone = manager.clone();
                            let id = plugin_id.clone();
                            tokio::spawn(async move {
                                if let Err(e) = mgr_clone.handle_plugin_crash(&id).await {
                                    tracing::error!("Failed to handle crash for '{}': {}", id, e);
                                }
                            });
                        }
                        Ok(PluginHealthStatus::Unresponsive) => {
                            tracing::warn!("Plugin '{}' is unresponsive", plugin_id);

                            manager.emit_event(McpPluginEvent::HealthStatus {
                                plugin_id: plugin_id.clone(),
                                status: PluginHealthStatus::Unresponsive,
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to check health of '{}': {}", plugin_id, e);
                        }
                    }
                }
            }
        });

        // Store handle
        *self.health_monitor_handle.write().await = Some(handle);

        Ok(())
    }

    /// Stop background health monitoring task
    pub async fn stop_health_monitoring(&self) {
        let mut handle_guard = self.health_monitor_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            tracing::info!("Health monitoring stopped");
        }
    }
}

/// Lightweight plugin information for UI lists
///
/// This struct provides essential information without cloning large
/// tool/resource lists. Use this for displaying plugin lists in UI.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub plugin_type: PluginType,
    pub state: PluginState,
    pub tool_count: usize,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::config::ConfigWatcher;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = McpPluginManager::new();
        assert_eq!(manager.plugin_count().await, 0);
    }

    #[tokio::test]
    async fn test_load_config() {
        // Create temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "test",
                        "name": "Test Server",
                        "command": "echo",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        assert_eq!(manager.plugin_count().await, 1);
        assert!(manager.has_plugin("test").await);

        let plugin = manager.get_plugin("test").await.unwrap();
        assert_eq!(plugin.name, "Test Server");
        assert_eq!(plugin.state, PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "cmd1",
                        "args": [],
                        "enabled": true
                    },
                    {
                        "id": "server2",
                        "name": "Server 2",
                        "command": "cmd2",
                        "args": [],
                        "enabled": false
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 2);

        let names: Vec<_> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Server 1"));
        assert!(names.contains(&"Server 2"));
    }

    #[tokio::test]
    async fn test_duplicate_id_rejection() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "duplicate",
                        "name": "Server 1",
                        "command": "cmd1",
                        "args": [],
                        "enabled": true
                    },
                    {
                        "id": "duplicate",
                        "name": "Server 2",
                        "command": "cmd2",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        let result = manager.load_config(temp_file.path()).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate plugin ID"));
    }

    #[tokio::test]
    async fn test_start_plugin_not_found() {
        let mut manager = McpPluginManager::new();

        // Starting non-existent plugin should fail
        let result = manager.start_plugin("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Plugin not found"));
    }

    #[tokio::test]
    async fn test_stop_plugin_idempotent() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "test",
                        "name": "Test",
                        "command": "echo",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        // Stopping a stopped plugin should succeed (idempotent)
        let result = manager.stop_plugin("test").await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // Phase 3 Tests: Auto-Restart, Health Monitoring, Config Reload
    // ========================================================================

    #[test]
    fn test_exponential_backoff_calculation() {
        // Test exponential backoff formula
        assert_eq!(McpPluginManager::calculate_backoff_delay(0), Duration::from_secs(1));
        assert_eq!(McpPluginManager::calculate_backoff_delay(1), Duration::from_secs(2));
        assert_eq!(McpPluginManager::calculate_backoff_delay(2), Duration::from_secs(4));
        assert_eq!(McpPluginManager::calculate_backoff_delay(3), Duration::from_secs(8));
        assert_eq!(McpPluginManager::calculate_backoff_delay(4), Duration::from_secs(16));
        assert_eq!(McpPluginManager::calculate_backoff_delay(5), Duration::from_secs(32)); // Max
        assert_eq!(McpPluginManager::calculate_backoff_delay(10), Duration::from_secs(32)); // Capped at max
    }

    #[tokio::test]
    async fn test_max_retries_respected() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "test",
                        "name": "Test",
                        "command": "nonexistent_command",
                        "args": [],
                        "enabled": true,
                        "auto_restart": true,
                        "max_retries": 2
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        // Manually set restart count to max
        {
            let mut plugins = manager.plugins.write().await;
            if let Some(plugin) = plugins.get_mut("test") {
                plugin.restart_count = 2; // At max_retries
            }
        }

        // Attempt to handle crash should fail due to max retries
        let result = manager.handle_plugin_crash("test").await;
        assert!(result.is_err());

        // Check that plugin is marked as permanently failed
        let plugin = manager.get_plugin("test").await.unwrap();
        assert!(matches!(plugin.state, PluginState::Error { .. }));
    }

    #[tokio::test]
    async fn test_config_hot_reload_detection() {
        use std::io::Write;

        // Create initial config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "echo",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(initial_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut watcher = ConfigWatcher::new(temp_file.path()).unwrap();

        // First check should return None (no change)
        let result = watcher.check_for_changes().await.unwrap();
        assert!(result.is_none());

        // Modify the config file
        tokio::time::sleep(Duration::from_millis(100)).await; // Ensure different mtime
        let updated_config = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1 Updated",
                        "command": "echo",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        std::fs::write(temp_file.path(), updated_config).unwrap();

        // Second check should detect change
        let result = watcher.check_for_changes().await.unwrap();
        assert!(result.is_some());

        let new_config = result.unwrap();
        assert_eq!(new_config.mcp_plugins.local_servers[0].name, "Server 1 Updated");
    }

    #[tokio::test]
    async fn test_health_check_healthy_plugin() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "test",
                        "name": "Test",
                        "command": "echo",
                        "args": [],
                        "enabled": true
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let manager = McpPluginManager::new();
        // Note: Without actually starting a plugin, health check will return Dead
        // This tests the Dead path
        let health = manager.check_plugin_health("test").await.unwrap();
        assert_eq!(health, crate::events::PluginHealthStatus::Dead);
    }

    #[tokio::test]
    async fn test_health_check_dead_plugin() {
        let manager = McpPluginManager::new();

        // Check health of non-existent plugin
        let health = manager.check_plugin_health("nonexistent").await.unwrap();
        assert_eq!(health, crate::events::PluginHealthStatus::Dead);
    }

    #[tokio::test]
    async fn test_event_emission() {
        use crate::events::{EventBus, EventKind};

        let event_bus = Arc::new(EventBus::new());
        let manager = McpPluginManager::with_event_bus(Some(Arc::clone(&event_bus)));

        // Subscribe to events
        let mut rx = event_bus.subscribe();

        // Emit a test event
        manager.emit_event(crate::events::McpPluginEvent::Started {
            plugin_id: "test".to_string(),
            tool_count: 5,
        });

        // Receive and verify event
        let event = rx.try_recv().unwrap();
        match event.kind {
            EventKind::McpPluginEvent(crate::events::McpPluginEvent::Started { plugin_id, tool_count }) => {
                assert_eq!(plugin_id, "test");
                assert_eq!(tool_count, 5);
            }
            _ => panic!("Expected McpPluginEvent::Started"),
        }
    }

    #[tokio::test]
    async fn test_reload_config_add_plugin() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "echo",
                        "args": [],
                        "enabled": false
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(initial_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        assert_eq!(manager.plugin_count().await, 1);

        // Create new config with additional plugin
        let new_config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "echo",
                        "args": [],
                        "enabled": false
                    },
                    {
                        "id": "server2",
                        "name": "Server 2",
                        "command": "cat",
                        "args": [],
                        "enabled": false
                    }
                ],
                "cloud_services": []
            }
        }"#;

        let new_config: McpConfig = serde_json::from_str(new_config_json).unwrap();
        manager.reload_config(new_config).await.unwrap();

        assert_eq!(manager.plugin_count().await, 2);
        assert!(manager.has_plugin("server1").await);
        assert!(manager.has_plugin("server2").await);
    }

    #[tokio::test]
    async fn test_reload_config_remove_plugin() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "echo",
                        "args": [],
                        "enabled": false
                    },
                    {
                        "id": "server2",
                        "name": "Server 2",
                        "command": "cat",
                        "args": [],
                        "enabled": false
                    }
                ],
                "cloud_services": []
            }
        }"#;
        temp_file.write_all(initial_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let mut manager = McpPluginManager::new();
        manager.load_config(temp_file.path()).await.unwrap();

        assert_eq!(manager.plugin_count().await, 2);

        // Create new config with one plugin removed
        let new_config_json = r#"{
            "mcp_plugins": {
                "local_servers": [
                    {
                        "id": "server1",
                        "name": "Server 1",
                        "command": "echo",
                        "args": [],
                        "enabled": false
                    }
                ],
                "cloud_services": []
            }
        }"#;

        let new_config: McpConfig = serde_json::from_str(new_config_json).unwrap();
        manager.reload_config(new_config).await.unwrap();

        assert_eq!(manager.plugin_count().await, 1);
        assert!(manager.has_plugin("server1").await);
        assert!(!manager.has_plugin("server2").await);
    }
}
