// Centralized error handling using thiserror for type-safe error management
//
// Design Decision: Unified error type with context
//
// Rationale: Instead of using Box<dyn Error> throughout, we define specific
// error variants that map to different failure modes in the application.
// This enables pattern matching, better error messages, and type safety.
//
// Trade-offs:
// - Type Safety: Specific error types vs. generic Box<dyn Error>
// - Ergonomics: thiserror auto-derives Display and Error trait
// - Conversion: #[from] attribute handles automatic conversions from std errors
//
// Alternatives Considered:
// 1. Keep Box<dyn Error>: Rejected for lack of type safety and match-ability
// 2. anyhow::Error everywhere: Rejected because we want type-safe error variants
// 3. Custom Error enum without thiserror: Rejected due to boilerplate
//
// Extension Points: Add new error variants as needed for specific failure modes

use thiserror::Error;

/// Main error type for Rustbot application
///
/// This enum covers all error conditions that can occur in the application.
/// Each variant provides contextual information about what went wrong.
///
/// Usage:
///     fn load_file() -> Result<String> {
///         let content = std::fs::read_to_string(path)
///             .map_err(|e| RustbotError::StorageError(
///                 format!("Failed to read config: {}", e)
///             ))?;
///         Ok(content)
///     }
///
/// Error Handling Strategy:
/// - IO errors: Automatically converted via #[from] IoError variant
/// - Serde errors: Automatically converted via #[from] SerdeError variant
/// - HTTP errors: Automatically converted via #[from] ReqwestError variant
/// - Application errors: Use specific variants (AgentNotFound, ConfigError, etc.)
#[derive(Debug, Error)]
pub enum RustbotError {
    /// Agent with specified ID not found in registry
    ///
    /// This occurs when trying to switch to or access a non-existent agent.
    /// The string contains the agent ID that was not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// API-level error with context
    ///
    /// Generic API errors that don't fit other categories.
    /// Contains a descriptive message about what failed.
    #[error("API error: {0}")]
    ApiError(String),

    /// LLM adapter or communication error
    ///
    /// Errors from LLM providers (OpenRouter, Claude API, etc.)
    /// Includes connection failures, rate limits, and API errors.
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Configuration loading or validation error
    ///
    /// Failures in reading or parsing configuration files,
    /// environment variables, or validation of config values.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Storage/persistence error
    ///
    /// File system operations, database errors, or serialization failures
    /// when reading/writing persistent data.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Event bus communication error
    ///
    /// Failures in the pub/sub event system, including channel errors
    /// and event delivery failures.
    #[error("Event bus error: {0}")]
    EventError(String),

    /// IO operation failed (file, network, etc.)
    ///
    /// Wraps std::io::Error with automatic conversion via #[from].
    /// This covers file operations, network I/O, and system calls.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization failed
    ///
    /// Wraps serde_json::Error with automatic conversion.
    /// Occurs when parsing or generating JSON data.
    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// HTTP request failed
    ///
    /// Wraps reqwest::Error with automatic conversion.
    /// Network errors, HTTP errors, and request building failures.
    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Environment variable not found or invalid
    ///
    /// Missing required environment variables (API keys, etc.)
    /// or values that fail validation.
    #[error("Environment error: {0}")]
    EnvError(String),

    /// Directory or path error
    ///
    /// Failures related to path operations: invalid paths,
    /// missing directories, or permission issues.
    #[error("Path error: {0}")]
    PathError(String),
}

/// Type alias for Result with RustbotError
///
/// Use this instead of std::result::Result<T, RustbotError> for convenience.
///
/// Example:
///     fn load_config() -> Result<Config> {
///         // ...
///     }
pub type Result<T> = std::result::Result<T, RustbotError>;

// Conversion from anyhow::Error for gradual migration
//
// This allows existing code using anyhow to interoperate with new
// thiserror-based code during the transition period.
impl From<anyhow::Error> for RustbotError {
    fn from(err: anyhow::Error) -> Self {
        RustbotError::ApiError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RustbotError::AgentNotFound("researcher".to_string());
        assert_eq!(err.to_string(), "Agent not found: researcher");

        let err = RustbotError::StorageError("Failed to save".to_string());
        assert_eq!(err.to_string(), "Storage error: Failed to save");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rustbot_err: RustbotError = io_err.into();

        match rustbot_err {
            RustbotError::IoError(_) => {} // Success
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_error() -> Result<i32> {
            Err(RustbotError::ConfigError("test error".to_string()))
        }

        let result = returns_error();
        assert!(result.is_err());
    }
}
