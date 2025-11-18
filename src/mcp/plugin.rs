//! MCP Plugin State and Metadata Types
//!
//! Design Decision: Explicit state machine for plugin lifecycle
//!
//! Rationale: Plugins go through distinct lifecycle states (Stopped -> Starting ->
//! Initializing -> Running -> Error). Explicit state tracking enables better UI
//! feedback, error recovery logic, and debugging. The state machine prevents
//! invalid transitions (e.g., can't stop an already stopped plugin).
//!
//! Trade-offs:
//! - Complexity vs Safety: Explicit states add code but prevent bugs
//! - Memory vs Features: Store rich metadata for UI display
//! - Performance vs Debugging: Track errors and restart counts for recovery
//!
//! Alternatives Considered:
//! 1. Boolean flags (is_running, has_error): Rejected - loses state transitions
//! 2. Simple enum without metadata: Rejected - need tool lists for UI
//! 3. External state tracking: Rejected - state belongs with plugin
//!
//! Extension Points:
//! - Add new states (e.g., Suspended, Updating, Reconnecting)
//! - Add plugin-specific metadata (permissions, resource usage)
//! - Add state transition history for debugging

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::config::{CloudServiceConfig, LocalServerConfig};

/// Plugin lifecycle state
///
/// State Machine Transitions:
/// ```text
/// Disabled ─enable─> Stopped ─start─> Starting ─spawn─> Initializing
///                        ▲                                    │
///                        │                               handshake
///                        │                                    │
///                     disable                                 ▼
///                        │                                Running
///                        │                                    │
///                        └────────── stop/error ──────────────┘
/// ```
///
/// Usage:
///     match plugin.state {
///         PluginState::Running => // Show green status icon
///         PluginState::Error { .. } => // Show error message
///         _ => // Show intermediate state
///     }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PluginState {
    /// Plugin is configured but not enabled
    ///
    /// User must explicitly enable to transition to Stopped
    Disabled,

    /// Plugin is enabled but not yet started
    ///
    /// Will transition to Starting when manager initializes it
    Stopped,

    /// Plugin is being started (process spawning / HTTP connecting)
    ///
    /// Transient state - should quickly move to Initializing or Error
    Starting,

    /// Plugin is performing MCP handshake (initialize/initialized exchange)
    ///
    /// Transient state - should quickly move to Running or Error
    Initializing,

    /// Plugin is operational and ready to handle tool calls
    ///
    /// This is the target state for healthy plugins
    Running,

    /// Plugin is being shut down
    ///
    /// Transient state - should quickly move to Stopped
    Stopping,

    /// Plugin encountered an error
    ///
    /// Contains error message and timestamp for debugging
    Error {
        message: String,
        timestamp: SystemTime,
    },
}

/// Plugin type discriminator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Local MCP server (stdio transport)
    LocalServer,

    /// Cloud MCP service (HTTP transport)
    CloudService,
}

/// Comprehensive plugin metadata
///
/// Stores all information about a plugin needed for:
/// - UI display (name, status, tool count)
/// - Plugin management (state, error info)
/// - Tool routing (available tools)
/// - Error recovery (restart count)
///
/// Performance:
/// - Time Complexity: O(1) for all field access
/// - Space Complexity: O(n) where n = number of tools
/// - Cloning: Moderate cost due to tool lists
///
/// Optimization Suggestions:
/// - For large tool lists (>100), consider Arc<Vec<ToolInfo>> to reduce clone cost
/// - For high-frequency state checks, add cached is_running() helper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,

    /// Display name for UI
    pub name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Plugin type (local server or cloud service)
    pub plugin_type: PluginType,

    /// Current lifecycle state
    pub state: PluginState,

    /// List of tools provided by this plugin
    ///
    /// Populated after successful initialization via tools/list RPC call
    #[serde(default)]
    pub tools: Vec<ToolInfo>,

    /// List of resources provided by this plugin
    ///
    /// Resources are MCP-specific content (files, database queries, etc.)
    /// that the plugin can provide. Populated during initialization.
    #[serde(default)]
    pub resources: Vec<ResourceInfo>,

    /// List of prompts provided by this plugin
    ///
    /// Prompts are reusable prompt templates with parameters.
    /// Populated during initialization.
    #[serde(default)]
    pub prompts: Vec<PromptInfo>,

    /// Number of times plugin has been restarted since last successful start
    ///
    /// Used for exponential backoff calculation:
    /// - 0 restarts: 1s delay
    /// - 1 restart: 2s delay
    /// - 2 restarts: 4s delay
    /// - 3 restarts: 8s delay
    /// - 4 restarts: 16s delay
    /// - 5+ restarts: 32s delay (max)
    #[serde(default)]
    pub restart_count: u32,

    /// Timestamp of last restart attempt
    ///
    /// Used to implement restart backoff logic and track restart patterns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_restart: Option<SystemTime>,

    /// Maximum number of restart attempts before marking plugin as failed
    ///
    /// Loaded from config file, defaults to 5
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

// Default value for max_retries
fn default_max_retries() -> u32 {
    5
}

/// Tool information from MCP tools/list response
///
/// Represents a tool that can be called via the tools/call RPC method.
///
/// Example:
///     {
///       "name": "read_file",
///       "description": "Read contents of a file",
///       "input_schema": {
///         "type": "object",
///         "properties": {
///           "path": { "type": "string" }
///         },
///         "required": ["path"]
///       }
///     }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool identifier (must be unique within plugin)
    pub name: String,

    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema describing tool input parameters
    ///
    /// This schema is used to:
    /// 1. Validate arguments before sending to plugin
    /// 2. Generate UI forms for manual tool invocation
    /// 3. Include in LLM tool definitions for Claude/GPT
    pub input_schema: serde_json::Value,
}

/// Resource information from MCP resources/list response
///
/// Resources represent content that can be read via resources/read RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Resource URI (e.g., "file:///path/to/file", "db://table/row")
    pub uri: String,

    /// Display name
    pub name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type (e.g., "text/plain", "application/json")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Prompt template information from MCP prompts/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// Prompt identifier
    pub name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Template arguments
    #[serde(default)]
    pub arguments: Vec<PromptArgument>,
}

/// Prompt template argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether argument is required
    #[serde(default)]
    pub required: bool,
}

impl PluginMetadata {
    /// Create new metadata for a local server
    pub fn new_local_server(config: &LocalServerConfig) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            description: config.description.clone(),
            plugin_type: PluginType::LocalServer,
            state: if config.enabled {
                PluginState::Stopped
            } else {
                PluginState::Disabled
            },
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            restart_count: 0,
            last_restart: None,
            max_retries: config.max_retries.unwrap_or(5),
        }
    }

    /// Create new metadata for a cloud service
    pub fn new_cloud_service(config: &CloudServiceConfig) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            description: config.description.clone(),
            plugin_type: PluginType::CloudService,
            state: if config.enabled {
                PluginState::Stopped
            } else {
                PluginState::Disabled
            },
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            restart_count: 0,
            last_restart: None,
            max_retries: config.max_retries.unwrap_or(5),
        }
    }

    /// Check if plugin is in a running state
    pub fn is_running(&self) -> bool {
        matches!(self.state, PluginState::Running)
    }

    /// Check if plugin is in an error state
    pub fn is_error(&self) -> bool {
        matches!(self.state, PluginState::Error { .. })
    }

    /// Get error message if in error state
    pub fn error_message(&self) -> Option<&str> {
        match &self.state {
            PluginState::Error { message, .. } => Some(message),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_serialization() {
        let state = PluginState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("running"));

        let state = PluginState::Error {
            message: "Test error".to_string(),
            timestamp: SystemTime::now(),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Test error"));
    }

    #[test]
    fn test_plugin_metadata_creation() {
        let config = LocalServerConfig {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
            description: Some("A test plugin".to_string()),
            command: "echo".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            enabled: true,
            auto_restart: false,
            max_retries: Some(5),
            health_check_interval: Some(30),
            timeout: 60,
            working_dir: None,
        };

        let metadata = PluginMetadata::new_local_server(&config);

        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.name, "Test Plugin");
        assert_eq!(metadata.plugin_type, PluginType::LocalServer);
        assert_eq!(metadata.state, PluginState::Stopped);
        assert_eq!(metadata.restart_count, 0);
    }

    #[test]
    fn test_plugin_metadata_helpers() {
        let mut metadata = PluginMetadata {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            plugin_type: PluginType::LocalServer,
            state: PluginState::Running,
            tools: vec![],
            resources: vec![],
            prompts: vec![],
            restart_count: 0,
            last_restart: None,
            max_retries: 5,
        };

        assert!(metadata.is_running());
        assert!(!metadata.is_error());
        assert_eq!(metadata.error_message(), None);

        metadata.state = PluginState::Error {
            message: "Failed".to_string(),
            timestamp: SystemTime::now(),
        };

        assert!(!metadata.is_running());
        assert!(metadata.is_error());
        assert_eq!(metadata.error_message(), Some("Failed"));
    }
}
