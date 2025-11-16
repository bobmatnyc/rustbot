//! MCP Error Types
//!
//! Design Decision: Dedicated error types for MCP plugin system
//!
//! Rationale: MCP plugins have specific failure modes (configuration errors,
//! transport failures, protocol violations) that warrant dedicated error types
//! separate from core RustbotError. This enables better error handling and
//! more actionable error messages for users.
//!
//! Trade-offs:
//! - Type Safety: Specific error variants vs. generic string errors
//! - Error Context: Rich error information vs. simple messages
//! - User Experience: Actionable error messages vs. technical details
//!
//! Alternatives Considered:
//! 1. Reuse RustbotError: Rejected - MCP has domain-specific error conditions
//! 2. String-based errors: Rejected - loses type safety and pattern matching
//! 3. anyhow::Error: Rejected - want specific error variants for UI display
//!
//! Extension Points: Add new error variants as MCP functionality expands
//! (e.g., ToolExecutionError, PermissionDenied, ProtocolVersionMismatch)

use thiserror::Error;

/// MCP-specific errors
///
/// These errors represent failures in the MCP plugin system, including
/// configuration issues, transport failures, and protocol violations.
///
/// Error Handling Strategy:
/// - Configuration errors: User must fix config file
/// - Transport errors: Auto-retry with exponential backoff
/// - Protocol errors: Log and mark plugin as errored
/// - Plugin lifecycle errors: Provide clear user feedback
///
/// Usage:
///     fn load_plugin_config() -> Result<PluginConfig> {
///         let config = std::fs::read_to_string(path)?;
///         serde_json::from_str(&config).map_err(McpError::from)
///     }
#[derive(Debug, Error)]
pub enum McpError {
    /// Configuration file or validation error
    ///
    /// Examples: Invalid JSON, duplicate plugin IDs, missing required fields
    /// Recovery: User must fix configuration file
    #[error("Configuration error: {0}")]
    Config(String),

    /// Plugin with specified ID not found in registry
    ///
    /// This occurs when trying to start, stop, or interact with a non-existent plugin.
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    /// Plugin already exists in registry
    ///
    /// Prevents duplicate plugin IDs when adding new plugins
    #[error("Plugin already exists: {0}")]
    PluginAlreadyExists(String),

    /// Transport-level error (stdio, HTTP, etc.)
    ///
    /// Examples: Process spawn failed, connection timeout, broken pipe
    /// Recovery: Auto-retry with exponential backoff
    #[error("Transport error: {0}")]
    Transport(String),

    /// MCP protocol violation or unexpected message
    ///
    /// Examples: Invalid JSON-RPC, missing required fields, protocol version mismatch
    /// Recovery: Mark plugin as errored, log for debugging
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// IO operation failed
    ///
    /// Wraps std::io::Error with automatic conversion via #[from]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization failed
    ///
    /// Wraps serde_json::Error with automatic conversion
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Type alias for Result with McpError
///
/// Use this throughout the MCP module for consistent error handling.
///
/// Example:
///     pub fn initialize_plugin(id: &str) -> Result<PluginState> {
///         // ...
///     }
pub type Result<T> = std::result::Result<T, McpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = McpError::PluginNotFound("test-plugin".to_string());
        assert_eq!(err.to_string(), "Plugin not found: test-plugin");

        let err = McpError::Config("Duplicate ID: filesystem".to_string());
        assert_eq!(err.to_string(), "Configuration error: Duplicate ID: filesystem");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mcp_err: McpError = io_err.into();

        match mcp_err {
            McpError::Io(_) => {}, // Success
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_json_error_conversion() {
        let json = r#"{ invalid json }"#;
        let result: std::result::Result<serde_json::Value, _> = serde_json::from_str(json);

        if let Err(json_err) = result {
            let mcp_err: McpError = json_err.into();
            match mcp_err {
                McpError::Json(_) => {}, // Success
                _ => panic!("Expected Json variant"),
            }
        }
    }
}
