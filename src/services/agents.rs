// Agent service implementation
//
// Design Decision: In-memory agent registry with lazy loading
//
// Rationale: Desktop application with small number of agents (<100):
// 1. Load all agents at startup into HashMap for O(1) lookup
// 2. Cache Agent instances to avoid repeated LLM adapter creation
// 3. Maintain current agent selection for quick switching
//
// Trade-offs:
// - Memory: In-memory registry vs. database (acceptable for <100 agents)
// - Startup time: Load all agents vs. lazy load (startup load simpler)
// - Consistency: Cached agents vs. reload on each access (cache better UX)
//
// Extension Points: Can add database backend or agent reloading
// for hot config updates (implement new AgentService adapter).

use super::traits::{AgentService, ConfigService};
use crate::agent::Agent;
use crate::error::{Result, RustbotError};
use crate::events::EventBus;
use crate::llm::OpenRouterAdapter;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Default agent service implementation
///
/// Manages the agent registry and provides access to loaded agents.
/// Agents are loaded from ConfigService and cached in memory.
///
/// Thread Safety: Uses Arc for shared Agent instances.
/// Agent switching requires &mut self (not thread-safe), should be
/// done from single UI thread.
///
/// Usage:
///     let config_service = Arc::new(FileConfigService::load()?);
///     let service = DefaultAgentService::new(config_service, ...).await?;
///     let agent = service.get_agent("researcher").await?;
pub struct DefaultAgentService {
    /// Loaded agents by ID
    agents: HashMap<String, Arc<Agent>>,

    /// Currently active agent ID
    active_agent_id: String,

    /// Configuration service for loading agents
    config_service: Arc<dyn ConfigService>,

    /// Event bus for agent communication
    event_bus: Arc<EventBus>,

    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,
}

impl DefaultAgentService {
    /// Create a new agent service
    ///
    /// Loads all agents from config service and initializes the registry.
    ///
    /// # Arguments
    /// * `config_service` - Configuration service providing agent configs
    /// * `event_bus` - Event bus for agent communication
    /// * `runtime` - Tokio runtime for async operations
    /// * `system_instructions` - System-level instructions for all agents
    ///
    /// # Errors
    /// - Agent loading errors
    /// - No agents found
    /// - Active agent not found in loaded agents
    pub async fn new(
        config_service: Arc<dyn ConfigService>,
        event_bus: Arc<EventBus>,
        runtime: Arc<Runtime>,
        system_instructions: String,
    ) -> Result<Self> {
        // Load agent configs
        let agent_configs = config_service.load_agent_configs().await?;

        if agent_configs.is_empty() {
            return Err(RustbotError::ConfigError(
                "No agents loaded from configuration".to_string()
            ));
        }

        // Get API key for LLM adapter
        let api_key = config_service.get_api_key()?;

        // Create agents
        let mut agents = HashMap::new();

        for config in agent_configs {
            // Create LLM adapter (currently using OpenRouter for all)
            let llm_adapter = Arc::new(OpenRouterAdapter::new(api_key.clone()));

            // Create agent
            let agent = Arc::new(Agent::new(
                config.clone(),
                llm_adapter,
                Arc::clone(&event_bus),
                Arc::clone(&runtime),
                system_instructions.clone(),
            ));

            agents.insert(config.id.clone(), agent);
        }

        // Get active agent ID (or default to first agent)
        let active_agent_id = config_service.get_active_agent_id().await
            .unwrap_or_else(|_| {
                agents.keys().next()
                    .expect("At least one agent should exist")
                    .clone()
            });

        // Verify active agent exists
        if !agents.contains_key(&active_agent_id) {
            return Err(RustbotError::AgentNotFound(active_agent_id));
        }

        Ok(Self {
            agents,
            active_agent_id,
            config_service,
            event_bus,
            runtime,
        })
    }
}

#[async_trait]
impl AgentService for DefaultAgentService {
    async fn get_agent(&self, id: &str) -> Result<Arc<Agent>> {
        self.agents.get(id)
            .cloned()
            .ok_or_else(|| RustbotError::AgentNotFound(id.to_string()))
    }

    fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    async fn switch_agent(&mut self, id: &str) -> Result<()> {
        if !self.agents.contains_key(id) {
            return Err(RustbotError::AgentNotFound(id.to_string()));
        }

        self.active_agent_id = id.to_string();

        // Note: Persisting active agent not yet implemented
        // Future enhancement: config_service.set_active_agent_id(id).await?;

        Ok(())
    }

    fn current_agent(&self) -> Arc<Agent> {
        self.agents.get(&self.active_agent_id)
            .expect("Active agent should always exist")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentConfig;
    use crate::services::traits::ConfigService;
    use std::path::PathBuf;

    // Mock ConfigService for testing
    struct MockConfigService {
        agents: Vec<AgentConfig>,
        api_key: String,
    }

    #[async_trait]
    impl ConfigService for MockConfigService {
        async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>> {
            Ok(self.agents.clone())
        }

        async fn save_agent_config(&self, _config: &AgentConfig) -> Result<()> {
            Ok(())
        }

        async fn get_active_agent_id(&self) -> Result<String> {
            Ok("agent1".to_string())
        }

        async fn set_active_agent_id(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        fn get_agents_dir(&self) -> PathBuf {
            PathBuf::from("agents")
        }

        fn get_api_key(&self) -> Result<String> {
            Ok(self.api_key.clone())
        }

        fn get_model(&self) -> String {
            "test-model".to_string()
        }
    }

    fn create_test_agent_config(id: &str) -> AgentConfig {
        AgentConfig {
            id: id.to_string(),
            name: format!("Agent {}", id),
            instructions: "Test instructions".to_string(),
            personality: None,
            model: "test-model".to_string(),
            enabled: true,
            is_primary: id == "agent1",
            web_search_enabled: false,
            mcp_extensions: vec![],
        }
    }

    #[test]
    fn test_agent_service_creation() {
        // Create a runtime for testing
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![
                    create_test_agent_config("agent1"),
                    create_test_agent_config("agent2"),
                ],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let service = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await.unwrap();

            assert_eq!(service.list_agents().len(), 2);
            assert!(service.list_agents().contains(&"agent1".to_string()));
            assert!(service.list_agents().contains(&"agent2".to_string()));
        });
    }

    #[test]
    fn test_get_agent() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![create_test_agent_config("agent1")],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let service = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await.unwrap();

            let agent = service.get_agent("agent1").await.unwrap();
            assert_eq!(agent.id(), "agent1");
        });
    }

    #[test]
    fn test_get_nonexistent_agent() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![create_test_agent_config("agent1")],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let service = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await.unwrap();

            let result = service.get_agent("nonexistent").await;
            assert!(result.is_err());

            match result {
                Err(RustbotError::AgentNotFound(id)) => {
                    assert_eq!(id, "nonexistent");
                }
                _ => panic!("Expected AgentNotFound error"),
            }
        });
    }

    #[test]
    fn test_switch_agent() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![
                    create_test_agent_config("agent1"),
                    create_test_agent_config("agent2"),
                ],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let mut service = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await.unwrap();

            // Initially on agent1
            assert_eq!(service.current_agent().id(), "agent1");

            // Switch to agent2
            service.switch_agent("agent2").await.unwrap();
            assert_eq!(service.current_agent().id(), "agent2");
        });
    }

    #[test]
    fn test_current_agent() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![create_test_agent_config("agent1")],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let service = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await.unwrap();

            let current = service.current_agent();
            assert_eq!(current.id(), "agent1");
        });
    }

    #[test]
    fn test_empty_agents_error() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            let mock_config = Arc::new(MockConfigService {
                agents: vec![],
                api_key: "test-key".to_string(),
            });

            let event_bus = Arc::new(EventBus::new());
            let runtime = Arc::new(Runtime::new().unwrap());

            let result = DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "System instructions".to_string(),
            ).await;

            assert!(result.is_err());
            match result {
                Err(RustbotError::ConfigError(msg)) => {
                    assert!(msg.contains("No agents loaded"));
                }
                _ => panic!("Expected ConfigError"),
            }
        });
    }
}
