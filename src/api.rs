// API layer for programmatic access to all Rustbot functionality
// This provides the core interface that both UI and external code can use
// Design principle: All functionality accessible programmatically

use crate::agent::{Agent, AgentConfig, AgentResponse, ToolDefinition};
use crate::events::{Event, EventBus, EventKind, AgentStatus};
use crate::llm::{Message as LlmMessage, LlmAdapter};
use crate::tool_executor::ToolExecutor;
use anyhow::{Result, Context as AnyhowContext};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::runtime::Runtime;

/// Core API for Rustbot functionality
/// All user actions should have equivalent API methods here
pub struct RustbotApi {
    /// Event bus for pub/sub communication
    event_bus: Arc<EventBus>,

    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,

    /// Registered agents
    agents: Vec<Agent>,

    /// Agent configurations (needed for building tool registry)
    agent_configs: Vec<AgentConfig>,

    /// Tool definitions for enabled specialist agents
    /// These are automatically built from agent_configs
    available_tools: Vec<ToolDefinition>,

    /// Currently active agent ID
    active_agent_id: String,

    /// Message history (for context)
    message_history: VecDeque<LlmMessage>,

    /// Maximum messages to keep in history
    max_history_size: usize,
}

impl RustbotApi {
    /// Create a new API instance
    pub fn new(
        event_bus: Arc<EventBus>,
        runtime: Arc<Runtime>,
        max_history_size: usize,
    ) -> Self {
        Self {
            event_bus,
            runtime,
            agents: Vec::new(),
            agent_configs: Vec::new(),
            available_tools: Vec::new(),
            active_agent_id: String::from("assistant"),
            message_history: VecDeque::new(),
            max_history_size,
        }
    }

    /// Build tool definitions from all enabled specialist agents
    /// This should be called whenever agent configs change (enable/disable)
    fn build_tool_definitions(&self) -> Vec<ToolDefinition> {
        tracing::info!("🔍 [DEBUG] build_tool_definitions called with {} agent configs", self.agent_configs.len());

        // 🔍 DEBUG: Log which configs are enabled specialists
        for config in &self.agent_configs {
            if config.enabled && !config.is_primary {
                tracing::info!(
                    "🔍 [DEBUG] Enabled specialist: id='{}', name='{}'",
                    config.id,
                    config.name
                );
            }
        }

        let tools = ToolDefinition::from_agents(&self.agent_configs);
        tracing::info!("🔍 [DEBUG] build_tool_definitions returning {} tools", tools.len());
        tools
    }

    /// Update the tool registry
    /// Call this when agents are enabled/disabled to rebuild the available tools
    pub fn update_tools(&mut self) {
        tracing::info!("🔍 [DEBUG] update_tools called");
        self.available_tools = self.build_tool_definitions();
        tracing::info!(
            "🔍 [DEBUG] Tool registry updated: {} tools available",
            self.available_tools.len()
        );

        // 🔍 DEBUG: Log tool names
        if !self.available_tools.is_empty() {
            let tool_names: Vec<&str> = self.available_tools
                .iter()
                .map(|t| t.function.name.as_str())
                .collect();
            tracing::info!("🔍 [DEBUG] Tools after update: {:?}", tool_names);
        } else {
            tracing::warn!("🔍 [DEBUG] WARNING: No tools available after update!");
        }
    }

    /// Get the current list of available tools
    pub fn available_tools(&self) -> &[ToolDefinition] {
        &self.available_tools
    }

    /// Register an agent with the system
    /// This makes the agent available for message processing
    pub fn register_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    /// Get list of all registered agent IDs
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.iter().map(|a| a.id().to_string()).collect()
    }

    /// Get the currently active agent ID
    pub fn active_agent(&self) -> &str {
        &self.active_agent_id
    }

    /// Switch to a different agent
    /// Returns error if agent ID doesn't exist
    pub fn switch_agent(&mut self, agent_id: &str) -> Result<()> {
        if !self.agents.iter().any(|a| a.id() == agent_id) {
            anyhow::bail!("Agent '{}' not found", agent_id);
        }

        self.active_agent_id = agent_id.to_string();

        // Publish agent switch event
        let event = Event::new(
            "system".to_string(),
            "broadcast".to_string(),
            EventKind::AgentStatusChange {
                agent_id: agent_id.to_string(),
                status: AgentStatus::Idle,
            },
        );
        self.event_bus.publish(event)?;

        Ok(())
    }

    /// Send a user message and get a streaming response
    /// This is the programmatic equivalent of typing a message in the UI
    /// Returns a channel that will stream the agent's response chunks
    pub async fn send_message(
        &mut self,
        message: &str,
    ) -> Result<mpsc::UnboundedReceiver<String>> {
        let start_time = std::time::Instant::now();
        tracing::debug!("⏱️  [PERF] send_message started");

        // 🔍 DEBUG: Check tool state at start of send_message
        tracing::info!(
            "🔍 [DEBUG] send_message called - available_tools.len() = {}, agent_configs.len() = {}, active_agent_id = '{}'",
            self.available_tools.len(),
            self.agent_configs.len(),
            self.active_agent_id
        );

        // 🔍 DEBUG: Log all available tool names
        if !self.available_tools.is_empty() {
            let tool_names: Vec<&str> = self.available_tools
                .iter()
                .map(|t| t.function.name.as_str())
                .collect();
            tracing::info!("🔍 [DEBUG] Available tools: {:?}", tool_names);
        } else {
            tracing::warn!("🔍 [DEBUG] WARNING: available_tools is EMPTY!");
        }

        // 🔍 DEBUG: Log all agent config IDs and their isPrimary status
        for config in &self.agent_configs {
            tracing::info!(
                "🔍 [DEBUG] Agent config: id='{}', name='{}', isPrimary={}, enabled={}",
                config.id,
                config.name,
                config.is_primary,
                config.enabled
            );
        }

        // Get context messages (last N messages) - WITHOUT adding current message yet
        // The agent will receive the current message separately and add it to context
        let context_messages: Vec<LlmMessage> = self.message_history
            .iter()
            .take(self.max_history_size)
            .cloned()
            .collect();

        tracing::debug!("⏱️  [PERF] Context prepared in {:?}", start_time.elapsed());

        // OPTIMIZATION: Publish immediate "thinking" status for better perceived performance
        // This provides instant feedback to the user before we wait for the LLM response
        let _ = self.event_bus.publish(Event::new(
            "system".to_string(),
            "broadcast".to_string(),
            EventKind::AgentStatusChange {
                agent_id: self.active_agent_id.clone(),
                status: AgentStatus::Thinking,
            },
        ));
        tracing::debug!("⏱️  [PERF] Published thinking status at {:?}", start_time.elapsed());

        // Find active agent
        let agent = self.agents
            .iter()
            .find(|a| a.id() == self.active_agent_id)
            .context("Active agent not found")?;

        // Determine if we should pass tools (only for primary agent)
        tracing::info!("🔍 [DEBUG] Looking for agent config with id = '{}'", self.active_agent_id);

        let agent_config = self.agent_configs
            .iter()
            .find(|c| c.id == self.active_agent_id);

        // 🔍 DEBUG: Log agent config lookup result
        match agent_config {
            Some(config) => {
                tracing::info!(
                    "🔍 [DEBUG] Found agent config: id='{}', isPrimary={}, enabled={}",
                    config.id,
                    config.is_primary,
                    config.enabled
                );
            }
            None => {
                tracing::error!("🔍 [DEBUG] CRITICAL: No agent config found for active_agent_id='{}'!", self.active_agent_id);
            }
        }

        let tools = if let Some(config) = agent_config {
            if config.is_primary {
                // Primary agent gets access to all enabled specialist tools
                tracing::info!(
                    "🔍 [DEBUG] Agent is PRIMARY, cloning {} tools",
                    self.available_tools.len()
                );
                Some(self.available_tools.clone())
            } else {
                // Specialist agents don't get tools
                tracing::info!("🔍 [DEBUG] Agent is NOT primary, no tools");
                None
            }
        } else {
            tracing::warn!("🔍 [DEBUG] No agent config found, no tools will be passed");
            None
        };

        // Log tool count if tools are being passed
        if let Some(ref tool_list) = tools {
            tracing::debug!(
                "Passing {} tools to primary agent: {:?}",
                tool_list.len(),
                tool_list.iter().map(|t| &t.function.name).collect::<Vec<_>>()
            );
        }

        // DIAGNOSTIC: Log tool passing status at INFO level for debugging
        if let Some(ref tool_list) = tools {
            tracing::info!(
                "🔧 [API] Passing {} tools to agent '{}': {:?}",
                tool_list.len(),
                self.active_agent_id,
                tool_list.iter().map(|t| &t.function.name).collect::<Vec<_>>()
            );
        } else {
            tracing::info!("🔧 [API] No tools passed to agent '{}'", self.active_agent_id);
        }

        // Process message through agent (non-blocking)
        tracing::debug!("⏱️  [PERF] Starting agent processing at {:?}", start_time.elapsed());
        let mut result_rx = agent.process_message_nonblocking(
            message.to_string(),
            context_messages,
            tools,
        );

        // Add user message to history AFTER sending to agent
        // This ensures the next message will have this one as context
        let user_msg = LlmMessage::new("user", message);
        tracing::debug!("📝 [HISTORY] Adding USER message - content_len: {}, total_history: {}",
            user_msg.content.len(), self.message_history.len() + 1);
        self.message_history.push_back(user_msg);

        // Trim history if needed
        while self.message_history.len() > self.max_history_size {
            self.message_history.pop_front();
        }

        // Wait for the agent response and handle tool execution if needed
        tracing::debug!("⏱️  [PERF] Waiting for agent response at {:?}", start_time.elapsed());
        let agent_response_result = result_rx.recv().await
            .context("No response from agent")?;

        tracing::debug!("⏱️  [PERF] Agent response received at {:?}", start_time.elapsed());

        match agent_response_result {
            Ok(AgentResponse::StreamingResponse(stream)) => {
                // No tools needed - return stream directly
                tracing::debug!("⏱️  [PERF] Streaming response started at {:?}", start_time.elapsed());

                // Publish responding status
                let _ = self.event_bus.publish(Event::new(
                    "system".to_string(),
                    "broadcast".to_string(),
                    EventKind::AgentStatusChange {
                        agent_id: self.active_agent_id.clone(),
                        status: AgentStatus::Responding,
                    },
                ));

                Ok(stream)
            }
            Ok(AgentResponse::NeedsToolExecution { tool_calls, mut messages }) => {
                tracing::info!("Tool execution required: {} tools to execute", tool_calls.len());
                tracing::debug!("⏱️  [PERF] Tool execution phase started at {:?}", start_time.elapsed());

                // CRITICAL FIX: Add the assistant message with tool calls to conversation history
                // The messages array from the agent includes: [...context, user_msg, assistant_with_tool_calls]
                // We need to add the assistant message to our history BEFORE adding tool results
                if let Some(assistant_msg) = messages.iter().rev().find(|m| m.role == "assistant") {
                    tracing::debug!("📝 [HISTORY] Adding ASSISTANT message with tool calls - content_len: {}, tool_calls: {}, total_history: {}",
                        assistant_msg.content.len(),
                        assistant_msg.tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0),
                        self.message_history.len() + 1);

                    // DEFENSIVE: Validate before adding
                    if assistant_msg.content.is_empty() && assistant_msg.tool_calls.is_none() {
                        tracing::error!("❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!");
                    } else {
                        self.message_history.push_back(assistant_msg.clone());
                    }
                }

                // Execute each tool call sequentially
                for (idx, tool_call) in tool_calls.iter().enumerate() {
                    tracing::info!("Executing tool {}/{}: {} (ID: {})",
                        idx + 1, tool_calls.len(), tool_call.name, tool_call.id);

                    // Publish tool execution status
                    let event = Event::new(
                        self.active_agent_id.clone(),
                        "broadcast".to_string(),
                        EventKind::AgentStatusChange {
                            agent_id: self.active_agent_id.clone(),
                            status: AgentStatus::ExecutingTool(tool_call.name.clone()),
                        },
                    );
                    let _ = self.event_bus.publish(event);

                    let tool_start = std::time::Instant::now();

                    // Execute the tool (delegates to specialist agent)
                    let args_str = tool_call.arguments.to_string();
                    let result = self.execute_tool(&tool_call.name, &args_str).await?;

                    tracing::info!("Tool {} completed in {:?}, result length: {} chars",
                        tool_call.name, tool_start.elapsed(), result.len());
                    tracing::debug!("⏱️  [PERF] Tool {}/{} completed at {:?} (took {:?})",
                        idx + 1, tool_calls.len(), start_time.elapsed(), tool_start.elapsed());

                    // Add tool result to messages array for current request
                    messages.push(LlmMessage::tool_result(tool_call.id.clone(), result.clone()));

                    // CRITICAL FIX: Add actual tool result content to conversation history
                    // (Previously stored placeholder "Tool executed", now stores actual result for better context)
                    tracing::debug!("📝 [HISTORY] Adding TOOL RESULT - tool_id: {}, result_len: {}, total_history: {}",
                        tool_call.id, result.len(), self.message_history.len() + 1);

                    // DEFENSIVE: Validate tool result has content
                    if result.is_empty() {
                        tracing::warn!("⚠️  [HISTORY] Tool result for {} is EMPTY - adding anyway (required for conversation flow)", tool_call.id);
                    }

                    self.message_history.push_back(LlmMessage::tool_result(tool_call.id.clone(), result));
                }

                // Make follow-up request with tool results to get final response
                tracing::info!("All tools executed, requesting final response from agent");
                tracing::debug!("⏱️  [PERF] All tools completed at {:?}, requesting final response", start_time.elapsed());

                // DEBUG: Log messages array to diagnose empty content error
                tracing::debug!("Messages array before process_with_results ({} messages):", messages.len());
                for (idx, msg) in messages.iter().enumerate() {
                    tracing::debug!(
                        "  Message[{}]: role={}, content_len={}, has_tool_calls={}, has_tool_call_id={}",
                        idx,
                        msg.role,
                        msg.content.len(),
                        msg.tool_calls.is_some(),
                        msg.tool_call_id.is_some()
                    );
                }

                let mut final_result_rx = agent.process_with_results(messages);

                // Wait for the final streaming response
                let final_stream = match final_result_rx.recv().await {
                    Some(Ok(stream)) => {
                        tracing::debug!("⏱️  [PERF] Final streaming response started at {:?}", start_time.elapsed());
                        Ok(stream)
                    }
                    Some(Err(e)) => Err(e),
                    None => anyhow::bail!("No final response from agent"),
                }?;

                // Publish responding status for final response
                let _ = self.event_bus.publish(Event::new(
                    "system".to_string(),
                    "broadcast".to_string(),
                    EventKind::AgentStatusChange {
                        agent_id: self.active_agent_id.clone(),
                        status: AgentStatus::Responding,
                    },
                ));

                // Return the final stream
                Ok(final_stream)
            }
            Err(e) => {
                // Error occurred during agent processing
                Err(e)
            }
        }
    }

    /// Send a message and wait for complete response (blocking)
    /// This is useful for scripting scenarios where you want the full response
    /// NOTE: This method is deprecated and may be removed in a future version.
    /// Use send_message() with proper async handling instead.
    #[deprecated(note = "Use async send_message() instead to avoid nested runtime issues")]
    pub fn send_message_blocking(&mut self, message: &str) -> Result<String> {
        // This is a simplified blocking version that doesn't support tool execution
        // For full functionality with tool support, use the async send_message() method

        let context_messages: Vec<LlmMessage> = self.message_history
            .iter()
            .take(self.max_history_size)
            .cloned()
            .collect();

        let agent = self.agents
            .iter()
            .find(|a| a.id() == self.active_agent_id)
            .context("Active agent not found")?;

        let mut result_rx = agent.process_message_nonblocking(
            message.to_string(),
            context_messages,
            None, // No tools in blocking mode to keep it simple
        );

        self.message_history.push_back(LlmMessage::new("user", message));

        while self.message_history.len() > self.max_history_size {
            self.message_history.pop_front();
        }

        let mut stream_rx = self.runtime.block_on(async {
            match result_rx.recv().await {
                Some(Ok(AgentResponse::StreamingResponse(stream))) => Ok(stream),
                Some(Ok(AgentResponse::NeedsToolExecution { .. })) => {
                    anyhow::bail!("Tool execution not supported in blocking mode")
                }
                Some(Err(e)) => Err(e),
                None => anyhow::bail!("No response received"),
            }
        })?;

        let mut full_response = String::new();
        self.runtime.block_on(async {
            while let Some(chunk) = stream_rx.recv().await {
                full_response.push_str(&chunk);
            }
        });

        // CRITICAL: Only add assistant message if it has content
        // Anthropic API rejects messages with empty content
        if !full_response.is_empty() {
            self.message_history.push_back(LlmMessage::new("assistant", full_response.clone()));
        } else {
            tracing::warn!("⚠️  Skipping empty assistant message in history");
        }

        Ok(full_response)
    }

    /// Clear the message history
    /// This is the programmatic equivalent of the "Clear" button
    pub fn clear_history(&mut self) {
        tracing::info!("🗑️  Clearing conversation history ({} messages)", self.message_history.len());
        self.message_history.clear();

        // Publish clear conversation event to notify all subscribers
        let event = Event::new(
            "api".to_string(),
            "broadcast".to_string(),
            EventKind::SystemCommand(crate::events::SystemCommand::ClearConversation),
        );

        if let Err(e) = self.event_bus.publish(event) {
            tracing::warn!("Failed to publish clear conversation event: {:?}", e);
        }
    }

    /// Get the current message history
    pub fn get_history(&self) -> Vec<LlmMessage> {
        self.message_history.iter().cloned().collect()
    }

    /// Get the status of an agent
    pub fn agent_status(&self, agent_id: &str) -> Option<&AgentStatus> {
        self.agents
            .iter()
            .find(|a| a.id() == agent_id)
            .map(|a| a.status())
    }

    /// Get the status of the currently active agent
    pub fn current_agent_status(&self) -> Option<&AgentStatus> {
        self.agent_status(&self.active_agent_id)
    }

    /// Publish a custom event to the event bus
    /// This allows external code to participate in the event system
    pub fn publish_event(&self, event: Event) -> Result<()> {
        self.event_bus.publish(event)
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to publish event: {:?}", e))
    }

    /// Subscribe to events from the event bus
    /// Returns a receiver that will get all published events
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.event_bus.subscribe()
    }

    /// Add an assistant response to the message history
    /// This should be called after receiving the complete response from streaming
    pub fn add_assistant_response(&mut self, response: String) {
        tracing::debug!("📝 [HISTORY] add_assistant_response called - response_len: {}, total_history: {}",
            response.len(), self.message_history.len());

        // CRITICAL: Only add assistant message if it has content
        // Anthropic API rejects messages with empty content
        if !response.is_empty() {
            tracing::debug!("📝 [HISTORY] Adding FINAL ASSISTANT response - content_len: {}, total_history: {}",
                response.len(), self.message_history.len() + 1);
            self.message_history.push_back(LlmMessage::new("assistant", response));
        } else {
            tracing::warn!("⚠️  [HISTORY] BLOCKED: Skipping empty assistant message in add_assistant_response");
        }

        // Trim history if needed
        while self.message_history.len() > self.max_history_size {
            self.message_history.pop_front();
        }
    }
}

/// Implement ToolExecutor for RustbotApi
/// This allows agents to execute tool calls by delegating to specialist agents
#[async_trait]
impl ToolExecutor for RustbotApi {
    async fn execute_tool(&self, tool_name: &str, arguments: &str) -> Result<String> {
        tracing::info!("Executing tool: {} with args: {}", tool_name, arguments);

        // Find the specialist agent matching the tool name
        let specialist_agent = self.agents
            .iter()
            .find(|a| a.id() == tool_name)
            .context(format!("Specialist agent '{}' not found", tool_name))?;

        // Parse arguments JSON (could be used to construct a more specific prompt)
        // For now, we'll just pass the arguments as the user message
        let prompt = format!("Execute with arguments: {}", arguments);

        // Execute the specialist agent with no context and no tools
        let mut result_rx = specialist_agent.process_message_nonblocking(
            prompt,
            vec![],  // No conversation context for tool execution
            None,    // Specialist agents don't get tools
        );

        // Await and collect the result
        let mut stream_rx = match result_rx.recv().await {
            Some(Ok(AgentResponse::StreamingResponse(stream))) => Ok(stream),
            Some(Ok(AgentResponse::NeedsToolExecution { .. })) => {
                anyhow::bail!("Unexpected: Specialist agent requested tool execution")
            }
            Some(Err(e)) => Err(e),
            None => anyhow::bail!("No response from specialist agent"),
        }?;

        // Collect all chunks into result
        let mut result = String::new();
        while let Some(chunk) = stream_rx.recv().await {
            result.push_str(&chunk);
        }

        tracing::info!("Tool execution result: {}", result);
        Ok(result)
    }
}

/// Builder for creating RustbotApi instances with configuration
pub struct RustbotApiBuilder {
    event_bus: Option<Arc<EventBus>>,
    runtime: Option<Arc<Runtime>>,
    max_history_size: usize,
    system_instructions: String,
    llm_adapter: Option<Arc<dyn LlmAdapter>>,
    agent_configs: Vec<AgentConfig>,
}

impl RustbotApiBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            event_bus: None,
            runtime: None,
            max_history_size: 20,
            system_instructions: String::new(),
            llm_adapter: None,
            agent_configs: vec![AgentConfig::default_assistant()],
        }
    }

    /// Set the event bus (optional - will create one if not provided)
    pub fn event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Set the tokio runtime (optional - will create one if not provided)
    pub fn runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set maximum history size
    pub fn max_history_size(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    /// Set system-level instructions for all agents
    pub fn system_instructions(mut self, instructions: String) -> Self {
        self.system_instructions = instructions;
        self
    }

    /// Set the LLM adapter for agents
    pub fn llm_adapter(mut self, adapter: Arc<dyn LlmAdapter>) -> Self {
        self.llm_adapter = Some(adapter);
        self
    }

    /// Add an agent configuration
    pub fn add_agent(mut self, config: AgentConfig) -> Self {
        self.agent_configs.push(config);
        self
    }

    /// Build the RustbotApi instance
    pub fn build(self) -> Result<RustbotApi> {
        let event_bus = self.event_bus.unwrap_or_else(|| Arc::new(EventBus::new()));
        let runtime = self.runtime.unwrap_or_else(|| Arc::new(Runtime::new().unwrap()));

        let llm_adapter = self.llm_adapter
            .context("LLM adapter must be provided")?;

        let mut api = RustbotApi::new(
            Arc::clone(&event_bus),
            Arc::clone(&runtime),
            self.max_history_size,
        );

        // Store agent configs for tool registry
        api.agent_configs = self.agent_configs.clone();

        // Create agents from configs
        for config in self.agent_configs {
            let agent = Agent::new(
                config,
                Arc::clone(&llm_adapter),
                Arc::clone(&event_bus),
                Arc::clone(&runtime),
                self.system_instructions.clone(),
            );
            api.register_agent(agent);
        }

        // Build initial tool registry from enabled specialist agents
        api.update_tools();

        Ok(api)
    }
}

impl Default for RustbotApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::OpenRouterAdapter;

    #[test]
    fn test_api_creation() {
        let event_bus = Arc::new(EventBus::new());
        let runtime = Arc::new(Runtime::new().unwrap());

        let api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

        assert_eq!(api.active_agent(), "assistant");
        assert_eq!(api.list_agents().len(), 0); // No agents registered yet
    }

    #[test]
    fn test_agent_registration() {
        let event_bus = Arc::new(EventBus::new());
        let runtime = Arc::new(Runtime::new().unwrap());
        let adapter: Arc<dyn LlmAdapter> = Arc::new(OpenRouterAdapter::new("test-key".to_string()));

        let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

        let agent = Agent::new(
            AgentConfig::default_assistant(),
            adapter,
            event_bus,
            runtime,
            String::new(),
        );

        api.register_agent(agent);

        assert_eq!(api.list_agents().len(), 1);
        assert_eq!(api.list_agents()[0], "assistant");
    }

    #[test]
    fn test_agent_switching() {
        let event_bus = Arc::new(EventBus::new());
        let runtime = Arc::new(Runtime::new().unwrap());
        let adapter: Arc<dyn LlmAdapter> = Arc::new(OpenRouterAdapter::new("test-key".to_string()));

        let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

        // Register two agents
        let agent1 = Agent::new(
            AgentConfig::default_assistant(),
            Arc::clone(&adapter),
            Arc::clone(&event_bus),
            Arc::clone(&runtime),
            String::new(),
        );
        api.register_agent(agent1);

        let mut config2 = AgentConfig::default_assistant();
        config2.id = "researcher".to_string();
        config2.name = "Researcher".to_string();

        let agent2 = Agent::new(
            config2,
            adapter,
            Arc::clone(&event_bus),
            runtime,
            String::new(),
        );
        api.register_agent(agent2);

        // Switch agents
        assert_eq!(api.active_agent(), "assistant");
        api.switch_agent("researcher").unwrap();
        assert_eq!(api.active_agent(), "researcher");

        // Try invalid agent
        assert!(api.switch_agent("invalid").is_err());
    }
}
