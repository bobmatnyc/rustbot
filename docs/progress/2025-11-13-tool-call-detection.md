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

## Git Commits

```
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

**In Progress:**
- 🔄 Tool call routing architecture design
- 🔄 Tool call accumulation logic
- 🔄 Tool execution mechanism

**Pending:**
- ⏳ Complete tool call accumulation
- ⏳ Route tool calls to specialist agents
- ⏳ Execute tools and get results
- ⏳ Send tool results back to LLM
- ⏳ Stream final response to user
- ⏳ End-to-end testing

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
1. Decide on streaming architecture approach (recommend Option 4 above)
2. Implement tool call accumulation in stream handler
3. Add tool execution logic in Agent layer
4. Test with web_search agent

### Short Term:
1. Create tool result message format
2. Implement follow-up LLM request with tool result
3. Handle multiple sequential tool calls
4. Add error handling for tool failures

### Future Enhancements:
1. Support parallel tool calls
2. Implement proper streaming with tool calls (upgrade from Option 4 to Option 3)
3. Add tool call caching/memoization
4. Add tool call timeout handling
5. Add tool call usage tracking

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
