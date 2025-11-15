# Tool Calling Message Format Fix - 2025-11-13

## Session Overview
Fixed critical bug in tool execution where Anthropic API was rejecting messages due to incorrect message format. Implemented custom serialization to transform OpenAI-compatible message format to Anthropic's content block format.

## Problem Statement

**Error:**
```
"messages.2.content.0: unexpected `tool_use_id` found in `tool_result` blocks: toolu_vrtx_011f4dtw2pHcnc92YobnXXeN. Each `tool_result` block must have a corresponding `tool_use` block in the previous message."
```

**Root Cause:**
- Internal Message type uses OpenAI format (`tool_calls`, `tool_call_id`)
- Anthropic API expects different format (content blocks with `tool_use`, `tool_use_id`)
- Tool result messages need role "user" (not "tool") in Anthropic format
- Tool IDs were not properly linked between tool_use and tool_result blocks

## Solution Implemented

### Custom Message Serializer
Created `serialize_messages_for_anthropic()` function in `src/llm/openrouter.rs` that transforms messages during serialization:

**Transformation Rules:**

1. **Tool Result Messages** (role="tool"):
   ```rust
   // Before: OpenAI format
   Message { role: "tool", content: "result", tool_call_id: "toolu_xxx" }

   // After: Anthropic format
   {
     "role": "user",
     "content": [{
       "type": "tool_result",
       "tool_use_id": "toolu_xxx",
       "content": "result"
     }]
   }
   ```

2. **Assistant Messages with Tool Calls**:
   ```rust
   // Before: OpenAI format
   Message {
     role: "assistant",
     content: "I'll help",
     tool_calls: [ToolCall { id, name, arguments }]
   }

   // After: Anthropic format
   {
     "role": "assistant",
     "content": [
       {"type": "text", "text": "I'll help"},
       {"type": "tool_use", "id": "...", "name": "...", "input": {...}}
     ]
   }
   ```

3. **Regular Messages**: Keep simple string content format

### Implementation Details

**File Modified:** `src/llm/openrouter.rs`

**Key Changes:**
1. Added `#[serde(serialize_with = "serialize_messages_for_anthropic")]` to `ApiRequest.messages`
2. Implemented custom serializer with proper content block structure
3. Ensured tool_use_id matches between tool_use and tool_result blocks
4. Changed tool result role from "tool" to "user" per Anthropic requirements

## Testing

### Tests Added

1. **`test_anthropic_tool_execution_sequence`**
   - Tests the exact 3-message sequence: user → assistant (tool_use) → user (tool_result)
   - Verifies tool_use_id matches between blocks
   - Ensures role="user" for tool results

2. **`test_serialize_messages_for_anthropic_format`**
   - Comprehensive test of all message format conversions
   - Validates JSON structure matches Anthropic's expectations
   - Tests content block array structure

### Test Results
```bash
cargo test test_anthropic_tool_execution_sequence
# Result: ok. 1 passed

cargo test test_serialize_messages_for_anthropic_format
# Result: ok. 1 passed

cargo test --lib
# Result: ok. 46 passed; 0 failed
```

## Files Modified

- `src/llm/openrouter.rs` (568 lines changed)
  - Added `serialize_messages_for_anthropic()` function
  - Added custom serializer to ApiRequest
  - Added comprehensive tests

## Documentation Created

- `docs/TOOL_CALLING_FIX.md` - Detailed explanation of problem, solution, and format differences
- `docs/progress/2025-11-13-tool-calling-format-fix.md` (this file)

## Technical Details

### Message Sequence Flow

**Correct Anthropic Format:**
```json
{
  "messages": [
    {"role": "user", "content": "What's 2+2?"},
    {
      "role": "assistant",
      "content": [
        {"type": "text", "text": "Let me calculate"},
        {"type": "tool_use", "id": "toolu_xxx", "name": "calc", "input": {...}}
      ]
    },
    {
      "role": "user",
      "content": [
        {"type": "tool_result", "tool_use_id": "toolu_xxx", "content": "4"}
      ]
    }
  ]
}
```

### Key Insights

1. **Provider-Specific Formats**: Different LLM providers have incompatible message formats for tool calling
2. **Serialization Point**: Best place to handle transformation is during serialization (not modifying internal types)
3. **Content Blocks**: Anthropic uses structured content blocks instead of separate fields
4. **Role Changes**: Tool results must use role="user", not role="tool"

## Verification Steps

To verify the fix works:

1. Build project: `cargo build`
2. Run tests: `cargo test --lib`
3. Test tool execution in UI (requires API key and running app)
4. Check logs for successful tool execution without API errors

## Future Considerations

1. **Multi-Provider Support**: If adding direct OpenAI/other providers, may need:
   - Provider detection mechanism
   - Format transformation selector
   - Keep internal format provider-agnostic

2. **Streaming Tool Calls**: Current implementation handles complete_chat for tool detection
   - May need to extend for streaming tool call detection
   - Consider incremental tool call parsing in streaming mode

3. **Error Handling**: Add better error messages if tool_use_id mismatch occurs

## References

- [Anthropic Tool Use Documentation](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- [OpenRouter API Documentation](https://openrouter.ai/docs)
- OpenAI Function Calling format (for reference)

## Success Metrics

- ✅ All tests pass (46/46)
- ✅ Code compiles without errors
- ✅ Message format matches Anthropic's requirements
- ✅ Tool execution flow properly serializes messages
- ✅ Comprehensive documentation created

## Next Steps

1. Test in production with actual tool calls through UI
2. Monitor logs for any serialization issues
3. Consider adding debug logging for message transformation
4. Update progress tracking documents
