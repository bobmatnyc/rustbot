# Tool Execution Implementation Progress

**Date**: 2025-11-13
**Goal**: Implement two-phase tool execution pattern for agent delegation
**Status**: In Progress - Core infrastructure complete, orchestration pending

## Session Overview

Implementing the recommended two-phase execution pattern to resolve circular dependency:
1. Agent detects tool calls and returns them (without executing)
2. RustbotApi orchestrates tool execution by delegating to specialist agents
3. RustbotApi makes follow-up request with tool results
4. Agent streams final response

## What's Complete ✅

### 1. AgentResponse Enum (src/agent/mod.rs:154-170)

Created enum to support two-phase execution:

```rust
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
```

### 2. Modified Agent.process_message_nonblocking (src/agent/mod.rs:369-512)

**Key Changes**:
- Changed return type from nested channel to `mpsc::UnboundedReceiver<Result<AgentResponse>>`
- When tools provided, uses `complete_chat` instead of `stream_chat`
- Detects tool calls in LLM response
- Returns `AgentResponse::NeedsToolExecution` when tools detected
- Returns `AgentResponse::StreamingResponse` for normal responses
- Added comprehensive logging for debugging

**Algorithm**:
```rust
if tools.is_some() {
    // Use complete_chat to detect tool calls
    let response = llm_adapter.complete_chat(request).await?;

    if response.tool_calls.is_some() {
        // Add assistant message with tool calls to history
        // Return AgentResponse::NeedsToolExecution
    } else {
        // No tools needed, return streaming response
    }
} else {
    // No tools provided, use stream_chat as before
}
```

### 3. Added Agent.process_with_results (src/agent/mod.rs:514-575)

New method for follow-up requests after tool execution:
- Takes complete message history including tool results
- Streams final response
- Used by RustbotApi after executing tools

### 4. Updated Imports (src/api.rs:5)

Added `AgentResponse` to imports from agent module.

## What's Pending ⏳

### 1. Update RustbotApi.execute_tool (Critical - Breaks Compilation)

**Location**: src/api.rs:290-317

**Current Issue**:
The `execute_tool` method still expects old return type from `process_message_nonblocking`.

**Error**:
```
error[E0599]: no method named `recv` found for enum `AgentResponse`
   --> src/api.rs:310:40
```

**Required Changes**:
```rust
async fn execute_tool(&mut self, tool_name: &str, tool_args: &str) -> Result<String> {
    // ... find specialist agent ...

    let result_rx = specialist_agent.process_message_nonblocking(
        prompt,
        vec![],
        None,  // Specialist agents don't get tools
    );

    // NEW: Handle AgentResponse enum
    let stream_rx = self.runtime.block_on(async {
        let mut rx = result_rx;
        match rx.recv().await {
            Some(Ok(AgentResponse::StreamingResponse(stream))) => Ok(stream),
            Some(Ok(AgentResponse::NeedsToolExecution { .. })) => {
                anyhow::bail!("Unexpected tool call in specialist agent")
            }
            Some(Err(e)) => Err(e),
            None => anyhow::bail!("No response from specialist agent"),
        }
    })?;

    // ... rest remains the same ...
}
```

### 2. Update RustbotApi.send_message (Critical - Main Orchestration)

**Location**: src/api.rs:127-188

**Required Changes**:
```rust
pub fn send_message(
    &mut self,
    message: &str,
) -> Result<mpsc::UnboundedReceiver<Result<mpsc::UnboundedReceiver<String>>>> {
    // ... existing setup code ...

    // Process message through agent
    let result_rx = agent.process_message_nonblocking(
        message.to_string(),
        context_messages,
        tools,
    );

    // Add user message to history
    self.message_history.push_back(LlmMessage::new("user", message));

    // NEW: Create channel for final response
    let (final_tx, final_rx) = mpsc::unbounded_channel();

    // NEW: Spawn task to handle tool execution orchestration
    let runtime = Arc::clone(&self.runtime);
    let agent_ref = agent; // Need to handle borrowing here

    runtime.spawn(async move {
        let mut rx = result_rx;
        match rx.recv().await {
            Some(Ok(AgentResponse::StreamingResponse(stream))) => {
                // No tools needed, forward stream directly
                let _ = final_tx.send(Ok(stream));
            }
            Some(Ok(AgentResponse::NeedsToolExecution { tool_calls, mut messages })) => {
                // Execute each tool call
                for tool_call in tool_calls {
                    tracing::info!("Executing tool: {}", tool_call.name);

                    // Execute tool (delegate to specialist)
                    let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await?;

                    // Add tool result to message history
                    messages.push(LlmMessage::tool_result(tool_call.id, result));
                }

                // Make follow-up request with tool results
                let final_stream_rx = agent_ref.process_with_results(messages);

                // Forward to final channel
                // ... handle nested Result ...
            }
            Some(Err(e)) => {
                let _ = final_tx.send(Err(e));
            }
            None => {
                let _ = final_tx.send(Err(anyhow::anyhow!("No response")));
            }
        }
    });

    Ok(final_rx)
}
```

**Challenge**: Need to handle borrowing/ownership for `self` in spawned task
**Solution**: May need to refactor to pass specific data (agents, runtime, etc.) or use Arc/Mutex

### 3. Handle Borrowing Issues

The spawned task in `send_message` needs access to:
- `self.runtime` - already Arc, can clone
- `self.execute_tool()` - needs &mut self
- Agent reference - needs to live long enough

**Options**:
1. Don't spawn - do everything synchronously with `block_on`
2. Extract tool execution logic to separate method that takes Arc references
3. Use channels to communicate with background task

**Recommended**: Option 1 (synchronous) for MVP, then optimize later

## Files Modified

| File | Status | Changes |
|------|--------|---------|
| `src/agent/mod.rs` | ✅ Complete | Added AgentResponse enum, updated process_message_nonblocking, added process_with_results |
| `src/api.rs` | ⏳ Pending | Added imports, need to update execute_tool and send_message |

## Implementation Plan

### Next Steps (In Order):

1. **Fix execute_tool method** (15 minutes)
   - Update to handle AgentResponse enum
   - Pattern match on StreamingResponse variant
   - Error on NeedsToolExecution (specialist shouldn't call tools)

2. **Update send_message for tool orchestration** (30-45 minutes)
   - Handle AgentResponse::NeedsToolExecution case
   - Implement tool execution loop
   - Make follow-up request with results
   - Handle borrowing/ownership (use block_on approach for MVP)

3. **Test compilation** (5 minutes)
   - `cargo check`
   - Fix any remaining compilation errors

4. **Manual testing** (15-20 minutes)
   - Start app with web_search agent enabled
   - Send message that triggers web search
   - Verify tool call detection
   - Verify tool execution
   - Verify final response incorporation

5. **Documentation** (15 minutes)
   - Update this document with results
   - Create session progress log
   - Document any issues found

## Technical Notes

### Why Two-Phase Execution?

Original problem: Circular dependency
```
RustbotApi owns Vec<Agent>
Agent needs ToolExecutor to execute tools
RustbotApi implements ToolExecutor
→ Can't pass RustbotApi to Agent (circular reference)
```

Solution: Separation of concerns
- Agent: Detects tool calls, doesn't execute
- RustbotApi: Orchestrates execution
- Clean ownership, no circular references

### Tool Execution Flow

```
User Message
    ↓
RustbotApi.send_message()
    ↓
Agent.process_message_nonblocking(tools)
    ↓
LLM complete_chat → Returns tool_calls
    ↓
Agent returns AgentResponse::NeedsToolExecution
    ↓
RustbotApi handles response
    ↓
For each tool_call:
    RustbotApi.execute_tool() → Specialist Agent
    Add result to messages
    ↓
Agent.process_with_results(messages)
    ↓
LLM stream_chat → Final response
    ↓
Stream to User
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│              RustbotApi                         │
│  (Orchestrator - owns agents, executes tools)  │
└────────────┬──────────────────┬─────────────────┘
             │                  │
             │                  │
    ┌────────▼──────────┐  ┌───▼────────────────┐
    │  Primary Agent    │  │ Specialist Agent  │
    │  (Tool Detection) │  │ (Tool Execution)  │
    └────────┬──────────┘  └───────────────────┘
             │
             ▼
      ┌────────────┐
      │    LLM     │
      │ (OpenRouter)│
      └────────────┘
```

## Debugging Tips

### Enable Trace Logging

All key points have `tracing::info!()` calls:
- "Processing message with tools enabled"
- "Tool calls detected: {} calls"
- "No tool calls detected, streaming response"
- "Processing message without tools, using stream_chat"

### Common Issues

1. **Tool calls not detected**: Check if tools are being passed to agent
2. **Specialist agent called with tools**: Bug - specialist shouldn't get tools
3. **Borrowing errors in send_message**: Use block_on instead of spawn
4. **Missing tool results in history**: Ensure tool_result messages added correctly

## Git Commits

```bash
# Current session (not yet committed)
# - Created AgentResponse enum
# - Modified Agent.process_message_nonblocking
# - Added Agent.process_with_results
# - Updated api.rs imports

# Will commit after compilation succeeds
```

## References

- [TOOL_EXECUTION_STATUS.md](./TOOL_EXECUTION_STATUS.md) - Original design document
- [tool-call-detection.md](./2025-11-13-tool-call-detection.md) - Previous session
- OpenAI Function Calling: https://platform.openai.com/docs/guides/function-calling

---

**Next Session**: Start with "Update RustbotApi.execute_tool" section above
