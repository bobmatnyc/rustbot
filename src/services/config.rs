// Configuration service implementation
//
// Design Decision: Environment variables with file-based agent configs
//
// Rationale: Hybrid approach balances security and flexibility:
// 1. Sensitive data (API keys) from environment variables (not in git)
// 2. Agent configs from JSON files (versioned, easy to modify)
// 3. Runtime config cached in memory (avoid repeated file/env reads)
//
// Trade-offs:
// - Security: Environment variables vs. config files (env vars safer for secrets)
// - Flexibility: File-based configs vs. hardcoded (files more flexible)
// - Performance: Load once at startup vs. read every time (startup load faster)
//
// Extension Points: Can add database backend or cloud config service
// by implementing ConfigService trait (no business logic changes).

use super::traits::ConfigService;
use crate::agent::{AgentConfig, AgentLoader};
use crate::error::{Result, RustbotError};
use async_trait::async_trait;
use std::path::PathBuf;

/// File-based configuration service
///
/// Loads configuration from:
/// 1. Environment variables (.env file via dotenvy)
/// 2. Agent JSON files in configured directory
///
/// Configuration is loaded once at service creation and cached.
/// This avoids repeated file I/O and provides consistent config.
///
/// Thread Safety: All config is immutable after loading, safe to share.
///
/// Usage:
///     let config = FileConfigService::load()?;
///     let api_key = config.get_api_key()?;
///     let agents = config.load_agent_configs().await?;
pub struct FileConfigService {
    /// API key for LLM provider
    api_key: String,

    /// Default model to use
    model: String,

    /// Directory containing agent JSON files
    agents_dir: PathBuf,

    /// Active agent ID (loaded from config or default)
    active_agent_id: String,
}

impl FileConfigService {
    /// Load configuration from environment and files
    ///
    /// Loads .env file and reads environment variables.
    ///
    /// Environment Variables:
    /// - OPENROUTER_API_KEY (required): API key for OpenRouter
    /// - MODEL (optional): Default model, defaults to anthropic/claude-sonnet-4
    /// - AGENTS_DIR (optional): Agent configs directory, defaults to "agents"
    /// - ACTIVE_AGENT_ID (optional): Default active agent, defaults to "assistant"
    ///
    /// # Errors
    /// - OPENROUTER_API_KEY not set
    /// - Environment variable parsing errors
    pub fn load() -> Result<Self> {
        // Load .env file (ignore if not found)
        dotenvy::dotenv().ok();

        // Get API key (required)
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| RustbotError::EnvError(
                "OPENROUTER_API_KEY environment variable not set".to_string()
            ))?;

        // Get optional config with defaults
        let model = std::env::var("MODEL")
            .unwrap_or_else(|_| "anthropic/claude-sonnet-4".to_string());

        let agents_dir = std::env::var("AGENTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("agents"));

        let active_agent_id = std::env::var("ACTIVE_AGENT_ID")
            .unwrap_or_else(|_| "assistant".to_string());

        Ok(Self {
            api_key,
            model,
            agents_dir,
            active_agent_id,
        })
    }
}

#[async_trait]
impl ConfigService for FileConfigService {
    async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>> {
        let loader = AgentLoader::new();

        loader.load_all()
            .map_err(|e| RustbotError::ConfigError(
                format!("Failed to load agent configs: {}", e)
            ))
    }

    async fn save_agent_config(&self, config: &AgentConfig) -> Result<()> {
        // For now, we don't implement saving agents to JSON
        // This can be added when we need runtime agent creation
        Err(RustbotError::ConfigError(
            "Saving agent configs not yet implemented".to_string()
        ))
    }

    async fn get_active_agent_id(&self) -> Result<String> {
        Ok(self.active_agent_id.clone())
    }

    async fn set_active_agent_id(&self, _id: &str) -> Result<()> {
        // For now, we don't persist active agent changes
        // This can be added when we implement user preferences storage
        Err(RustbotError::ConfigError(
            "Setting active agent ID not yet implemented".to_string()
        ))
    }

    fn get_agents_dir(&self) -> PathBuf {
        self.agents_dir.clone()
    }

    fn get_api_key(&self) -> Result<String> {
        Ok(self.api_key.clone())
    }

    fn get_model(&self) -> String {
        self.model.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize config tests to avoid env var conflicts
    static CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_service_requires_api_key() {
        let _lock = CONFIG_TEST_LOCK.lock().unwrap();

        // Clear all config env vars
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("MODEL");
        std::env::remove_var("AGENTS_DIR");
        std::env::remove_var("ACTIVE_AGENT_ID");

        let result = FileConfigService::load();
        assert!(result.is_err(), "Expected error when API key not set");

        match result {
            Err(RustbotError::EnvError(msg)) => {
                assert!(msg.contains("OPENROUTER_API_KEY"));
            }
            _ => panic!("Expected EnvError"),
        }
    }

    #[test]
    fn test_config_service_with_api_key() {
        let _lock = CONFIG_TEST_LOCK.lock().unwrap();

        // Clear all first
        std::env::remove_var("MODEL");
        std::env::remove_var("AGENTS_DIR");
        std::env::remove_var("ACTIVE_AGENT_ID");

        // Set API key
        std::env::set_var("OPENROUTER_API_KEY", "test-key-12345");

        let config = FileConfigService::load().unwrap();

        assert_eq!(config.get_api_key().unwrap(), "test-key-12345");
        assert_eq!(config.get_model(), "anthropic/claude-sonnet-4");
        assert_eq!(config.get_agents_dir(), PathBuf::from("agents"));

        // Cleanup
        std::env::remove_var("OPENROUTER_API_KEY");
    }

    #[test]
    fn test_config_service_with_custom_values() {
        let _lock = CONFIG_TEST_LOCK.lock().unwrap();

        // Set custom environment variables
        std::env::set_var("OPENROUTER_API_KEY", "custom-key");
        std::env::set_var("MODEL", "anthropic/claude-opus-4");
        std::env::set_var("AGENTS_DIR", "custom/agents");
        std::env::set_var("ACTIVE_AGENT_ID", "researcher");

        let config = FileConfigService::load().unwrap();

        assert_eq!(config.get_api_key().unwrap(), "custom-key");
        assert_eq!(config.get_model(), "anthropic/claude-opus-4");
        assert_eq!(config.get_agents_dir(), PathBuf::from("custom/agents"));

        // Cleanup
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("MODEL");
        std::env::remove_var("AGENTS_DIR");
        std::env::remove_var("ACTIVE_AGENT_ID");
    }

    #[tokio::test]
    async fn test_get_active_agent_id() {
        let _lock = CONFIG_TEST_LOCK.lock().unwrap();

        std::env::set_var("OPENROUTER_API_KEY", "test-key");
        std::env::set_var("ACTIVE_AGENT_ID", "test-agent");

        let config = FileConfigService::load().unwrap();
        let active_id = config.get_active_agent_id().await.unwrap();

        assert_eq!(active_id, "test-agent");

        // Cleanup
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("ACTIVE_AGENT_ID");
    }
}
