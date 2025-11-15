# Empty Message Bug Investigation - 2025-11-13

## Problem Statement

User reports error STILL occurring:
```
messages.6: all messages must have non-empty content except for the optional final assistant message
```

Previous fix (empty content validation in `add_assistant_response()`) did NOT work. The error moved from messages.2 to messages.6, indicating empty messages are accumulating over multiple interactions.

## Investigation Findings

### Code Locations Where Messages Are Added to History

Found **6 locations** in `src/api.rs` where messages are added:

1. **Line 201**: User message addition
   - ✅ Has validation: Always has content (from user input)

2. **Line 249**: Assistant message with tool calls
   - ⚠️ POTENTIAL BUG: Added validation to check `content.is_empty() && tool_calls.is_none()`
   - Previous code had NO validation here

3. **Line 282**: Tool result messages
   - ⚠️ POTENTIAL BUG: Tool results CAN be empty
   - Added warning log when empty

4. **Line 351** (`send_message_blocking`): User message
   - ✅ Has validation: Always has content

5. **Line 377** (`send_message_blocking`): Assistant final response
   - ✅ Has validation: Checks `!full_response.is_empty()`

6. **Line 447** (`add_assistant_response`): Final assistant response
   - ✅ Has validation: Checks `!response.is_empty()`

### Root Cause Hypothesis

**Most Likely Cause**: Tool execution flow adds messages to history in wrong order or with empty content.

The flow is:
1. User sends message → Added to history ✅
2. Agent detects tool calls needed
3. **Assistant message with tool_calls** → Added to history (line 249)
   - This message SHOULD have either:
     - Non-empty content (placeholder text), OR
     - tool_calls array present
4. Tool results → Added to history (line 282)
5. Final response → Streamed and added (line 447)

**Bug Location**: Line 246-250 validation was MISSING before this fix.

## Debugging Strategy Applied

### Aggressive Logging Added

Added detailed logging at EVERY message addition point:

```rust
tracing::debug!("📝 [HISTORY] Adding {TYPE} - content_len: {}, total_history: {}", ...)
```

This will show:
- Exact order of message additions
- Content length of each message
- Total history size after each addition

### Defensive Validation Added

**Line 246-250**: Check assistant message with tool calls
```rust
if assistant_msg.content.is_empty() && assistant_msg.tool_calls.is_none() {
    tracing::error!("❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!");
} else {
    self.message_history.push_back(assistant_msg.clone());
}
```

**Line 278-280**: Warn about empty tool results
```rust
if result.is_empty() {
    tracing::warn!("⚠️  [HISTORY] Tool result for {} is EMPTY - adding anyway (required for conversation flow)", tool_call.id);
}
```

### API Request Logging Enhanced

Added detailed message dump in `src/llm/openrouter.rs` line 71-80:
- Shows EVERY message being sent to API
- Displays role, content_len, has_tool_calls, has_tool_call_id
- Logs full JSON request

## Next Steps

1. **Run the app with RUST_LOG=debug** to capture all logs
2. **Trigger the bug** by:
   - Sending a message that triggers tool calls
   - Having multiple exchanges with tool execution
3. **Analyze logs** to find:
   - Which message addition creates the empty entry
   - What the message_history looks like before the API error
   - Whether validation is catching and blocking empty messages

## Files Modified

- `/Users/masa/Projects/rustbot/src/api.rs` (lines 198-201, 236-250, 269-282, 438-450)
- `/Users/masa/Projects/rustbot/src/llm/openrouter.rs` (lines 71-80)

## Expected Outcome

With aggressive logging, we should see in the logs:
- **If working**: "📝 [HISTORY] Adding..." for each valid message
- **If broken**: "❌ [HISTORY] BLOCKED: Assistant message has EMPTY content..." message, followed by the message.6 error

This will definitively identify whether:
1. The validation is being executed
2. Empty messages are being blocked
3. There's a code path we missed

## Build Status

- ✅ Release build successful
- ⚠️ 31 warnings (unrelated to this fix)
- Binary ready for testing at `target/release/rustbot`
