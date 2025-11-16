//! MCP (Model Context Protocol) Plugin System
//!
//! This module implements support for MCP plugins in Rustbot, enabling integration
//! with both local MCP servers (via stdio) and cloud MCP services (via HTTP).
//!
//! # Architecture Overview
//!
//! The MCP plugin system consists of several layers:
//!
//! 1. **Configuration Layer** (`config.rs`)
//!    - JSON-based configuration matching Claude Desktop format
//!    - Environment variable substitution for secrets
//!    - Validation of plugin definitions
//!
//! 2. **Plugin Management Layer** (`manager.rs`)
//!    - Lifecycle coordination (start, stop, restart)
//!    - State tracking and error recovery
//!    - Tool registration and dispatch
//!
//! 3. **Plugin State Layer** (`plugin.rs`)
//!    - Explicit state machine (Stopped → Starting → Running → Error)
//!    - Metadata storage (tools, resources, prompts)
//!    - Restart count tracking for backoff logic
//!
//! 4. **Transport Layer** (Phase 2: `transport/`)
//!    - stdio transport for local servers
//!    - HTTP/SSE transport for cloud services
//!    - JSON-RPC 2.0 protocol implementation
//!
//! 5. **Error Handling Layer** (`error.rs`)
//!    - MCP-specific error types
//!    - Clear error messages for users
//!    - Automatic error recovery strategies
//!
//! # Implementation Status
//!
//! ## Phase 1: Foundation (CURRENT)
//! - ✅ Configuration schema and loading
//! - ✅ Plugin metadata and state machine
//! - ✅ Basic plugin manager
//! - ✅ Error types
//!
//! ## Phase 2: stdio Transport (NEXT)
//! - ⏳ Process spawning with tokio::process
//! - ⏳ stdin/stdout JSON-RPC communication
//! - ⏳ MCP handshake (initialize/initialized)
//! - ⏳ Tool discovery (tools/list)
//! - ⏳ Tool execution (tools/call)
//!
//! ## Phase 3: Plugin Manager Core
//! - ⏳ Auto-restart with exponential backoff
//! - ⏳ Tool registry integration
//! - ⏳ Event bus integration
//! - ⏳ Configuration hot-reload
//!
//! ## Phase 4: UI Integration
//! - ⏳ Plugins pane in egui
//! - ⏳ Plugin cards with status icons
//! - ⏳ Enable/disable controls
//! - ⏳ Configuration viewer/editor
//!
//! ## Phase 5: HTTP Transport
//! - ⏳ HTTP/SSE transport
//! - ⏳ OAuth 2.1 authentication
//! - ⏳ Session management
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use rustbot::mcp::{McpPluginManager, McpConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create plugin manager
//!     let mut manager = McpPluginManager::new();
//!
//!     // Load configuration
//!     manager.load_config(Path::new("mcp_config.json")).await?;
//!
//!     // List available plugins
//!     let plugins = manager.list_plugins().await;
//!     for plugin in plugins {
//!         println!("{}: {}", plugin.name, plugin.state);
//!     }
//!
//!     // Phase 2+: Start a plugin
//!     // manager.start_plugin("filesystem").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Design Principles
//!
//! 1. **Type Safety**: Use Rust's type system to prevent invalid states
//! 2. **Async-First**: All I/O operations are async to avoid blocking UI
//! 3. **Error Recovery**: Automatic retry with exponential backoff
//! 4. **User-Friendly**: Clear error messages with actionable fixes
//! 5. **Compatibility**: Match Claude Desktop's configuration format
//!
//! # Security Considerations
//!
//! - Environment variables for secrets (not stored in config files)
//! - Process isolation for local servers
//! - Future: Plugin permissions system for file access control
//! - Future: OAuth token secure storage (keychain integration)
//!
//! # Performance Characteristics
//!
//! - Plugin lookup: O(1) via HashMap
//! - Plugin list: O(n) where n = number of plugins
//! - Concurrent plugin operations: Supported via Arc<RwLock<>>
//! - Memory overhead: ~1KB per plugin (excluding tool schemas)

pub mod config;
pub mod error;
pub mod manager;
pub mod plugin;
// pub mod transport;  // Phase 2: Transport layer (stdio, HTTP)

// Re-export commonly used types for convenience
pub use config::{
    McpConfig,
    McpPlugins,
    LocalServerConfig,
    CloudServiceConfig,
    AuthConfig,
    resolve_env_var,
};

pub use plugin::{
    PluginMetadata,
    PluginState,
    PluginType,
    ToolInfo,
    ResourceInfo,
    PromptInfo,
    PromptArgument,
};

pub use manager::{
    McpPluginManager,
    PluginInfo,
};

pub use error::{
    McpError,
    Result,
};

/// MCP Protocol Version
///
/// We target the latest stable MCP protocol version: 2024-11-05
/// This version includes:
/// - Core JSON-RPC 2.0 messaging
/// - Tools (tools/list, tools/call)
/// - Resources (resources/list, resources/read)
/// - Prompts (prompts/list, prompts/get)
/// - Logging capabilities
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Client information sent during MCP handshake
pub const MCP_CLIENT_NAME: &str = "rustbot";

/// Get MCP client version (matches Rustbot version)
pub fn mcp_client_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        assert_eq!(MCP_PROTOCOL_VERSION, "2024-11-05");
    }

    #[test]
    fn test_client_info() {
        assert_eq!(MCP_CLIENT_NAME, "rustbot");
        assert!(!mcp_client_version().is_empty());
    }
}
