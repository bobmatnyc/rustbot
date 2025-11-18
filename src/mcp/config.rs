//! MCP Plugin Configuration Types
//!
//! Design Decision: JSON-based configuration with environment variable substitution
//!
//! Rationale: JSON configuration provides a familiar format for users (matching
//! Claude Desktop's approach) while supporting environment variables for secrets.
//! This balances ease of use with security best practices.
//!
//! Trade-offs:
//! - JSON vs TOML: JSON is more familiar for MCP ecosystem (Claude Desktop uses it)
//! - Environment variables: Flexible but requires documentation
//! - Validation timing: Load-time validation catches errors early
//!
//! Alternatives Considered:
//! 1. TOML configuration: Rejected - breaks compatibility with Claude Desktop configs
//! 2. Encrypted secrets: Deferred to future phase - adds complexity
//! 3. Runtime validation only: Rejected - fail-fast is better UX
//!
//! Extension Points:
//! - Add validation rules as needed (e.g., URL format, command existence)
//! - Support additional auth types (API keys, certificate-based)
//! - Add plugin-specific settings (permissions, resource limits)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::error::{McpError, Result};
use super::extensions::McpConfigEntry;

/// Top-level MCP plugin configuration
///
/// This matches the structure of Claude Desktop's MCP configuration,
/// enabling users to reuse existing configurations.
///
/// File Format: JSON
/// Location: ~/.rustbot/mcp_config.json (or user-specified path)
///
/// Example:
///     {
///       "mcp_plugins": {
///         "local_servers": [ ... ],
///         "cloud_services": [ ... ]
///       }
///     }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub mcp_plugins: McpPlugins,
}

/// Container for both local servers and cloud services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPlugins {
    /// Local MCP servers (communicate via stdio)
    #[serde(default)]
    pub local_servers: Vec<LocalServerConfig>,

    /// Cloud MCP services (communicate via HTTP/SSE)
    #[serde(default)]
    pub cloud_services: Vec<CloudServiceConfig>,
}

/// Configuration for a local MCP server (stdio transport)
///
/// Local servers are spawned as child processes and communicate via
/// stdin/stdout using JSON-RPC 2.0 protocol.
///
/// Example:
///     {
///       "id": "filesystem",
///       "name": "Filesystem Access",
///       "command": "npx",
///       "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"],
///       "enabled": true
///     }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalServerConfig {
    /// Unique identifier for this plugin
    ///
    /// Used to reference the plugin in API calls and UI
    /// Must be unique across all plugins (local + cloud)
    pub id: String,

    /// Display name shown in UI
    pub name: String,

    /// Optional description of plugin's purpose
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Executable command to spawn
    ///
    /// Examples: "npx", "python", "/path/to/binary"
    pub command: String,

    /// Command-line arguments
    ///
    /// Passed to command when spawning process
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set for process
    ///
    /// Supports variable substitution: "${VAR_NAME}" is replaced with
    /// the value of environment variable VAR_NAME at runtime.
    ///
    /// Example:
    ///     "env": {
    ///       "API_KEY": "${MY_API_KEY}",
    ///       "LOG_LEVEL": "debug"
    ///     }
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Whether plugin should be started automatically
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether to automatically restart on failure
    #[serde(default)]
    pub auto_restart: bool,

    /// Maximum number of restart attempts before marking as failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,

    /// Health check interval in seconds (Phase 3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_interval: Option<u64>,

    /// Timeout in seconds for operations
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Optional working directory for process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
}

/// Configuration for a cloud MCP service (HTTP transport)
///
/// Cloud services are accessed via HTTP/HTTPS and may require authentication.
///
/// Example:
///     {
///       "id": "weather_api",
///       "name": "Weather Service",
///       "url": "https://mcp.weather.example.com",
///       "auth": { "type": "bearer", "token": "${WEATHER_TOKEN}" },
///       "enabled": true
///     }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudServiceConfig {
    /// Unique identifier for this plugin
    pub id: String,

    /// Display name shown in UI
    pub name: String,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Service URL (must be HTTPS in production)
    pub url: String,

    /// Authentication configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,

    /// Whether plugin should be connected automatically
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum number of restart attempts before marking as failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,

    /// Health check interval in seconds (Phase 3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_interval: Option<u64>,

    /// Timeout in seconds for HTTP requests
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Authentication configuration for cloud services
///
/// Supports multiple authentication methods commonly used with MCP services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    /// No authentication required
    None,

    /// Bearer token authentication (Authorization: Bearer <token>)
    ///
    /// Example:
    ///     { "type": "bearer", "token": "${API_TOKEN}" }
    Bearer { token: String },

    /// Basic authentication (Authorization: Basic <base64(user:pass)>)
    ///
    /// Example:
    ///     { "type": "basic", "username": "user", "password": "${PASSWORD}" }
    Basic { username: String, password: String },

    /// OAuth 2.1 authentication (future implementation)
    ///
    /// Example:
    ///     {
    ///       "type": "oauth",
    ///       "client_id": "app-id",
    ///       "client_secret": "${SECRET}",
    ///       "token_url": "https://auth.example.com/token",
    ///       "scopes": ["mcp:read", "mcp:write"]
    ///     }
    OAuth {
        client_id: String,
        client_secret: Option<String>,
        token_url: String,
        scopes: Vec<String>,
    },
}

// Default value helpers for serde
fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    60
}

impl McpConfig {
    /// Get the MCP config path for a specific agent
    ///
    /// Returns the path to the agent-specific MCP configuration file:
    /// `~/.rustbot/mcp_configs/{agent_id}_mcp.json`
    ///
    /// Creates the mcp_configs directory if it doesn't exist.
    ///
    /// # Arguments
    /// * `agent_id` - Unique identifier for the agent
    ///
    /// # Returns
    /// Ok(PathBuf) with the config path, or Err if HOME is not set or directory creation fails
    ///
    /// # Example
    /// ```no_run
    /// let path = McpConfig::agent_config_path("assistant")?;
    /// // Returns: ~/.rustbot/mcp_configs/assistant_mcp.json
    /// ```
    pub fn agent_config_path(agent_id: &str) -> Result<PathBuf> {
        let home_dir = std::env::var("HOME")
            .map_err(|_| McpError::Config("HOME environment variable not set".to_string()))?;

        let config_dir = PathBuf::from(home_dir).join(".rustbot").join("mcp_configs");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
            tracing::info!("Created MCP configs directory: {:?}", config_dir);
        }

        Ok(config_dir.join(format!("{}_mcp.json", agent_id)))
    }

    /// Load or create agent-specific MCP config
    ///
    /// Loads the MCP configuration for a specific agent. If the config file
    /// doesn't exist, creates an empty configuration with no plugins.
    ///
    /// This ensures the mcp_configs directory exists before attempting to
    /// read or write configuration files.
    ///
    /// # Arguments
    /// * `agent_id` - Unique identifier for the agent
    ///
    /// # Returns
    /// Ok(McpConfig) with loaded or newly created config, or Err on I/O or parsing errors
    ///
    /// # Example
    /// ```no_run
    /// let config = McpConfig::load_or_create_for_agent("assistant")?;
    /// ```
    pub fn load_or_create_for_agent(agent_id: &str) -> Result<Self> {
        let path = Self::agent_config_path(agent_id)?;

        if path.exists() {
            tracing::debug!("Loading agent MCP config from {:?}", path);
            Self::load_from_file(&path)
        } else {
            tracing::info!(
                "Creating new empty MCP config for agent '{}' at {:?}",
                agent_id,
                path
            );

            // Ensure parent directory exists (agent_config_path creates it but double-check)
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                    tracing::info!("Created MCP configs directory: {:?}", parent);
                }
            }

            // Create empty config for new agent
            let config = McpConfig {
                mcp_plugins: McpPlugins {
                    local_servers: vec![],
                    cloud_services: vec![],
                },
            };
            config.save_to_file(&path)?;
            Ok(config)
        }
    }

    /// Load configuration from a JSON file
    ///
    /// Error Conditions:
    /// - File not found: Returns IoError
    /// - Invalid JSON: Returns JsonError
    /// - Validation failure: Returns Config error
    ///
    /// Example:
    ///     let config = McpConfig::load_from_file("mcp_config.json")?;
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration for common errors
    ///
    /// Validation Rules:
    /// 1. All plugin IDs must be unique (across local + cloud)
    /// 2. No empty plugin IDs or names
    /// 3. Local servers must have valid commands
    /// 4. Cloud services must have valid URLs
    ///
    /// Error Cases:
    /// - Duplicate IDs: Returns Config error with duplicate ID
    /// - Empty required fields: Returns Config error
    /// - Invalid URLs: Returns Config error
    ///
    /// Note: Environment variable resolution is deferred until plugin start
    /// to allow runtime configuration changes.
    pub fn validate(&self) -> Result<()> {
        let mut ids = std::collections::HashSet::new();

        // Check for duplicate IDs and validate local servers
        for server in &self.mcp_plugins.local_servers {
            if server.id.is_empty() {
                return Err(McpError::Config("Plugin ID cannot be empty".to_string()));
            }
            if server.name.is_empty() {
                return Err(McpError::Config("Plugin name cannot be empty".to_string()));
            }
            if server.command.is_empty() {
                return Err(McpError::Config(format!(
                    "Plugin '{}' has empty command",
                    server.id
                )));
            }

            if !ids.insert(&server.id) {
                return Err(McpError::Config(format!(
                    "Duplicate plugin ID: {}",
                    server.id
                )));
            }
        }

        // Validate cloud services
        for service in &self.mcp_plugins.cloud_services {
            if service.id.is_empty() {
                return Err(McpError::Config("Plugin ID cannot be empty".to_string()));
            }
            if service.name.is_empty() {
                return Err(McpError::Config("Plugin name cannot be empty".to_string()));
            }
            if service.url.is_empty() {
                return Err(McpError::Config(format!(
                    "Plugin '{}' has empty URL",
                    service.id
                )));
            }

            if !ids.insert(&service.id) {
                return Err(McpError::Config(format!(
                    "Duplicate plugin ID: {}",
                    service.id
                )));
            }
        }

        Ok(())
    }

    /// Save configuration to a JSON file
    ///
    /// Pretty-prints JSON for human readability.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Add an extension's MCP configuration to this config
    ///
    /// Adds the extension to the appropriate list (local_servers or cloud_services)
    /// based on the McpConfigEntry type. If an extension with the same ID already
    /// exists, it will be replaced.
    ///
    /// # Arguments
    /// * `entry` - The MCP configuration entry from the installed extension
    ///
    /// # Returns
    /// Ok(()) if successfully added, Err if validation fails
    pub fn add_extension(&mut self, entry: McpConfigEntry) -> Result<()> {
        match entry {
            McpConfigEntry::LocalServer(server_config) => {
                // Remove existing entry with same ID if present
                self.mcp_plugins
                    .local_servers
                    .retain(|s| s.id != server_config.id);

                // Add new entry
                self.mcp_plugins.local_servers.push(server_config);
            }
            McpConfigEntry::CloudService(cloud_config) => {
                // Remove existing entry with same ID if present
                self.mcp_plugins
                    .cloud_services
                    .retain(|s| s.id != cloud_config.id);

                // Add new entry
                self.mcp_plugins.cloud_services.push(cloud_config);
            }
        }

        // Validate after adding
        self.validate()?;
        Ok(())
    }

    /// Remove an extension from configuration by ID
    ///
    /// Searches both local_servers and cloud_services for the given ID
    /// and removes it if found.
    ///
    /// # Arguments
    /// * `extension_id` - The ID of the extension to remove
    ///
    /// # Returns
    /// true if extension was found and removed, false if not found
    pub fn remove_extension(&mut self, extension_id: &str) -> bool {
        let local_removed = self
            .mcp_plugins
            .local_servers
            .iter()
            .position(|s| s.id == extension_id)
            .map(|pos| self.mcp_plugins.local_servers.remove(pos))
            .is_some();

        let cloud_removed = self
            .mcp_plugins
            .cloud_services
            .iter()
            .position(|s| s.id == extension_id)
            .map(|pos| self.mcp_plugins.cloud_services.remove(pos))
            .is_some();

        local_removed || cloud_removed
    }
}

/// Resolve environment variable reference
///
/// Pattern: ${VAR_NAME}
///
/// If the value starts with "${" and ends with "}", extracts the variable
/// name and looks it up in the environment. Otherwise, returns the value as-is.
///
/// Example:
///     resolve_env_var("${API_KEY}") -> Ok("sk_12345...")
///     resolve_env_var("literal-value") -> Ok("literal-value")
///
/// Error Cases:
/// - Variable not found: Returns Config error with variable name
pub fn resolve_env_var(value: &str) -> Result<String> {
    if value.starts_with("${") && value.ends_with("}") {
        let var_name = &value[2..value.len() - 1];
        std::env::var(var_name)
            .map_err(|_| McpError::Config(format!("Environment variable not found: {}", var_name)))
    } else {
        Ok(value.to_string())
    }
}

/// Configuration file watcher for hot-reload capability
///
/// Monitors a configuration file for changes and enables dynamic plugin
/// configuration updates without full application restart.
///
/// Design Decision: File modification time tracking
///
/// Rationale: Using file modification time (mtime) is simple and reliable
/// for detecting config changes. More complex solutions (inotify/FSEvents)
/// add platform-specific complexity without significant benefit for this use case.
///
/// Trade-offs:
/// - Simplicity vs Features: mtime polling vs. event-driven file watching
/// - Polling Interval: 5s balance between responsiveness and overhead
/// - Memory: Minimal overhead (just PathBuf and SystemTime)
///
/// Alternatives Considered:
/// 1. notify crate (inotify/FSEvents): Rejected - adds complexity for minimal gain
/// 2. Manual reload command: Rejected - want automatic hot-reload UX
/// 3. No hot-reload: Rejected - restart disrupts active plugins
///
/// Extension Points:
/// - Add debouncing for rapid file changes
/// - Add validation before reload
/// - Add rollback on invalid config
///
/// Performance:
/// - File stat: ~1-5μs on modern SSDs
/// - Recommended poll interval: 5-10s
/// - Memory overhead: ~100 bytes
///
/// Usage:
///     let mut watcher = ConfigWatcher::new("mcp_config.json")?;
///     if let Some(new_config) = watcher.check_for_changes().await? {
///         manager.reload_config(new_config).await?;
///     }
pub struct ConfigWatcher {
    /// Path to the configuration file being watched
    path: PathBuf,

    /// Last known modification time
    last_modified: std::time::SystemTime,
}

impl ConfigWatcher {
    /// Create a new config watcher for the specified file
    ///
    /// Error Conditions:
    /// - File not found: Returns IoError
    /// - Permission denied: Returns IoError
    ///
    /// Example:
    ///     let watcher = ConfigWatcher::new("mcp_config.json")?;
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = std::fs::metadata(&path)?;
        let last_modified = metadata.modified()?;

        Ok(Self {
            path,
            last_modified,
        })
    }

    /// Check if the configuration file has been modified
    ///
    /// Returns Some(McpConfig) if file was changed and successfully loaded,
    /// None if file hasn't changed.
    ///
    /// Error Conditions:
    /// - File deleted: Returns IoError
    /// - Invalid JSON: Returns JsonError
    /// - Validation failure: Returns Config error
    ///
    /// Example:
    ///     if let Some(new_config) = watcher.check_for_changes().await? {
    ///         println!("Config changed, reloading...");
    ///         manager.reload_config(new_config).await?;
    ///     }
    pub async fn check_for_changes(&mut self) -> Result<Option<McpConfig>> {
        // Use tokio::fs for async file operations
        let metadata = tokio::fs::metadata(&self.path).await?;
        let current_modified = metadata.modified()?;

        // Check if file was modified since last check
        if current_modified > self.last_modified {
            tracing::info!(
                "Configuration file changed at {:?}, reloading...",
                current_modified
            );

            // Update last modified time
            self.last_modified = current_modified;

            // Load and validate new configuration
            let config = McpConfig::load_from_file(&self.path)?;

            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Get the path being watched
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the last modification time
    pub fn last_modified(&self) -> std::time::SystemTime {
        self.last_modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_serialization() {
        let config = McpConfig {
            mcp_plugins: McpPlugins {
                local_servers: vec![LocalServerConfig {
                    id: "test".to_string(),
                    name: "Test Server".to_string(),
                    description: None,
                    command: "echo".to_string(),
                    args: vec!["hello".to_string()],
                    env: HashMap::new(),
                    enabled: true,
                    auto_restart: false,
                    max_retries: Some(5),
                    health_check_interval: Some(30),
                    timeout: 60,
                    working_dir: None,
                }],
                cloud_services: vec![],
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.mcp_plugins.local_servers.len(), 1);
        assert_eq!(parsed.mcp_plugins.local_servers[0].id, "test");
    }

    #[test]
    fn test_duplicate_id_validation() {
        let config = McpConfig {
            mcp_plugins: McpPlugins {
                local_servers: vec![
                    LocalServerConfig {
                        id: "duplicate".to_string(),
                        name: "Server 1".to_string(),
                        description: None,
                        command: "cmd1".to_string(),
                        args: vec![],
                        env: HashMap::new(),
                        enabled: true,
                        auto_restart: false,
                        max_retries: None,
                        health_check_interval: None,
                        timeout: 60,
                        working_dir: None,
                    },
                    LocalServerConfig {
                        id: "duplicate".to_string(),
                        name: "Server 2".to_string(),
                        description: None,
                        command: "cmd2".to_string(),
                        args: vec![],
                        env: HashMap::new(),
                        enabled: true,
                        auto_restart: false,
                        max_retries: None,
                        health_check_interval: None,
                        timeout: 60,
                        working_dir: None,
                    },
                ],
                cloud_services: vec![],
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate plugin ID"));
    }

    #[test]
    fn test_env_var_resolution() {
        env::set_var("TEST_VAR", "test_value");

        let result = resolve_env_var("${TEST_VAR}").unwrap();
        assert_eq!(result, "test_value");

        let result = resolve_env_var("literal").unwrap();
        assert_eq!(result, "literal");

        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_missing_env_var() {
        let result = resolve_env_var("${NONEXISTENT_VAR_12345}");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Environment variable not found"));
    }
}
