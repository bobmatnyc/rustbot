// Mock test helpers and common mock patterns
//
// Design Decision: Centralized mock helpers for consistent testing
//
// Rationale: Provides reusable mock constructors with sensible defaults.
// Tests can override specific behaviors while inheriting baseline setup.
//
// Usage:
//     use crate::services::mocks::test_helpers::*;
//     let mut mock_fs = create_mock_filesystem();
//     mock_fs.expect_read_to_string()
//         .returning(|_| Ok("custom data".to_string()));

#[cfg(test)]
pub mod test_helpers {
    use super::super::traits::*;
    use crate::agent::AgentConfig;
    use mockall::predicate::*;
    use std::path::PathBuf;

    /// Create a mock filesystem with default "file not found" behavior
    ///
    /// Default behavior:
    /// - exists() returns false
    /// - read_to_string() returns IoError
    /// - write() succeeds
    /// - create_dir_all() succeeds
    /// - read_dir() returns empty vec
    pub fn create_mock_filesystem() -> MockFileSystem {
        let mut mock = MockFileSystem::new();

        // Default: file doesn't exist
        mock.expect_exists().returning(|_| false);

        mock
    }

    /// Create a mock filesystem that simulates existing files
    ///
    /// All operations succeed by default.
    /// Use this when testing happy path scenarios.
    pub fn create_existing_filesystem() -> MockFileSystem {
        let mut mock = MockFileSystem::new();

        // Default: file exists
        mock.expect_exists().returning(|_| true);

        // Default: successful read returns empty string
        mock.expect_read_to_string()
            .returning(|_| Ok(String::new()));

        // Default: writes succeed
        mock.expect_write().returning(|_, _| Ok(()));

        // Default: directory creation succeeds
        mock.expect_create_dir_all().returning(|_| Ok(()));

        // Default: read_dir returns empty
        mock.expect_read_dir().returning(|_| Ok(vec![]));

        mock
    }

    /// Create a mock storage service with default successful operations
    ///
    /// Default behavior:
    /// - load_token_stats() returns default TokenStats
    /// - save_token_stats() succeeds
    /// - load_system_prompts() returns default SystemPrompts
    /// - save_system_prompts() succeeds
    pub fn create_mock_storage() -> MockStorageService {
        let mut mock = MockStorageService::new();

        // Default: return empty stats
        mock.expect_load_token_stats()
            .returning(|| Ok(TokenStats::default()));

        mock.expect_save_token_stats().returning(|_| Ok(()));

        // Default: return empty prompts
        mock.expect_load_system_prompts()
            .returning(|| Ok(SystemPrompts::default()));

        mock.expect_save_system_prompts().returning(|_| Ok(()));

        mock
    }

    /// Create a mock config service with test defaults
    ///
    /// Default behavior:
    /// - load_agent_configs() returns empty vec
    /// - save_agent_config() succeeds
    /// - get_active_agent_id() returns "default"
    /// - set_active_agent_id() succeeds
    /// - get_agents_dir() returns PathBuf::from("agents")
    /// - get_api_key() returns "test-api-key"
    /// - get_model() returns "test-model"
    pub fn create_mock_config() -> MockConfigService {
        let mut mock = MockConfigService::new();

        // Default: return empty config
        mock.expect_load_agent_configs().returning(|| Ok(vec![]));

        mock.expect_save_agent_config().returning(|_| Ok(()));

        mock.expect_get_active_agent_id()
            .returning(|| Ok("default".to_string()));

        mock.expect_set_active_agent_id().returning(|_| Ok(()));

        mock.expect_get_agents_dir()
            .returning(|| PathBuf::from("agents"));

        mock.expect_get_api_key()
            .returning(|| Ok("test-api-key".to_string()));

        mock.expect_get_model()
            .returning(|| "test-model".to_string());

        mock
    }

    /// Create a mock config service with test agent configs
    ///
    /// Includes 2 test agents: "agent1" and "agent2"
    pub fn create_mock_config_with_agents() -> MockConfigService {
        let mut mock = MockConfigService::new();

        let agents = vec![
            AgentConfig {
                id: "agent1".to_string(),
                name: "Agent 1".to_string(),
                instructions: "Test instructions 1".to_string(),
                personality: None,
                model: "test-model".to_string(),
                enabled: true,
                is_primary: true,
                web_search_enabled: false,
                mcp_extensions: vec![],
            },
            AgentConfig {
                id: "agent2".to_string(),
                name: "Agent 2".to_string(),
                instructions: "Test instructions 2".to_string(),
                personality: None,
                model: "test-model".to_string(),
                enabled: true,
                is_primary: false,
                web_search_enabled: false,
                mcp_extensions: vec![],
            },
        ];

        mock.expect_load_agent_configs()
            .returning(move || Ok(agents.clone()));

        mock.expect_get_active_agent_id()
            .returning(|| Ok("agent1".to_string()));

        mock.expect_get_api_key()
            .returning(|| Ok("test-api-key".to_string()));

        mock.expect_get_model()
            .returning(|| "test-model".to_string());

        mock.expect_get_agents_dir()
            .returning(|| PathBuf::from("agents"));

        mock
    }

    /// Create a test AgentConfig
    pub fn create_test_agent_config(id: &str) -> AgentConfig {
        AgentConfig {
            id: id.to_string(),
            name: format!("Test Agent {}", id),
            instructions: format!("Test instructions for {}", id),
            personality: None,
            model: "test-model".to_string(),
            enabled: true,
            is_primary: false,
            web_search_enabled: false,
            mcp_extensions: vec![],
        }
    }

    /// Create test TokenStats with custom values
    pub fn create_test_token_stats(input: u64, output: u64, cost: f64) -> TokenStats {
        TokenStats {
            total_input_tokens: input,
            total_output_tokens: output,
            total_cost: cost,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Create test SystemPrompts
    pub fn create_test_system_prompts(base: &str, context: Option<&str>) -> SystemPrompts {
        SystemPrompts {
            base_prompt: base.to_string(),
            context: context.map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::traits::*;
    use super::test_helpers::*;

    #[test]
    fn test_create_mock_filesystem() {
        let mock = create_mock_filesystem();
        // Just verify it compiles and can be created
        drop(mock);
    }

    #[test]
    fn test_create_mock_storage() {
        let mock = create_mock_storage();
        drop(mock);
    }

    #[test]
    fn test_create_mock_config() {
        let mock = create_mock_config();
        drop(mock);
    }

    #[test]
    fn test_create_test_agent_config() {
        let config = create_test_agent_config("test");
        assert_eq!(config.id, "test");
        assert_eq!(config.name, "Test Agent test");
    }

    #[test]
    fn test_create_test_token_stats() {
        let stats = create_test_token_stats(100, 50, 0.05);
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);
        assert_eq!(stats.total_cost, 0.05);
    }
}
