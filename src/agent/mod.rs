// Agent framework for managing AI agents with their own instructions and personality
// Each agent has its own LLM instance and communicates via the event system
//
// Module Organization:
// - config.rs: JSON-based agent configuration with multi-provider LLM support
// - loader.rs: Directory-based agent discovery and loading
// - tools.rs: Tool definitions for OpenAI-compatible function calling
// - Core Agent and AgentConfig types defined in this file
//
// Design Decision: Hybrid module structure
// - Maintains existing AgentConfig for backward compatibility
// - New JsonAgentConfig for JSON-based configuration
// - AgentLoader converts JsonAgentConfig → AgentConfig
//
// This approach allows incremental migration while preserving existing APIs.

pub mod config;
pub mod loader;
pub mod tools;

use crate::events::{AgentStatus, Event, EventBus, EventKind};
use crate::llm::{LlmAdapter, LlmRequest, Message as LlmMessage, ToolCall};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

// Re-export JSON configuration types
pub use config::{AgentCapabilities, AgentMetadata, JsonAgentConfig, ModelParameters};
pub use loader::AgentLoader;
pub use tools::{FunctionDefinition, FunctionParameters, ToolDefinition};

/// Runtime configuration for an agent
///
/// This is the configuration used by the Agent runtime. It can be created:
/// 1. Directly via AgentConfig::new() for programmatic usage
/// 2. Via AgentConfig::default_assistant() for the default assistant
/// 3. Via AgentLoader which loads from JSON and converts to AgentConfig
///
/// Design: Separate from JsonAgentConfig to maintain backward compatibility
/// and allow runtime-optimized structure (e.g., pre-built system prompts).
///
/// Agent Types:
/// - Primary Agent (is_primary = true): Always active, handles all user messages
/// - Specialist Agent (is_primary = false): Callable by primary agent when enabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier for this agent
    pub id: String,

    /// Display name for the agent
    pub name: String,

    /// Agent-specific instructions (what the agent does)
    pub instructions: String,

    /// Agent personality (how the agent behaves) - optional, agent-specific
    pub personality: Option<String>,

    /// LLM model to use for this agent
    pub model: String,

    /// Whether this agent is currently enabled (can be called by primary agent)
    pub enabled: bool,

    /// Whether this is the primary agent (handles user messages directly)
    /// Primary agent is always active. Non-primary agents are callable tools.
    #[serde(default)]
    pub is_primary: bool,

    /// Whether this agent has web search capabilities enabled
    #[serde(default)]
    pub web_search_enabled: bool,

    /// MCP extensions enabled for this agent
    /// List of extension IDs (e.g., "ai.exa/exa") that should be loaded
    /// and made available as tools for this agent.
    #[serde(default)]
    pub mcp_extensions: Vec<String>,
}

impl AgentConfig {
    /// Create a new agent configuration
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            instructions: String::new(),
            personality: None,
            model: "openai/gpt-4o".to_string(),
            enabled: true,
            is_primary: false, // Default to specialist agent
            web_search_enabled: false,
            mcp_extensions: Vec::new(),
        }
    }

    /// Create the default assistant agent with intent detection
    pub fn default_assistant() -> Self {
        Self {
            id: "assistant".to_string(),
            name: "Assistant".to_string(),
            instructions: Self::build_assistant_instructions(),
            personality: Some("Be concise, friendly, and professional.".to_string()),
            model: "openai/gpt-4o".to_string(),
            enabled: true,
            is_primary: true, // Assistant is the primary agent
            web_search_enabled: false,
            mcp_extensions: Vec::new(),
        }
    }

    /// Build assistant-specific instructions with intent detection
    fn build_assistant_instructions() -> String {
        r#"You are a helpful AI assistant with access to specialized capabilities.

## Intent Detection

Analyze each user message to detect their intent:

**Web Search Intent** - User needs current/recent information:
- "What's the weather in..."
- "Latest news about..."
- "Current price of..."
- "Who won the game..."
- "What happened recently..."
- Questions about events after your knowledge cutoff
- Keywords: "latest", "current", "today", "recent", "now", "this week"

**Direct Response** - You can answer directly:
- General knowledge questions within your training
- Explanations of concepts
- Advice and recommendations
- Conversational queries
- Questions about historical facts
- Technical explanations

## Response Protocol

When you detect **web search intent**:
1. Acknowledge that you'll search for current information
2. Note: "I'll search for that information..." (future: call web_search agent)
3. For now, explain what you would search for and why

For **direct response** intents:
- Answer immediately using your knowledge
- Be clear and helpful
- Cite sources if making factual claims

## Guidelines

- Always prefer direct responses when you have reliable knowledge
- Use web search for time-sensitive or recent information
- Be transparent about your limitations
- If unsure, explain what information you'd need to search for"#
            .to_string()
    }
}

/// Response from agent message processing
///
/// Two-phase execution pattern to avoid circular dependencies:
/// - Agent detects tool calls and returns them to caller
/// - Caller (RustbotApi) executes tools and makes follow-up request
pub enum AgentResponse {
    /// Normal streaming response - no tools needed
    StreamingResponse(mpsc::UnboundedReceiver<String>),

    /// Tool calls detected - need execution before final response
    NeedsToolExecution {
        /// Tool calls requested by the LLM
        tool_calls: Vec<ToolCall>,
        /// Conversation history including the assistant's tool call message
        messages: Vec<LlmMessage>,
    },
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

    /// Tokio runtime handle for async operations
    runtime: tokio::runtime::Handle,

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
        runtime: tokio::runtime::Handle,
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
            parts.push(format!(
                "## Agent Instructions\n\n{}",
                self.config.instructions
            ));
        }

        // Add agent personality (only if defined for this agent)
        if let Some(personality) = &self.config.personality {
            if !personality.is_empty() {
                parts.push(format!("## Agent Personality\n\n{}", personality));
            }
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
            api_messages.push(LlmMessage::new("system", system_content));
        }

        // Add conversation context
        api_messages.extend(context_messages);

        // Add current user message
        api_messages.push(LlmMessage::new("user", user_message));

        // Create request with web search if enabled for this agent
        let mut request = LlmRequest::new(api_messages);
        request.web_search = Some(self.config.web_search_enabled);

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

    /// Process a message without blocking - returns a channel to poll for the result
    /// This spawns the async work in the background and immediately returns
    ///
    /// Two-phase execution pattern for tool calls:
    /// 1. If tools provided, use complete_chat to detect tool calls
    /// 2. Return AgentResponse::NeedsToolExecution if tools detected
    /// 3. Caller executes tools and calls process_with_results
    /// 4. Finally stream the response
    ///
    /// # Arguments
    /// * `user_message` - The message to process
    /// * `context_messages` - Previous conversation messages for context
    /// * `tools` - Optional tool definitions (for primary agent delegation)
    pub fn process_message_nonblocking(
        &self,
        user_message: String,
        context_messages: Vec<LlmMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> mpsc::UnboundedReceiver<Result<AgentResponse>> {
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Clone everything we need for the async task
        let llm_adapter = Arc::clone(&self.llm_adapter);
        let system_instructions = self.system_instructions.clone();
        let config_instructions = self.config.instructions.clone();
        let config_personality = self.config.personality.clone();
        let web_search_enabled = self.config.web_search_enabled;
        let runtime = self.runtime.clone();
        let agent_id = self.config.id.clone();
        let event_bus = Arc::clone(&self.event_bus);

        // Spawn async task
        runtime.spawn(async move {
            let agent_start = std::time::Instant::now();
            tracing::debug!("⏱️  [AGENT] Processing started");

            // Build system message
            let mut parts = Vec::new();
            if !system_instructions.is_empty() {
                parts.push(system_instructions);
            }
            if !config_instructions.is_empty() {
                parts.push(format!("## Agent Instructions\n\n{}", config_instructions));
            }
            // Only include personality if defined
            if let Some(personality) = config_personality {
                if !personality.is_empty() {
                    parts.push(format!("## Agent Personality\n\n{}", personality));
                }
            }
            let system_content = parts.join("\n\n");

            tracing::debug!(
                "⏱️  [AGENT] System message built in {:?}",
                agent_start.elapsed()
            );

            // Build complete message history
            let mut api_messages = Vec::new();
            if !system_content.is_empty() {
                api_messages.push(LlmMessage::new("system", system_content));
            }
            api_messages.extend(context_messages);
            api_messages.push(LlmMessage::new("user", user_message));

            // Create request with web search if enabled
            let mut request = LlmRequest::new(api_messages.clone());
            request.web_search = Some(web_search_enabled);

            let result = if let Some(tool_defs) = tools {
                // Tools provided - use complete_chat to detect tool calls
                request.tools = Some(tool_defs);
                request.tool_choice = Some("auto".to_string());

                tracing::info!("Processing message with tools enabled");
                tracing::debug!(
                    "⏱️  [AGENT] Starting complete_chat (non-streaming) at {:?}",
                    agent_start.elapsed()
                );

                // Use complete_chat for tool detection
                match llm_adapter.complete_chat(request).await {
                    Ok(response) => {
                        tracing::debug!(
                            "⏱️  [AGENT] complete_chat finished at {:?}",
                            agent_start.elapsed()
                        );

                        if let Some(tool_calls) = response.tool_calls {
                            // Tool calls detected - return for execution
                            tracing::info!("Tool calls detected: {} calls", tool_calls.len());

                            // Add assistant message with tool calls to history
                            // Anthropic requires non-empty content, so use placeholder if needed
                            let content = if response.content.is_empty() {
                                "I'll use the available tools to help with that.".to_string()
                            } else {
                                response.content
                            };

                            api_messages
                                .push(LlmMessage::with_tool_calls(content, tool_calls.clone()));

                            Ok(AgentResponse::NeedsToolExecution {
                                tool_calls,
                                messages: api_messages,
                            })
                        } else {
                            // No tool calls - stream the response we already got
                            tracing::info!("No tool calls detected, streaming response");

                            let (tx, rx) = mpsc::unbounded_channel();
                            let _ = tx.send(response.content);

                            // Publish responding status
                            let event = Event::new(
                                agent_id.clone(),
                                "broadcast".to_string(),
                                EventKind::AgentStatusChange {
                                    agent_id: agent_id.clone(),
                                    status: AgentStatus::Responding,
                                },
                            );
                            let _ = event_bus.publish(event);

                            Ok(AgentResponse::StreamingResponse(rx))
                        }
                    }
                    Err(e) => {
                        tracing::error!("Tool-enabled complete_chat failed: {}", e);
                        Err(e)
                    }
                }
            } else {
                // No tools - use streaming as before
                tracing::info!("Processing message without tools, using stream_chat");
                tracing::debug!(
                    "⏱️  [AGENT] Starting stream_chat at {:?}",
                    agent_start.elapsed()
                );

                let (tx, rx) = mpsc::unbounded_channel();

                match llm_adapter.stream_chat(request, tx).await {
                    Ok(_) => {
                        // Publish responding status
                        let event = Event::new(
                            agent_id.clone(),
                            "broadcast".to_string(),
                            EventKind::AgentStatusChange {
                                agent_id: agent_id.clone(),
                                status: AgentStatus::Responding,
                            },
                        );
                        let _ = event_bus.publish(event);

                        Ok(AgentResponse::StreamingResponse(rx))
                    }
                    Err(e) => {
                        tracing::error!("stream_chat failed: {}", e);
                        Err(e)
                    }
                }
            };

            // Handle errors and publish status
            if let Err(ref e) = result {
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

            let _ = result_tx.send(result);
        });

        result_rx
    }

    /// Process a follow-up request with tool results
    /// Used after tools have been executed to get the final response
    pub fn process_with_results(
        &self,
        messages_with_tool_results: Vec<LlmMessage>,
    ) -> mpsc::UnboundedReceiver<Result<mpsc::UnboundedReceiver<String>>> {
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Clone everything we need
        let llm_adapter = Arc::clone(&self.llm_adapter);
        let web_search_enabled = self.config.web_search_enabled;
        let runtime = self.runtime.clone();
        let agent_id = self.config.id.clone();
        let event_bus = Arc::clone(&self.event_bus);

        runtime.spawn(async move {
            // Create request with the updated message history (includes tool results)
            let mut request = LlmRequest::new(messages_with_tool_results);
            request.web_search = Some(web_search_enabled);

            let (tx, rx) = mpsc::unbounded_channel();

            let result = llm_adapter.stream_chat(request, tx).await;

            let final_result = match result {
                Ok(_) => {
                    // Publish responding status
                    let event = Event::new(
                        agent_id.clone(),
                        "broadcast".to_string(),
                        EventKind::AgentStatusChange {
                            agent_id: agent_id.clone(),
                            status: AgentStatus::Responding,
                        },
                    );
                    let _ = event_bus.publish(event);

                    Ok(rx)
                }
                Err(e) => {
                    tracing::error!("stream_chat with tool results failed: {}", e);

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

                    Err(e)
                }
            };

            let _ = result_tx.send(final_result);
        });

        result_rx
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
        assert_eq!(config.model, "openai/gpt-4o");
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_build_system_message() {
        let runtime = tokio::runtime::Handle::current();
        let event_bus = Arc::new(EventBus::new());

        // Create a mock adapter (we won't actually use it in this test)
        use crate::llm::OpenRouterAdapter;
        let adapter = Arc::new(OpenRouterAdapter::new("test-key".to_string()));

        let mut config = AgentConfig::default_assistant();
        config.instructions = "You are a helpful assistant.".to_string();
        config.personality = Some("Be friendly and concise.".to_string());

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
