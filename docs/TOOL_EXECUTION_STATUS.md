# Tool Execution Implementation Status

**Last Updated**: 2025-11-13
**Status**: Infrastructure Complete, Execution Loop Pending

## What's Complete ✅

### 1. Tool Registration & Discovery
- ✅ `ToolDefinition` type matching OpenAI function calling format
- ✅ Tool registry in RustbotApi built from enabled specialist agents
- ✅ Tools automatically passed to primary agent

### 2. Tool Call Detection
- ✅ `Delta` struct extended with `tool_calls` field
- ✅ Streaming responses detect tool call chunks (logged)
- ✅ `complete_chat` parses tool calls from response

### 3. Message Format Support
- ✅ Message type supports `tool_calls` field (assistant messages)
- ✅ Message type supports `tool_call_id` field (tool result messages)
- ✅ Helper constructors: `Message::tool_result()`, `Message::with_tool_calls()`

### 4. LLM Adapter Support
- ✅ `LlmRequest` accepts `tools` and `tool_choice` parameters
- ✅ OpenRouter adapter passes tools in API requests
- ✅ OpenRouter adapter parses tool calls from responses
- ✅ `LlmResponse` includes `tool_calls` field

### 5. Tool Execution Infrastructure
- ✅ `ToolExecutor` trait defined
- ✅ `RustbotApi` implements `ToolExecutor`
- ✅ Can execute tools by delegating to specialist agents

## What's Pending ⏳

### 1. Tool Execution Loop

**Challenge**: Architectural circular dependency

The current architecture has this ownership:
```
RustbotApi owns Vec<Agent>
Agent needs ToolExecutor to execute tools
RustbotApi implements ToolExecutor
→ Can't pass RustbotApi to Agent (circular reference)
```

**Solutions**:

#### Option A: Two-Phase Execution (Recommended)
Agent returns tool calls without executing them, RustbotApi orchestrates:

```rust
// In Agent
pub fn process_with_tools() -> Result<AgentResponse> {
    let initial_response = llm_adapter.complete_chat(request).await?;

    if let Some(tool_calls) = initial_response.tool_calls {
        // Return tool calls to caller for execution
        return Ok(AgentResponse::NeedsToolExecution {
            tool_calls,
            messages: conversation_history,
        });
    }

    // No tools needed, stream normal response
    Ok(AgentResponse::StreamingResponse(rx))
}

// In RustbotApi
pub fn send_message(&mut self, message: &str) -> Result<...> {
    let response = agent.process_with_tools(...)?;

    match response {
        AgentResponse::NeedsToolExecution { tool_calls, mut messages } => {
            // Execute each tool call
            for tool_call in tool_calls {
                let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await?;
                messages.push(Message::tool_result(tool_call.id, result));
            }

            // Make follow-up request with results
            let final_response = agent.process_with_results(messages)?;
            Ok(final_response)
        }
        AgentResponse::StreamingResponse(rx) => Ok(rx),
    }
}
```

#### Option B: Separate ToolExecutor Service
Create a standalone service that has access to agents:

```rust
pub struct ToolExecutorService {
    agents: Arc<Vec<Agent>>,  // Shared reference to agents
    runtime: Arc<Runtime>,
}

impl ToolExecutor for ToolExecutorService {
    async fn execute_tool(...) -> Result<String> {
        // Find specialist and execute
    }
}

// Pass Arc<ToolExecutorService> to Agent
// RustbotApi creates the service with Arc::new(agents)
```

#### Option C: Callback Function
Pass a closure to Agent that can execute tools:

```rust
type ToolExecutorFn = Arc<dyn Fn(&str, &str) -> BoxFuture<'static, Result<String>> + Send + Sync>;

impl Agent {
    pub fn with_tool_executor(self, executor: ToolExecutorFn) -> Self {
        self.tool_executor = Some(executor);
        self
    }
}

// In RustbotApi
let executor: ToolExecutorFn = Arc::new(move |name, args| {
    // Closure has access to agents via Arc
    Box::pin(async move {
        // Execute tool
    })
});
```

### 2. Response Streaming with Tool Calls

**Challenge**: How to stream when tools might be called mid-conversation

**Current Approach** (Decided):
- Use non-streaming `complete_chat` for initial request when tools present
- Execute tools
- Stream final response

**Future Enhancement**:
- Full streaming with tool calls
- Show "Calling tool..." message to user
- Stream final response after tool execution

### 3. Error Handling

Need to handle:
- Tool execution failures
- Specialist agent not found
- Invalid tool arguments
- Timeout scenarios

### 4. Testing

End-to-end testing needed:
- Create simple specialist agent (e.g., calculator)
- Primary agent calls calculator tool
- Verify result is incorporated into final response

## Next Steps

### Immediate (Implement Option A):

1. **Create AgentResponse enum** (`src/agent/mod.rs`):
   ```rust
   pub enum AgentResponse {
       StreamingResponse(mpsc::UnboundedReceiver<String>),
       NeedsToolExecution {
           tool_calls: Vec<ToolCall>,
           messages: Vec<Message>,
       },
   }
   ```

2. **Modify Agent.process_message_nonblocking**:
   - Check if tools parameter is provided
   - If yes, use `complete_chat` instead of `stream_chat`
   - Check response for tool calls
   - Return `AgentResponse` enum

3. **Update RustbotApi.send_message**:
   - Handle `AgentResponse` enum
   - Execute tools using `self.execute_tool()`
   - Make follow-up request with results
   - Stream final response

4. **Add logging**:
   - Log tool calls detected
   - Log tool execution start/end
   - Log tool results

### Short Term:

1. Add error recovery for tool execution
2. Add timeout handling
3. Support multiple sequential tool calls
4. Add tool execution metrics

### Future:

1. Support parallel tool calls
2. Implement streaming with tool calls
3. Add tool call caching
4. Add tool call retries

## Architecture Diagram

```
User Message
     ↓
RustbotApi.send_message()
     ↓
Agent.process_message_nonblocking(tools)
     ↓
LLM (with tools) → Response with tool_calls
     ↓
Agent returns AgentResponse::NeedsToolExecution
     ↓
RustbotApi.execute_tool() → Specialist Agent
     ↓
Tool Result
     ↓
Agent.process_with_results() → LLM (with tool results)
     ↓
Final Response → Stream to User
```

## Files Modified

- ✅ `src/agent/tools.rs` - Tool definitions
- ✅ `src/llm/types.rs` - Message extensions, ToolCall type
- ✅ `src/llm/openrouter.rs` - Tool support in adapter
- ✅ `src/api.rs` - ToolExecutor implementation
- ✅ `src/tool_executor.rs` - ToolExecutor trait
- ⏳ `src/agent/mod.rs` - Need AgentResponse enum and execution loop

## Commits

```
67277d1 feat: add ToolExecutor trait and implement for RustbotApi
dc71aa3 docs: update progress log with completed tool infrastructure work
19c9311 feat: add tool definition support to OpenRouter adapter
c1aac35 feat: add tool call and tool result support to Message type
77e085d feat: add tool call detection in streaming responses
```

## References

- OpenAI Function Calling: https://platform.openai.com/docs/guides/function-calling
- OpenRouter Tool Support: https://openrouter.ai/docs/function-calling
- Tool Call Message Format: Standard OpenAI format with `role: "tool"`
