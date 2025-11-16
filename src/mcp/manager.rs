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
use tokio::sync::RwLock;

use super::config::McpConfig;
use super::plugin::{PluginMetadata, PluginState, PluginType};
use super::error::{McpError, Result};

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
/// Usage:
///     let manager = McpPluginManager::new(config_path);
///     manager.initialize().await?;
///
///     let plugins = manager.list_plugins().await;
///     manager.enable_plugin("filesystem").await?;
#[derive(Clone)]
pub struct McpPluginManager {
    /// Configuration (shared for hot-reload capability)
    config: Arc<RwLock<McpConfig>>,

    /// Plugin metadata registry
    plugins: Arc<RwLock<HashMap<String, PluginMetadata>>>,
}

impl McpPluginManager {
    /// Create a new plugin manager
    ///
    /// Note: Call initialize() after construction to load config and create plugins
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(McpConfig {
                mcp_plugins: super::config::McpPlugins {
                    local_servers: Vec::new(),
                    cloud_services: Vec::new(),
                },
            })),
            plugins: Arc::new(RwLock::new(HashMap::new())),
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
    /// Phase 1: Stub - returns error
    /// Phase 2: Will spawn process / establish HTTP connection
    ///
    /// Error Conditions:
    /// - Plugin not found: Returns PluginNotFound
    /// - Plugin already running: No-op, returns Ok
    /// - Transport failure: Returns Transport error
    pub async fn start_plugin(&mut self, _id: &str) -> Result<()> {
        Err(McpError::Protocol(
            "Plugin starting not implemented in Phase 1 (foundation only)".to_string()
        ))
    }

    /// Stop a plugin (Phase 2 implementation)
    ///
    /// Phase 1: Stub - returns error
    /// Phase 2: Will terminate process / close HTTP connection
    pub async fn stop_plugin(&mut self, _id: &str) -> Result<()> {
        Err(McpError::Protocol(
            "Plugin stopping not implemented in Phase 1 (foundation only)".to_string()
        ))
    }

    /// Reload configuration from disk (Phase 3 implementation)
    ///
    /// Phase 1: Stub - returns error
    /// Phase 3: Will implement hot-reload of configuration
    pub async fn reload_config(&mut self, _new_config: McpConfig) -> Result<()> {
        Err(McpError::Protocol(
            "Config hot-reload not implemented in Phase 1 (foundation only)".to_string()
        ))
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
    async fn test_phase1_stub_methods() {
        let manager = McpPluginManager::new();

        // Phase 1 stubs should return errors
        let result = manager.clone().start_plugin("test").await;
        assert!(result.is_err());

        let result = manager.clone().stop_plugin("test").await;
        assert!(result.is_err());
    }
}
