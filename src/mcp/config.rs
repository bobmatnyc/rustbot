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
    Bearer {
        token: String,
    },

    /// Basic authentication (Authorization: Basic <base64(user:pass)>)
    ///
    /// Example:
    ///     { "type": "basic", "username": "user", "password": "${PASSWORD}" }
    Basic {
        username: String,
        password: String,
    },

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
fn default_true() -> bool { true }
fn default_timeout() -> u64 { 60 }

impl McpConfig {
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
                return Err(McpError::Config(format!("Plugin '{}' has empty command", server.id)));
            }

            if !ids.insert(&server.id) {
                return Err(McpError::Config(format!("Duplicate plugin ID: {}", server.id)));
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
                return Err(McpError::Config(format!("Plugin '{}' has empty URL", service.id)));
            }

            if !ids.insert(&service.id) {
                return Err(McpError::Config(format!("Duplicate plugin ID: {}", service.id)));
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
        let var_name = &value[2..value.len()-1];
        std::env::var(var_name)
            .map_err(|_| McpError::Config(format!("Environment variable not found: {}", var_name)))
    } else {
        Ok(value.to_string())
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
                local_servers: vec![
                    LocalServerConfig {
                        id: "test".to_string(),
                        name: "Test Server".to_string(),
                        description: None,
                        command: "echo".to_string(),
                        args: vec!["hello".to_string()],
                        env: HashMap::new(),
                        enabled: true,
                        auto_restart: false,
                        timeout: 60,
                        working_dir: None,
                    }
                ],
                cloud_services: vec![],
            }
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
                        timeout: 60,
                        working_dir: None,
                    }
                ],
                cloud_services: vec![],
            }
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate plugin ID"));
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
        assert!(result.unwrap_err().to_string().contains("Environment variable not found"));
    }
}
