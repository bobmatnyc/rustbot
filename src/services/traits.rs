// Core trait definitions for service layer dependency injection
//
// Design Decision: Trait-based abstractions for infrastructure concerns
//
// Rationale: Traits provide compile-time polymorphism in Rust, enabling:
// 1. Zero-cost abstractions (no runtime overhead vs. direct calls)
// 2. Type-safe mocking (trait objects with Arc<dyn Trait>)
// 3. Explicit contracts for infrastructure dependencies
// 4. Send + Sync bounds for async/concurrent safety
//
// All traits are marked Send + Sync to work with tokio's async runtime,
// which requires thread-safe types for spawning tasks across threads.

use crate::agent::{Agent, AgentConfig};
use crate::error::Result;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::path::Path;
use std::sync::Arc;

/// Filesystem abstraction for file I/O operations
///
/// Provides async file operations to enable dependency injection and testing.
/// All operations use tokio::fs under the hood for non-blocking I/O.
///
/// Design: Simple, focused interface covering common file operations.
/// More specialized operations can be added as needed.
///
/// Usage:
///     let fs: Arc<dyn FileSystem> = Arc::new(RealFileSystem);
///     let content = fs.read_to_string(Path::new("config.json")).await?;
#[cfg_attr(test, automock)]
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Read entire file contents as a UTF-8 string
    ///
    /// # Errors
    /// - File not found
    /// - Permission denied
    /// - Invalid UTF-8 encoding
    async fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Write string content to a file (creates or overwrites)
    ///
    /// # Errors
    /// - Permission denied
    /// - Disk full
    /// - Invalid path
    async fn write(&self, path: &Path, content: &str) -> Result<()>;

    /// Check if a path exists (file or directory)
    ///
    /// Returns false on permission errors (cannot distinguish from non-existence)
    async fn exists(&self, path: &Path) -> bool;

    /// Create directory and all parent directories (like mkdir -p)
    ///
    /// # Errors
    /// - Permission denied
    /// - Disk full
    /// - Invalid path
    async fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Read directory entries, returning file paths
    ///
    /// # Errors
    /// - Directory not found
    /// - Permission denied
    async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>>;
}

/// Storage service for application data persistence
///
/// High-level interface for reading/writing structured application data.
/// Abstracts serialization format and storage location from business logic.
///
/// Design: Domain-specific storage operations (not generic read/write).
/// Each method handles a specific data type with appropriate serialization.
///
/// Usage:
///     let storage: Arc<dyn StorageService> = Arc::new(FileStorageService::new(...));
///     let stats = storage.load_token_stats().await?;
///     storage.save_token_stats(&updated_stats).await?;
#[cfg_attr(test, automock)]
#[async_trait]
pub trait StorageService: Send + Sync {
    /// Load token usage statistics from persistent storage
    ///
    /// Returns default/empty stats if file doesn't exist (first run).
    ///
    /// # Errors
    /// - Deserialization errors (corrupt data)
    /// - Permission errors
    async fn load_token_stats(&self) -> Result<TokenStats>;

    /// Save token usage statistics to persistent storage
    ///
    /// # Errors
    /// - Serialization errors
    /// - Write errors (disk full, permissions)
    async fn save_token_stats(&self, stats: &TokenStats) -> Result<()>;

    /// Load system prompts from persistent storage
    ///
    /// Returns default prompts if file doesn't exist.
    ///
    /// # Errors
    /// - Deserialization errors
    /// - Permission errors
    async fn load_system_prompts(&self) -> Result<SystemPrompts>;

    /// Save system prompts to persistent storage
    ///
    /// # Errors
    /// - Serialization errors
    /// - Write errors
    async fn save_system_prompts(&self, prompts: &SystemPrompts) -> Result<()>;

    /// Load user profile from persistent storage
    ///
    /// Returns default profile if file doesn't exist.
    ///
    /// # Errors
    /// - Deserialization errors
    /// - Permission errors
    async fn load_user_profile(&self) -> Result<UserProfile>;

    /// Save user profile to persistent storage
    ///
    /// # Errors
    /// - Serialization errors
    /// - Write errors
    async fn save_user_profile(&self, profile: &UserProfile) -> Result<()>;
}

/// Configuration service for application settings
///
/// Centralized access to configuration from environment variables and config files.
/// Handles validation and provides type-safe accessors.
///
/// Design: Configuration is loaded once at startup and cached in memory.
/// This avoids repeated file/env reads and provides consistent config during runtime.
///
/// Usage:
///     let config: Arc<dyn ConfigService> = Arc::new(FileConfigService::load()?);
///     let api_key = config.get_api_key()?;
///     let agents_dir = config.get_agents_dir();
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ConfigService: Send + Sync {
    /// Load all agent configurations from configured directory
    ///
    /// # Errors
    /// - Directory read errors
    /// - JSON parse errors
    /// - Validation errors
    async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>>;

    /// Save an agent configuration to the agent directory
    ///
    /// # Errors
    /// - Serialization errors
    /// - Write errors
    async fn save_agent_config(&self, config: &AgentConfig) -> Result<()>;

    /// Get the currently active agent ID
    ///
    /// # Errors
    /// - Config file not found or corrupt
    async fn get_active_agent_id(&self) -> Result<String>;

    /// Set the active agent ID
    ///
    /// # Errors
    /// - Write errors
    async fn set_active_agent_id(&self, id: &str) -> Result<()>;

    /// Get agents directory path
    fn get_agents_dir(&self) -> std::path::PathBuf;

    /// Get API key (from config or environment)
    ///
    /// # Errors
    /// - API key not configured
    fn get_api_key(&self) -> Result<String>;

    /// Get default model identifier
    fn get_model(&self) -> String;
}

/// Agent service for managing the agent registry
///
/// Provides access to loaded agents and handles agent switching.
/// Maintains the registry of available agents and current selection.
///
/// Design: Centralized agent registry with lazy loading.
/// Agents are loaded from config on service creation and cached in memory.
///
/// Usage:
///     let service: Arc<dyn AgentService> = DefaultAgentService::new(config).await?;
///     let agent = service.get_agent("researcher").await?;
///     let all_agents = service.list_agents();
///     service.switch_agent("writer").await?;
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AgentService: Send + Sync {
    /// Get agent by ID
    ///
    /// # Errors
    /// - Agent not found
    /// - Agent disabled
    async fn get_agent(&self, id: &str) -> Result<Arc<Agent>>;

    /// List all available agent IDs
    fn list_agents(&self) -> Vec<String>;

    /// Switch to a different agent
    ///
    /// # Errors
    /// - Agent not found
    /// - Agent disabled
    async fn switch_agent(&mut self, id: &str) -> Result<()>;

    /// Get the current active agent
    fn current_agent(&self) -> Arc<Agent>;
}

// Placeholder types for StorageService
// These should be moved to appropriate modules once storage is implemented

/// Token usage statistics
///
/// Tracks cumulative token usage and costs across all conversations.
/// Used for budget monitoring and usage analytics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenStats {
    /// Total input tokens consumed
    pub total_input_tokens: u64,

    /// Total output tokens generated
    pub total_output_tokens: u64,

    /// Total cost in USD
    pub total_cost: f64,

    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for TokenStats {
    fn default() -> Self {
        Self {
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// User profile information for personalization
///
/// Stores user details that can be used to personalize AI responses.
/// This information is provided as context in system prompts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    /// User's full name
    pub name: String,

    /// User's email address
    pub email: String,

    /// User's timezone (e.g., "America/Los_Angeles")
    pub timezone: Option<String>,

    /// User's location (e.g., "San Francisco, CA")
    pub location: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            timezone: None,
            location: None,
        }
    }
}

/// System-level prompts shared across agents
///
/// Contains common instructions and context that all agents inherit.
/// Allows customization of base behavior without modifying agent configs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemPrompts {
    /// Base system prompt for all agents
    pub base_prompt: String,

    /// Additional context or instructions
    pub context: Option<String>,
}

impl Default for SystemPrompts {
    fn default() -> Self {
        Self {
            base_prompt: String::new(),
            context: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_stats_default() {
        let stats = TokenStats::default();
        assert_eq!(stats.total_input_tokens, 0);
        assert_eq!(stats.total_output_tokens, 0);
        assert_eq!(stats.total_cost, 0.0);
    }

    #[test]
    fn test_system_prompts_default() {
        let prompts = SystemPrompts::default();
        assert_eq!(prompts.base_prompt, "");
        assert_eq!(prompts.context, None);
    }
}
