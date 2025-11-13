# Tool Registration and Discovery Design

**Date**: 2025-11-13
**Purpose**: Enable assistant agent to delegate to specialist agents via tool calling

## Flow Overview

```
User: "What's the weather in Tokyo?"
    ↓
Assistant receives message
    ↓
Assistant sees available tools: [web_search]
    ↓
Assistant decides to use web_search tool
    ↓
Assistant makes tool call:
  {
    "name": "web_search",
    "arguments": { "query": "current weather in Tokyo" }
  }
    ↓
Rustbot intercepts tool call
    ↓
Routes to web_search agent
    ↓
Web search agent processes query
    ↓
Returns result to Rustbot
    ↓
Rustbot sends tool result back to assistant:
  {
    "role": "tool",
    "content": "Tokyo weather: 72°F, sunny...",
    "tool_call_id": "call_abc123"
  }
    ↓
Assistant synthesizes response
    ↓
User receives: "The current weather in Tokyo is 72°F and sunny..."
```

## Tool Definition Format

Each enabled specialist agent becomes a tool in the assistant's context:

```json
{
  "type": "function",
  "function": {
    "name": "web_search",
    "description": "Search the web for current, real-time information. Use this when the user asks about recent events, current data, or information after your knowledge cutoff.",
    "parameters": {
      "type": "object",
      "properties": {
        "query": {
          "type": "string",
          "description": "The search query to execute"
        }
      },
      "required": ["query"]
    }
  }
}
```

## Implementation Components

### 1. Tool Definition Builder

Create a module to convert `AgentConfig` → Tool Definition:

```rust
// src/agent/tools.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,  // "function"
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: FunctionParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameters {
    #[serde(rename = "type")]
    pub param_type: String,  // "object"
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

impl ToolDefinition {
    /// Convert an AgentConfig to a tool definition
    pub fn from_agent(agent: &AgentConfig) -> Self {
        // Extract tool description from agent's instructions
        let description = Self::extract_tool_description(agent);

        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: agent.id.clone(),
                description,
                parameters: Self::build_parameters(agent),
            },
        }
    }

    fn extract_tool_description(agent: &AgentConfig) -> String {
        // For web_search: extract first paragraph
        // For other agents: use description field
        // Fallback: use name

        if agent.id == "web_search" {
            "Search the web for current, real-time information. Use this when the user asks about recent events, current data, or information after your knowledge cutoff.".to_string()
        } else {
            agent.name.clone()
        }
    }

    fn build_parameters(agent: &AgentConfig) -> FunctionParameters {
        // Web search takes a query parameter
        if agent.id == "web_search" {
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "query": {
                        "type": "string",
                        "description": "The search query to execute"
                    }
                }),
                required: vec!["query".to_string()],
            }
        } else {
            // Generic: message parameter
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "message": {
                        "type": "string",
                        "description": "The message to send to the agent"
                    }
                }),
                required: vec!["message".to_string()],
            }
        }
    }
}
```

### 2. Tool Registry in RustbotApi

Update `src/api.rs` to maintain available tools:

```rust
pub struct RustbotApi {
    // ... existing fields ...

    /// Available tools (derived from enabled specialist agents)
    available_tools: Vec<ToolDefinition>,
}

impl RustbotApi {
    /// Build tool definitions from enabled specialist agents
    fn build_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.agents
            .values()
            .filter(|agent| !agent.config.is_primary && agent.config.enabled)
            .map(|agent| ToolDefinition::from_agent(&agent.config))
            .collect()
    }

    /// Update available tools (call when agents enabled/disabled)
    pub fn update_tools(&mut self) {
        self.available_tools = self.build_tool_definitions();
    }
}
```

### 3. Include Tools in LLM Request

When primary agent sends a message, include tools:

```rust
// In RustbotApi::send_message
pub async fn send_message(&mut self, message: &str) -> Result<mpsc::UnboundedReceiver<String>> {
    // ... existing code to find primary agent ...

    // Build LLM request with tools
    let mut llm_request = LlmRequest::new(messages);

    // Add tools if this is the primary agent
    if primary_agent.config.is_primary {
        llm_request.tools = Some(self.available_tools.clone());
    }

    // ... continue with streaming ...
}
```

### 4. Tool Call Detection and Routing

Modify the streaming response handler to detect tool calls:

```rust
// In stream handling
match chunk_type {
    "text" => {
        // Regular text response
        let _ = tx.send(content);
    }
    "tool_calls" => {
        // Assistant wants to use a tool
        let tool_calls: Vec<ToolCall> = serde_json::from_value(data)?;

        for tool_call in tool_calls {
            // Route to the appropriate agent
            let result = self.execute_tool_call(tool_call).await?;

            // Send tool result back to assistant
            let tool_result_message = LlmMessage {
                role: "tool".to_string(),
                content: result,
                tool_call_id: Some(tool_call.id),
            };

            // Continue conversation with tool result
            history.push(tool_result_message);

            // Get assistant's response after receiving tool result
            // (This happens automatically in the next iteration)
        }
    }
}
```

### 5. Tool Call Execution

```rust
impl RustbotApi {
    async fn execute_tool_call(&mut self, tool_call: ToolCall) -> Result<String> {
        let agent_id = &tool_call.function.name;

        // Find the specialist agent
        let agent = self.agents.get_mut(agent_id)
            .ok_or_else(|| anyhow!("Tool {} not found", agent_id))?;

        // Parse arguments
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

        // Build message for specialist agent
        let message = match agent_id.as_str() {
            "web_search" => {
                args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing query parameter"))?
                    .to_string()
            }
            _ => {
                args.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing message parameter"))?
                    .to_string()
            }
        };

        // Send message to specialist agent
        let mut response_rx = agent.process_message_nonblocking(message, vec![]);

        // Wait for specialist agent to complete
        let stream_rx = response_rx.recv().await
            .ok_or_else(|| anyhow!("Agent did not respond"))??;

        // Collect full response from specialist
        let mut full_response = String::new();
        while let Some(chunk) = stream_rx.recv().await {
            full_response.push_str(&chunk);
        }

        Ok(full_response)
    }
}
```

## LLM Request Structure Updates

Update `src/llm/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,  // NEW

    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,  // NEW: for tool result messages
}
```

## OpenRouter Streaming Response Format

OpenRouter returns tool calls in streaming chunks:

```json
// Regular text chunk
{
  "choices": [{
    "delta": {
      "role": "assistant",
      "content": "I'll search for that information."
    }
  }]
}

// Tool call chunk
{
  "choices": [{
    "delta": {
      "tool_calls": [{
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "web_search",
          "arguments": "{\"query\": \"current weather Tokyo\"}"
        }
      }]
    }
  }]
}
```

## Message History Management

After tool call, history looks like:

```rust
vec![
    // User's original message
    Message { role: "user", content: "What's the weather in Tokyo?" },

    // Assistant's tool call (OpenRouter format)
    Message {
        role: "assistant",
        content: None,
        tool_calls: Some([...])
    },

    // Tool result
    Message {
        role: "tool",
        content: "Tokyo weather: 72°F, sunny...",
        tool_call_id: "call_abc123"
    },

    // Assistant's final response
    Message {
        role: "assistant",
        content: "The current weather in Tokyo is..."
    }
]
```

## Agent Description System

For better tool descriptions, add to JSON schema:

```json
{
  "name": "web_search",
  "description": "Search the web for current, real-time information",
  "toolDescription": "Use this when the user asks about recent events, current data, weather, news, or any information after your knowledge cutoff. Provide a clear, specific search query.",
  ...
}
```

Then use `toolDescription` in the tool definition if available.

## Implementation Steps

1. ✅ Create `src/agent/tools.rs` module
2. ✅ Add `ToolDefinition` and related types
3. ✅ Implement `from_agent()` conversion
4. ✅ Update `RustbotApi` with tool registry
5. ✅ Include tools in primary agent's LLM requests
6. ✅ Handle tool call responses in streaming
7. ✅ Implement `execute_tool_call()`
8. ✅ Test with web search delegation

## Testing Plan

### Test Case 1: Simple Web Search
```
User: "What's the weather in Tokyo?"
Expected:
1. Assistant receives tools: [web_search]
2. Assistant calls web_search("current weather Tokyo")
3. Web search agent returns results
4. Assistant synthesizes response
```

### Test Case 2: No Tool Needed
```
User: "What is 2+2?"
Expected:
1. Assistant receives tools: [web_search]
2. Assistant responds directly: "4"
3. No tool calls made
```

### Test Case 3: Multiple Tools Available
```
Agents: assistant (primary), web_search (enabled), code_gen (enabled)
User: "Search for the latest Python news"
Expected:
1. Assistant sees: [web_search, code_gen]
2. Assistant calls web_search (not code_gen)
3. Proper routing to correct agent
```

## Error Handling

```rust
// Tool not found
if !self.agents.contains_key(&tool_name) {
    return Err("Tool not available");
}

// Tool disabled
if !agent.config.enabled {
    return Err("Tool is currently disabled");
}

// Agent timeout
timeout(Duration::from_secs(30), agent.process_message())
    .await
    .map_err(|_| "Agent timeout")?;

// Invalid arguments
serde_json::from_str(&arguments)
    .map_err(|e| format!("Invalid tool arguments: {}", e))?;
```

## Future Enhancements

1. **Tool Chaining**: Allow specialists to call other specialists
2. **Parallel Tool Calls**: Execute multiple tools simultaneously
3. **Tool Result Validation**: Verify results before sending to assistant
4. **Tool Usage Analytics**: Track which tools are used most
5. **Custom Tool Parameters**: Allow agents to define their own schemas

## Success Criteria

- ✅ Assistant can see available specialist agents as tools
- ✅ Assistant can make tool calls with proper arguments
- ✅ Tool calls route to correct specialist agents
- ✅ Specialist responses return to assistant
- ✅ Assistant synthesizes final response for user
- ✅ User sees seamless delegation (no exposed mechanics)
