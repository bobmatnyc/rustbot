// Agent framework for managing AI agents with their own instructions and personality
// Each agent has its own LLM instance and communicates via the event system

use crate::events::{Event, EventBus, EventKind, AgentStatus};
use crate::llm::{LlmAdapter, LlmRequest, Message as LlmMessage};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::runtime::Runtime;

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier for this agent
    pub id: String,

    /// Display name for the agent
    pub name: String,

    /// Agent-specific instructions (what the agent does)
    pub instructions: String,

    /// Agent personality (how the agent behaves)
    pub personality: String,

    /// LLM model to use for this agent
    pub model: String,

    /// Whether this agent is currently enabled
    pub enabled: bool,
}

impl AgentConfig {
    /// Create a new agent configuration
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            instructions: String::new(),
            personality: String::new(),
            model: "anthropic/claude-sonnet-4.5".to_string(),
            enabled: true,
        }
    }

    /// Create the default assistant agent
    pub fn default_assistant() -> Self {
        Self {
            id: "assistant".to_string(),
            name: "Assistant".to_string(),
            instructions: String::new(),
            personality: String::new(),
            model: "anthropic/claude-sonnet-4.5".to_string(),
            enabled: true,
        }
    }
}

/// An AI agent that processes messages and responds via the event system
pub struct Agent {
    /// Agent configuration
    config: AgentConfig,

    /// LLM adapter for this agent
    llm_adapter: Arc<dyn LlmAdapter>,

    /// Event bus for communication
    event_bus: Arc<EventBus>,

    /// Receiver for events
    event_rx: broadcast::Receiver<Event>,

    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,

    /// System-level instructions (shared across all agents)
    system_instructions: String,

    /// Current status of the agent
    status: AgentStatus,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        config: AgentConfig,
        llm_adapter: Arc<dyn LlmAdapter>,
        event_bus: Arc<EventBus>,
        runtime: Arc<Runtime>,
        system_instructions: String,
    ) -> Self {
        let event_rx = event_bus.subscribe();

        Self {
            config,
            llm_adapter,
            event_bus,
            event_rx,
            runtime,
            system_instructions,
            status: AgentStatus::Idle,
        }
    }

    /// Get the agent's ID
    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// Get the agent's name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get the agent's current status
    pub fn status(&self) -> &AgentStatus {
        &self.status
    }

    /// Update the agent's status and publish status change event
    fn set_status(&mut self, status: AgentStatus) {
        self.status = status.clone();

        let event = Event::new(
            self.config.id.clone(),
            "broadcast".to_string(),
            EventKind::AgentStatusChange {
                agent_id: self.config.id.clone(),
                status,
            },
        );

        let _ = self.event_bus.publish(event);
    }

    /// Build the complete system message for this agent
    fn build_system_message(&self) -> String {
        let mut parts = Vec::new();

        // Add system-level instructions (shared)
        if !self.system_instructions.is_empty() {
            parts.push(self.system_instructions.clone());
        }

        // Add agent-specific instructions
        if !self.config.instructions.is_empty() {
            parts.push(format!("## Agent Instructions\n\n{}", self.config.instructions));
        }

        // Add agent personality
        if !self.config.personality.is_empty() {
            parts.push(format!("## Agent Personality\n\n{}", self.config.personality));
        }

        parts.join("\n\n")
    }

    /// Process a user message and generate a response
    pub async fn process_message(
        &mut self,
        user_message: String,
        context_messages: Vec<LlmMessage>,
    ) -> Result<mpsc::UnboundedReceiver<String>> {
        // Update status to thinking
        self.set_status(AgentStatus::Thinking);

        // Build complete message history
        let mut api_messages = Vec::new();

        // Add system message
        let system_content = self.build_system_message();
        if !system_content.is_empty() {
            api_messages.push(LlmMessage {
                role: "system".to_string(),
                content: system_content,
            });
        }

        // Add conversation context
        api_messages.extend(context_messages);

        // Add current user message
        api_messages.push(LlmMessage {
            role: "user".to_string(),
            content: user_message,
        });

        // Create request
        let request = LlmRequest::new(api_messages);

        // Update status to responding
        self.set_status(AgentStatus::Responding);

        // Create channel for streaming response
        let (tx, rx) = mpsc::unbounded_channel();

        // Clone adapter for async task
        let adapter = Arc::clone(&self.llm_adapter);
        let agent_id = self.config.id.clone();
        let event_bus = Arc::clone(&self.event_bus);

        // Spawn async task to stream response
        self.runtime.spawn(async move {
            match adapter.stream_chat(request, tx).await {
                Ok(_) => {
                    // Publish agent idle status when done
                    let event = Event::new(
                        agent_id.clone(),
                        "broadcast".to_string(),
                        EventKind::AgentStatusChange {
                            agent_id,
                            status: AgentStatus::Idle,
                        },
                    );
                    let _ = event_bus.publish(event);
                }
                Err(e) => {
                    tracing::error!("Agent LLM stream error: {}", e);
                    // Publish error status
                    let event = Event::new(
                        agent_id.clone(),
                        "broadcast".to_string(),
                        EventKind::AgentStatusChange {
                            agent_id,
                            status: AgentStatus::Error(e.to_string()),
                        },
                    );
                    let _ = event_bus.publish(event);
                }
            }
        });

        Ok(rx)
    }

    /// Check if this agent should handle an event
    pub fn should_handle_event(&self, event: &Event) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Handle events targeted to this agent or broadcast
        event.is_for(&self.config.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new("test".to_string(), "Test Agent".to_string());
        assert_eq!(config.id, "test");
        assert_eq!(config.name, "Test Agent");
        assert!(config.enabled);
    }

    #[test]
    fn test_default_assistant_config() {
        let config = AgentConfig::default_assistant();
        assert_eq!(config.id, "assistant");
        assert_eq!(config.name, "Assistant");
        assert_eq!(config.model, "anthropic/claude-sonnet-4.5");
        assert!(config.enabled);
    }

    #[test]
    fn test_build_system_message() {
        let runtime = Arc::new(Runtime::new().unwrap());
        let event_bus = Arc::new(EventBus::new());

        // Create a mock adapter (we won't actually use it in this test)
        use crate::llm::OpenRouterAdapter;
        let adapter = Arc::new(OpenRouterAdapter::new("test-key".to_string()));

        let mut config = AgentConfig::default_assistant();
        config.instructions = "You are a helpful assistant.".to_string();
        config.personality = "Be friendly and concise.".to_string();

        let agent = Agent::new(
            config,
            adapter,
            event_bus,
            runtime,
            "System context here.".to_string(),
        );

        let system_msg = agent.build_system_message();
        assert!(system_msg.contains("System context here."));
        assert!(system_msg.contains("You are a helpful assistant."));
        assert!(system_msg.contains("Be friendly and concise."));
    }
}
