# Fix: Empty Content in Tool Result Messages

## Problem Summary

When tool results were being serialized for the Anthropic API (via OpenRouter), empty content was causing API errors:

```
Error: messages.2: all messages must have non-empty content except for the optional final assistant message
```

## Root Cause

The Anthropic API has strict validation requirements:
1. **All messages must have non-empty content** (except the optional final assistant message)
2. This applies to:
   - User messages
   - Assistant messages (must have either text or tool_use blocks)
   - Tool result messages (must have non-empty result content)

The bug occurred when a `Message` with role="tool" had an **empty `content` field**. During serialization, this empty content was being placed into the `tool_result` block, creating an invalid message for Anthropic's API.

## Solution

Added comprehensive validation in `serialize_messages_for_anthropic()` to catch empty content **before** sending to the API:

### 1. Tool Result Validation
```rust
// Lines 308-319 in src/llm/openrouter.rs
if message.content.is_empty() {
    tracing::error!("❌ Tool result message {} has EMPTY content! tool_use_id={}", idx, tool_use_id);
    return Err(serde::ser::Error::custom(format!(
        "Tool result message {} has empty content (tool_use_id: {})",
        idx, tool_use_id
    )));
}
```

### 2. Assistant with Tool Calls Validation
```rust
// Lines 356-363 in src/llm/openrouter.rs
if content_blocks.is_empty() {
    return Err(serde::ser::Error::custom(format!(
        "Assistant message {} has tool_calls but generated no content blocks! tool_calls: {:?}, content: {:?}",
        idx, message.tool_calls, message.content
    )));
}
```

### 3. Regular Message Validation
```rust
// Lines 386-392 in src/llm/openrouter.rs
if message.content.is_empty() {
    tracing::error!("❌ Regular message {} (role: {}) has EMPTY content!", idx, message.role);
    return Err(serde::ser::Error::custom(format!(
        "Message {} (role: {}) has empty content - Anthropic requires non-empty content",
        idx, message.role
    )));
}
```

### 4. Early Warning Logs
```rust
// Lines 280-284 in src/llm/openrouter.rs
// Validate: Anthropic requires all messages (except final assistant) to have non-empty content
if message.content.is_empty() && message.role != "tool" && message.tool_calls.is_none() {
    tracing::warn!("⚠️  Message {} has empty content (role: {})", idx, message.role);
}
```

### 5. Debug Logging
```rust
// Lines 67-70 in src/llm/openrouter.rs
// DEBUG: Log the serialized request to see what's actually being sent
if let Ok(json) = serde_json::to_string_pretty(&api_request) {
    tracing::debug!("🔍 Sending API request:\n{}", json);
}
```

## Testing

Added two new tests to verify the fix:

### Test 1: Empty Tool Result Rejection
```rust
#[test]
fn test_empty_tool_result_is_rejected()
```
Verifies that tool result messages with empty content are caught during serialization and produce a descriptive error message.

### Test 2: Empty Regular Message Rejection
```rust
#[test]
fn test_empty_regular_message_is_rejected()
```
Verifies that regular messages (user/assistant) with empty content are rejected.

## Impact

**Before Fix:**
- Empty tool results would cause cryptic API errors from Anthropic
- Debugging required inspecting network traffic to see the actual JSON sent
- No way to identify which message had empty content

**After Fix:**
- **Fail-fast validation**: Empty content is caught during serialization
- **Descriptive errors**: Error messages include message index, role, and tool_use_id (if applicable)
- **Debug logging**: Full request JSON logged at debug level for troubleshooting
- **Proactive warnings**: Early warnings for messages that might cause issues

## Next Steps to Debug the Original Issue

Since the serialization code was already correct (tests prove it), the bug must be in how `Message::tool_result()` is being called. The validation we added will now help identify exactly where empty content is coming from.

**To debug:**
1. Run the application with `RUST_LOG=debug`
2. Watch for error logs: `❌ Tool result message X has EMPTY content!`
3. Check the calling code where `Message::tool_result()` is created
4. Verify that the `content` parameter is not an empty string

**Likely culprits to investigate:**
- `src/api.rs:210` - Where tool results are added to message history
- Tool execution functions in specialist agents
- Any code that creates `Message::tool_result()` with potentially empty strings

## Files Modified

- `src/llm/openrouter.rs`:
  - Added validation for empty content in tool results (lines 308-319)
  - Added validation for empty content in assistant messages (lines 356-363)
  - Added validation for empty content in regular messages (lines 386-392)
  - Added debug logging for API requests (lines 67-70)
  - Added early warning logs (lines 280-284)
  - Added 2 new tests (lines 771-837)

## Verification

All tests pass:
```bash
cargo test --lib llm::openrouter::tests
# running 8 tests
# test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out
```

## Anthropic API Requirements Reference

From Anthropic's documentation:
> All messages must have non-empty content except for the optional final assistant message.

**Tool Result Format:**
```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_xxx",
      "content": "The actual result here"  // ← Must be non-empty
    }
  ]
}
```

**Assistant with Tool Use Format:**
```json
{
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "I'll use this tool"  // Optional if tool_use blocks present
    },
    {
      "type": "tool_use",
      "id": "toolu_xxx",
      "name": "tool_name",
      "input": { "param": "value" }
    }
  ]
}
```

## Related Code

- **Message struct**: `src/llm/types.rs:115-158`
- **Tool execution**: `src/api.rs:196-218`
- **Message serialization**: `src/llm/openrouter.rs:269-405`
- **Agent processing**: `src/agent/mod.rs:417-447`
