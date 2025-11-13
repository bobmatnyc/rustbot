# Tool Calling Message Format Fix

## Problem

When using tool calls (function calling) with Anthropic models through OpenRouter, the API was returning errors:

```
"messages.2.content.0: unexpected `tool_use_id` found in `tool_result` blocks: toolu_vrtx_011f4dtw2pHcnc92YobnXXeN. Each `tool_result` block must have a corresponding `tool_use` block in the previous message."
```

## Root Cause

**OpenAI vs Anthropic Message Format Difference**

Our internal `Message` type was using OpenAI's format:
- Tool calls: `tool_calls` array on assistant messages
- Tool results: `tool_call_id` field on "tool" role messages

But Anthropic API expects a different format:
- Tool calls: Content blocks with `type: "tool_use"`
- Tool results: Content blocks with `type: "tool_result"` and `tool_use_id` field
- Tool results must have role "user", not "tool"

## Solution

Implemented a custom serializer (`serialize_messages_for_anthropic`) that transforms messages when sending to OpenRouter/Anthropic API:

### 1. Tool Result Messages (role="tool")
**Before** (OpenAI format):
```json
{
  "role": "tool",
  "content": "Weather in NYC: 72°F",
  "tool_call_id": "toolu_xxx"
}
```

**After** (Anthropic format):
```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_xxx",
      "content": "Weather in NYC: 72°F"
    }
  ]
}
```

### 2. Assistant Messages with Tool Calls
**Before** (OpenAI format):
```json
{
  "role": "assistant",
  "content": "I'll check the weather",
  "tool_calls": [
    {
      "id": "toolu_xxx",
      "name": "get_weather",
      "arguments": {"location": "NYC"}
    }
  ]
}
```

**After** (Anthropic format):
```json
{
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "I'll check the weather"
    },
    {
      "type": "tool_use",
      "id": "toolu_xxx",
      "name": "get_weather",
      "input": {"location": "NYC"}
    }
  ]
}
```

### 3. Regular Messages
Simple messages remain unchanged with string content.

## Expected Message Sequence

For tool execution, the sequence must be:

1. **User message**: Initial request
2. **Assistant message**: Response with `tool_use` blocks (containing tool IDs)
3. **User message**: Tool results with `tool_result` blocks (referencing those IDs)

## Implementation Details

- **File**: `src/llm/openrouter.rs`
- **Function**: `serialize_messages_for_anthropic()`
- **Applied to**: `ApiRequest.messages` field via `#[serde(serialize_with = "...")]`

## Testing

Added comprehensive tests:
- `test_anthropic_tool_execution_sequence`: Verifies the exact message sequence
- `test_serialize_messages_for_anthropic_format`: Tests all message format conversions

Run tests:
```bash
cargo test test_anthropic_tool_execution_sequence
cargo test test_serialize_messages_for_anthropic_format
```

## References

- [Anthropic Tool Use Documentation](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- OpenRouter API (uses provider-specific formats)

## Future Considerations

If adding support for direct OpenAI or other providers, may need to:
1. Detect provider type
2. Apply appropriate message format transformation
3. Keep internal format provider-agnostic

Current implementation assumes Anthropic format (suitable for OpenRouter with Claude models).
