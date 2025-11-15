# Debug Logging Enhancement and Clear Chat Fix - 2025-11-13

## Session Overview

Implemented comprehensive debug logging to diagnose the "empty content" bug and fixed the Clear Chat functionality to properly publish events.

## Features Implemented

### 1. Aggressive Debug Logging for Message History

**Problem**: User reports empty message error still occurring at messages.6, despite previous fix. Need to trace exactly where empty messages are being added.

**Solution**: Added detailed logging at every message addition point in the codebase.

#### Changes Made

**File**: `src/api.rs`

**Line 198-201**: User message logging
```rust
let user_msg = LlmMessage::new("user", message);
tracing::debug!("📝 [HISTORY] Adding USER message - content_len: {}, total_history: {}",
    user_msg.content.len(), self.message_history.len() + 1);
self.message_history.push_back(user_msg);
```

**Line 240-250**: Assistant message with tool calls (WITH VALIDATION!)
```rust
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
```

**Line 274-282**: Tool result logging
```rust
tracing::debug!("📝 [HISTORY] Adding TOOL RESULT - tool_id: {}, result_len: {}, total_history: {}",
    tool_call.id, result.len(), self.message_history.len() + 1);

// DEFENSIVE: Validate tool result has content
if result.is_empty() {
    tracing::warn!("⚠️  [HISTORY] Tool result for {} is EMPTY - adding anyway (required for conversation flow)", tool_call.id);
}

self.message_history.push_back(LlmMessage::tool_result(tool_call.id.clone(), result));
```

**Line 439-450**: Final assistant response logging
```rust
tracing::debug!("📝 [HISTORY] add_assistant_response called - response_len: {}, total_history: {}",
    response.len(), self.message_history.len());

if !response.is_empty() {
    tracing::debug!("📝 [HISTORY] Adding FINAL ASSISTANT response - content_len: {}, total_history: {}",
        response.len(), self.message_history.len() + 1);
    self.message_history.push_back(LlmMessage::new("assistant", response));
} else {
    tracing::warn!("⚠️  [HISTORY] BLOCKED: Skipping empty assistant message in add_assistant_response");
}
```

### 2. Enhanced API Request Logging

**File**: `src/llm/openrouter.rs` (lines 71-80)

Added detailed message dump before every API request:

```rust
tracing::debug!("🔍 [API] Sending request with {} messages", api_request.messages.len());
for (idx, msg) in api_request.messages.iter().enumerate() {
    tracing::debug!("🔍 [API] Message[{}]: role={}, content_len={}, has_tool_calls={}, has_tool_call_id={}",
        idx, msg.role, msg.content.len(),
        msg.tool_calls.is_some(),
        msg.tool_call_id.is_some());
}
if let Ok(json) = serde_json::to_string_pretty(&api_request) {
    tracing::debug!("🔍 [API] Full request JSON:\n{}", json);
}
```

This shows:
- Total message count
- Each message's role, content length, and special fields
- Full JSON payload being sent to API

### 3. Clear Chat Event Publishing

**Problem**: Clear Chat button only cleared UI state, not the API conversation history or events.

**Solution**: Implement full clear flow with event publishing.

#### Changes Made

**File**: `src/api.rs` (lines 397-411)

```rust
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
```

**File**: `src/main.rs` (lines 374-388)

```rust
fn clear_conversation(&mut self) {
    tracing::info!("🗑️  Clearing conversation - UI messages: {}", self.messages.len());

    // Clear UI state
    self.messages.clear();
    self.current_response.clear();
    self.context_tracker.update_counts(0, 0);

    // Clear API conversation history and publish event
    let api = Arc::clone(&self.api);
    self.runtime.spawn(async move {
        let mut api_guard = api.lock().await;
        api_guard.clear_history();
    });
}
```

## Defensive Validation Added

### Critical Fix: Line 246-250 in api.rs

**Previously**: Assistant messages with tool calls were added to history WITHOUT validation.

**Now**: Validates that the message has EITHER content OR tool_calls before adding:

```rust
if assistant_msg.content.is_empty() && assistant_msg.tool_calls.is_none() {
    tracing::error!("❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!");
} else {
    self.message_history.push_back(assistant_msg.clone());
}
```

This prevents adding empty assistant messages that would cause the Anthropic API error.

## Testing Strategy

### How to Reproduce and Debug

1. **Run with debug logging**:
   ```bash
   RUST_LOG=debug ./target/release/rustbot 2>&1 | tee rustbot-debug.log
   ```

2. **Trigger the bug**:
   - Send a message that uses tool calling
   - Have multiple exchanges with tool execution
   - Look for the "messages.6" error

3. **Analyze logs**:
   - Search for `[HISTORY]` to see all message additions
   - Look for `❌ [HISTORY] BLOCKED` to see if validation caught empty messages
   - Check `[API]` logs to see what's being sent to Anthropic
   - If error occurs, trace back to see which message.6 had empty content

### Expected Log Output

**Successful Flow**:
```
📝 [HISTORY] Adding USER message - content_len: 45, total_history: 1
📝 [HISTORY] Adding ASSISTANT message with tool calls - content_len: 35, tool_calls: 2, total_history: 2
📝 [HISTORY] Adding TOOL RESULT - tool_id: call_abc123, result_len: 156, total_history: 3
📝 [HISTORY] Adding TOOL RESULT - tool_id: call_def456, result_len: 203, total_history: 4
📝 [HISTORY] add_assistant_response called - response_len: 421, total_history: 4
📝 [HISTORY] Adding FINAL ASSISTANT response - content_len: 421, total_history: 5
```

**Blocked Empty Message**:
```
❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!
⚠️  [HISTORY] BLOCKED: Skipping empty assistant message in add_assistant_response
```

## Files Modified

1. `/Users/masa/Projects/rustbot/src/api.rs`
   - Lines 198-201: User message logging
   - Lines 240-250: Assistant with tool calls validation + logging
   - Lines 274-282: Tool result logging
   - Lines 397-411: Clear history with event publishing
   - Lines 439-450: Final response logging

2. `/Users/masa/Projects/rustbot/src/llm/openrouter.rs`
   - Lines 71-80: Enhanced API request logging

3. `/Users/masa/Projects/rustbot/src/main.rs`
   - Lines 374-388: Clear conversation with API call

## Root Cause Hypothesis

The most likely source of empty messages is **line 239-250** (assistant message with tool calls), which previously had NO validation. The defensive check added should prevent this.

## Next Steps for User

1. **Rebuild and run** with the new binary:
   ```bash
   cargo build --release
   RUST_LOG=debug ./target/release/rustbot
   ```

2. **Try to reproduce** the empty content error

3. **Share the logs** showing:
   - All `[HISTORY]` entries before the error
   - The exact error message
   - The `[API]` message dump showing what was sent

4. **If error still occurs**, the logs will definitively show:
   - Which message index (0-6) had empty content
   - Whether our validation caught it
   - What the message's role and properties were

## Build Status

- ✅ Release build successful
- ✅ No compilation errors
- ⚠️ Some warnings (unrelated to this fix)
- Binary ready: `target/release/rustbot`

## Success Criteria

- ✅ All message additions now logged with details
- ✅ Validation prevents empty assistant messages without tool_calls
- ✅ Clear Chat publishes SystemCommand::ClearConversation event
- ✅ Empty tool results logged but allowed (may be valid)
- ✅ Comprehensive API request logging shows full message array

## Expected Outcome

With these changes, either:
1. **Bug is fixed**: Validation blocks the empty message, preventing the API error
2. **Bug location identified**: Logs show exactly which message addition creates the empty entry
3. **New bug found**: Logs reveal a code path we didn't know about

In all cases, we'll have definitive diagnostic information to fix the root cause.
