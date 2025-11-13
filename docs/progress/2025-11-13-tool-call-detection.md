# Session Progress: Tool Call Detection Implementation

**Date**: 2025-11-13
**Goal**: Implement tool call detection and routing for agent delegation

## Session Overview

Continuing from previous session on tool registration, focused on implementing tool call detection in streaming responses and planning the routing architecture.

## Features Implemented

### 1. Tool Call Detection in Streaming Responses

Added support for detecting tool calls in OpenRouter streaming responses:

- **Delta struct enhancement** (`src/llm/openrouter.rs:217-235`)
  - Added `tool_calls: Option<Vec<ToolCallDelta>>` field
  - Created `ToolCallDelta` struct with id, type, and function fields
  - Created `FunctionCall` struct with name and arguments

- **Detection logic** (`src/llm/openrouter.rs:114-119`)
  - Added tool call detection in stream handler
  - Logs detected tool calls for debugging
  - TODO: Implement actual routing and execution

### 2. Copy Button for Assistant Messages

Added clipboard copy functionality to UI:

- **Copy button** (`src/ui/views.rs:66-71`)
  - Shows only for assistant messages with content
  - Uses egui's `output_mut` API for clipboard access
  - Small button with clipboard icon next to message header

- **MessageRole comparison fix** (`src/ui/types.rs:124`)
  - Added `PartialEq` and `Eq` derives to enable enum comparison

## Files Modified

| File | Changes | Lines |
|------|---------|-------|
| `src/llm/openrouter.rs` | Added tool call types and detection | 217-235, 114-119 |
| `src/ui/views.rs` | Added copy button to messages | 66-71 |
| `src/ui/types.rs` | Added PartialEq to MessageRole | 124 |

## Technical Details

### Tool Call Streaming Architecture Challenge

Discovered significant architectural complexity for tool call handling in streaming:

**Current Flow:**
```
User → RustbotApi → Agent → stream_chat → Text chunks → User
```

**Desired Flow with Tools:**
```
User → RustbotApi → Agent → stream_chat → Tool call detected
  → Stop stream → Execute tool → New stream_chat with result → Final response
```

**Key Challenges:**

1. **stream_chat signature**: Currently returns `Result<()>` and pushes to channel
   - Need way to signal "tool call detected" back to caller
   - Tool calls come incrementally across multiple stream chunks
   - Need to accumulate chunks before executing

2. **Interrupting streams**: Tool calls require:
   - Stopping content streaming to user
   - Executing tool synchronously
   - Making new API request with tool result
   - Resuming streaming with final answer

3. **Architectural options**:
   - **Option A**: Change `stream_chat` to return enum `StreamEvent::Content | StreamEvent::ToolCall`
   - **Option B**: Use separate channel for tool call events
   - **Option C**: Have `stream_chat` accumulate tool calls and return them at end
   - **Option D**: Use non-streaming `complete_chat` when tools are present

### Tool Call Delta Accumulation

Tool calls in streaming come across multiple chunks:
```json
Chunk 1: {"delta": {"tool_calls": [{"index": 0, "id": "call_123"}]}}
Chunk 2: {"delta": {"tool_calls": [{"index": 0, "function": {"name": "web"}}]}}
Chunk 3: {"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "{\"q\""}}]}}
Chunk 4: {"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "uery\": \"test\"}"}}]}}
```

Need to:
1. Track accumulation state by index
2. Build complete tool call from fragments
3. Detect when tool call is complete
4. Execute tool
5. Format result for LLM

### 3. Message Type Extended for Tool Support

Enhanced the Message type to support tool calls and tool results:

- **Message fields** (`src/llm/types.rs:115-126`)
  - Added `tool_call_id: Option<String>` for tool result messages
  - Added `tool_calls: Option<Vec<ToolCall>>` for assistant tool calls
  - Both fields with `#[serde(default)]` for backward compatibility

- **Helper constructors** (`src/llm/types.rs:128-158`)
  - `Message::new()` - Simple user/assistant/system messages
  - `Message::tool_result()` - Create tool result messages
  - `Message::with_tool_calls()` - Create assistant message with tool calls

- **Updated all Message creations** throughout codebase to use new constructors

### 4. OpenRouter Adapter Tool Support

Updated OpenRouter adapter to support standard OpenAI function calling:

- **ApiRequest changes** (`src/llm/openrouter.rs:187-209`)
  - Changed `tools` from `Vec<WebSearchTool>` to `Vec<ToolDefinition>`
  - Added `tool_choice: Option<String>` parameter
  - Separated web_search (provider feature) from custom tools

- **complete_chat enhancement** (`src/llm/openrouter.rs:169-176`)
  - Now parses tool_calls from response messages
  - Returns tool calls in LlmResponse
  - Removes hardcoded `tool_calls: None` limitation

- **stream_chat updates** (`src/llm/openrouter.rs:40-64`)
  - Passes through custom tools from request
  - Passes through tool_choice from request
  - Maintains web_search as separate provider feature

## Git Commits

```
19c9311 feat: add tool definition support to OpenRouter adapter
c1aac35 feat: add tool call and tool result support to Message type
77e085d feat: add tool call detection in streaming responses
e8a6fe3 feat: add copy button to assistant messages
```

## Testing

### Compilation
- ✅ Build succeeds with warnings (expected for unused fields)
- ✅ No runtime errors

### Manual Testing
- ✅ Copy button appears on assistant messages
- ✅ Copy button correctly copies message to clipboard
- ⏳ Tool call detection not yet testable (need to send tools to LLM first)

## Current State

**Completed:**
- ✅ Tool registration system (from previous session)
- ✅ Tool passing to primary agent (from previous session)
- ✅ Tool call type definitions
- ✅ Tool call detection in stream (logging only)
- ✅ Copy button UI feature
- ✅ Message type extended for tool calls and results
- ✅ OpenRouter adapter supports ToolDefinition format
- ✅ complete_chat parses tool calls from response
- ✅ Tool infrastructure ready for execution

**In Progress:**
- 🔄 Tool execution loop in Agent layer

**Pending:**
- ⏳ Implement tool call execution in Agent.process_message_nonblocking
- ⏳ Add agent lookup logic (find specialist by tool name)
- ⏳ Handle tool execution errors gracefully
- ⏳ Format tool results for LLM
- ⏳ Make follow-up request with tool results
- ⏳ End-to-end testing with real specialist agents

## Architectural Decisions Needed

### 1. Stream Event Architecture

**Question**: How should we handle tool calls in streaming?

**Options:**
1. **Change stream_chat return type**: Return tool calls at end of stream
2. **Separate event channel**: Stream content on one channel, tool calls on another
3. **Enum stream events**: `StreamChunk::Content(String) | StreamChunk::ToolCall(...)`
4. **Non-streaming for tools**: Use `complete_chat` when tools are present

**Recommendation**: Option 4 (non-streaming for tools) initially, then migrate to Option 3 later
- Simplest to implement
- Tools are typically slow anyway (network calls, processing)
- User experience: Show "Calling tool..." message, then stream final response
- Can optimize later with full streaming support

### 2. Tool Execution Location

**Question**: Where should tool calls be executed?

**Options:**
1. **LLM Adapter level**: openrouter.rs handles tool execution
2. **Agent level**: Agent detects tool calls and executes them
3. **API level**: RustbotApi handles tool orchestration

**Recommendation**: Option 2 (Agent level)
- Agent already has event bus access
- Agent has the tool registry
- Keeps LLM adapter focused on API communication
- Allows different agents to have different tool execution strategies

### 3. Tool Result Format

**Question**: How should we format tool results for the LLM?

**Standard (OpenAI format)**:
```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "Tool execution result here"
}
```

This matches OpenAI/Anthropic conventions and should work with OpenRouter.

## Next Steps

### Immediate (Next Session):

**PRIORITY: Implement tool execution loop in Agent layer**

The infrastructure is complete. Now we need to connect the pieces:

1. **Modify Agent.process_message_nonblocking()** to handle tools:
   - Check if tools parameter is provided
   - If yes, use `complete_chat` instead of `stream_chat` for initial request
   - Check if response contains tool calls
   - If tool calls present, execute tool loop (see below)
   - If no tool calls, proceed with streaming the response

2. **Implement tool execution logic**:
   ```rust
   // Pseudo-code for tool execution loop
   if let Some(tool_calls) = response.tool_calls {
       for tool_call in tool_calls {
           // 1. Find specialist agent by tool name
           //    (need access to agent registry - may need to pass to Agent)

           // 2. Execute tool call:
           let result = execute_tool(tool_call.name, tool_call.arguments);

           // 3. Create tool result message:
           let tool_msg = Message::tool_result(tool_call.id, result);
           messages.push(tool_msg);
       }

       // 4. Make follow-up request with tool results
       let final_response = llm_adapter.stream_chat(updated_request, tx).await?;
   }
   ```

3. **Agent registry access**:
   - Agent currently doesn't have access to other agents
   - Need to either:
     - Pass agent registry to Agent via Arc
     - Have RustbotApi handle tool execution orchestration
     - Create a ToolExecutor service that has access to agents

   **RECOMMENDATION**: Have RustbotApi orchestrate tool calls
   - RustbotApi already has agent registry
   - Agent returns tool calls to RustbotApi
   - RustbotApi executes tools by routing to specialist agents
   - RustbotApi makes follow-up request with results

### Short Term:
1. Add error handling for tool execution failures
2. Handle multiple sequential tool calls properly
3. Add tool call timeout handling
4. Test with real specialist agents (web_search, etc.)
5. Add tool call logging/debugging

### Future Enhancements:
1. Support parallel tool calls
2. Implement streaming with tool calls (upgrade to full streaming support)
3. Add tool call caching/memoization
4. Add tool call usage tracking and analytics
5. Support tool call retries with exponential backoff

## Technical Debt

- **Unused field warnings**: ToolCallDelta fields not yet used (will be used in routing)
- **TODO comments**: Added for tool routing implementation
- **Streaming architecture**: Current design doesn't cleanly support mid-stream tool calls

## Notes

- Tool calls are more complex in streaming than initially estimated
- May need architectural refactor for clean tool call support
- Consider using non-streaming `complete_chat` for tool-enabled requests initially
- Full streaming with tools can be added as enhancement later

---

**Status**: Tool detection implemented, routing architecture needs design decision before proceeding.
