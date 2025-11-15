# Debugging Empty Message Bug - User Guide

## What Was Done

I've added comprehensive debug logging and defensive validation to identify and fix the "empty content" bug you're experiencing.

## Changes Made

### 1. ✅ Aggressive Debug Logging

Every place in the code that adds messages to history now logs:
- Message type (USER, ASSISTANT, TOOL RESULT)
- Content length
- Current history size
- Special properties (tool_calls, tool_call_id)

**Search for**: `📝 [HISTORY]` in logs to see all message additions

### 2. ✅ Defensive Validation

Added validation at **line 246-250** in `src/api.rs` to prevent adding assistant messages that have:
- Empty content AND
- No tool_calls

This was the MISSING validation that could cause your error.

### 3. ✅ Enhanced API Logging

Before every request to Anthropic, the code now logs:
- All messages in the request
- Each message's role and content length
- Full JSON payload

**Search for**: `🔍 [API]` in logs to see what's being sent

### 4. ✅ Clear Chat Fix

Clear Chat button now:
- Clears UI messages
- Clears API conversation history
- Publishes `SystemCommand::ClearConversation` event

## How to Test

### Step 1: Rebuild

```bash
cd /Users/masa/Projects/rustbot
cargo build --release
```

### Step 2: Run with Debug Logging

```bash
RUST_LOG=debug ./target/release/rustbot 2>&1 | tee rustbot-debug.log
```

This will:
- Show all debug output in terminal
- Save to `rustbot-debug.log` file

### Step 3: Trigger the Bug

Try to reproduce the error:
1. Send messages that trigger tool calling
2. Have multiple exchanges
3. Wait for the "messages.6" error

### Step 4: Analyze Logs

If the error occurs, search the log file for:

```bash
# See all message additions
grep "[HISTORY]" rustbot-debug.log

# See blocked messages
grep "BLOCKED" rustbot-debug.log

# See what was sent to API
grep -A 20 "\[API\] Sending request" rustbot-debug.log
```

## What to Look For

### If the Fix Worked ✅

You'll see this in logs:
```
❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!
```

This means the validation CAUGHT the empty message and prevented it from being added.

**Result**: No API error!

### If Bug Still Occurs ❌

You'll see the Anthropic error:
```
messages.6: all messages must have non-empty content except for the optional final assistant message
```

Then look at the logs BEFORE the error to see:
1. All `📝 [HISTORY]` entries showing what was added
2. The `🔍 [API]` dump showing exactly what was sent
3. Which message (0-6) had empty content

**Share these logs with me** and I'll identify the exact code path causing the issue.

## Log Examples

### Successful Flow (No Error)

```
📝 [HISTORY] Adding USER message - content_len: 45, total_history: 1
📝 [HISTORY] Adding ASSISTANT message with tool calls - content_len: 35, tool_calls: 2, total_history: 2
📝 [HISTORY] Adding TOOL RESULT - tool_id: call_abc, result_len: 156, total_history: 3
📝 [HISTORY] Adding TOOL RESULT - tool_id: call_def, result_len: 203, total_history: 4
📝 [HISTORY] add_assistant_response called - response_len: 421, total_history: 4
📝 [HISTORY] Adding FINAL ASSISTANT response - content_len: 421, total_history: 5
🔍 [API] Sending request with 5 messages
🔍 [API] Message[0]: role=user, content_len=45, has_tool_calls=false, has_tool_call_id=false
🔍 [API] Message[1]: role=assistant, content_len=35, has_tool_calls=true, has_tool_call_id=false
🔍 [API] Message[2]: role=tool, content_len=156, has_tool_calls=false, has_tool_call_id=true
🔍 [API] Message[3]: role=tool, content_len=203, has_tool_calls=false, has_tool_call_id=true
🔍 [API] Message[4]: role=assistant, content_len=421, has_tool_calls=false, has_tool_call_id=false
```

### Blocked Empty Message

```
📝 [HISTORY] Adding USER message - content_len: 45, total_history: 1
❌ [HISTORY] BLOCKED: Assistant message has EMPTY content AND no tool_calls!
(Message NOT added to history)
```

### Empty Message Still Added (Bug Location Found)

```
📝 [HISTORY] Adding USER message - content_len: 45, total_history: 1
📝 [HISTORY] Adding ASSISTANT message - content_len: 0, tool_calls: 0, total_history: 2
(Empty message was added - this shows WHERE the bug is)
```

## Clear Chat Testing

Test the Clear Chat button:

1. Have a conversation with several messages
2. Click "Clear Chat"
3. Check logs for:
   ```
   🗑️  Clearing conversation - UI messages: X
   🗑️  Clearing conversation history (X messages)
   ```
4. Verify conversation history is actually cleared
5. Send a new message - it should start fresh with no previous context

## Quick Command Reference

```bash
# Build release
cargo build --release

# Run with debug logs
RUST_LOG=debug ./target/release/rustbot 2>&1 | tee rustbot-debug.log

# After error, analyze logs
grep "[HISTORY]" rustbot-debug.log > history.txt
grep "\[API\]" rustbot-debug.log > api-calls.txt
grep "BLOCKED\|ERROR" rustbot-debug.log > errors.txt
```

## What I Need If Bug Still Occurs

If you still get the error, please share:

1. **The exact error message** (which message index: messages.0, messages.6, etc.)
2. **All `[HISTORY]` logs** from before the error
3. **The `[API]` message dump** showing what was sent
4. **What you did** to trigger it (rough description)

This will let me identify the exact code location adding the empty message.

## Expected Outcome

**Most Likely**: The validation at line 246-250 will catch and block the empty assistant message, preventing the API error.

**If Not**: The comprehensive logging will show us EXACTLY where the empty message is coming from, allowing a targeted fix.

Either way, we'll solve this!
