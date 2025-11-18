// Library interface for Rustbot
// This exposes the core functionality as a library that can be:
// - Used programmatically from Rust code
// - Called from scripts and tests
// - Integrated into other applications

pub mod agent;
pub mod api;
pub mod error;
pub mod events;
pub mod llm;
pub mod mcp;  // MCP (Model Context Protocol) plugin system
pub mod mermaid;  // Mermaid diagram rendering
pub mod services;  // Service layer for dependency injection (Phase 1 - additive)
pub mod tool_executor;
pub mod version;

// Re-export commonly used types for convenience
pub use api::{RustbotApi, RustbotApiBuilder};
pub use agent::{Agent, AgentConfig, AgentLoader, JsonAgentConfig};
pub use error::{Result, RustbotError};
pub use events::{Event, EventBus, EventKind, AgentStatus};
pub use llm::{LlmAdapter, LlmProvider, Message as LlmMessage, LlmRequest};

// Re-export service layer types (Phase 1 - new dependency injection layer)
// Note: These are additive and don't affect existing code paths.
// Services can be used for new code or gradual migration of existing code.
pub use services::{
    FileSystem, StorageService, ConfigService, AgentService,
    RealFileSystem, FileStorageService, FileConfigService, DefaultAgentService,
};
