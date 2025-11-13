# Tool Calling Bug Analysis - 2025-11-13

## Status

Tool execution implementation is **working** but there's a critical bug in conversation history management that needs to be fixed.

## What's Working ✅

1. **Tool execution flow** - The core two-phase pattern is implemented correctly:
   - Phase 1: Agent detects tool calls via `complete_chat()`
   - Phase 2: API executes tools, then agent generates final response via `stream_chat()`

2. **Message serialization for Anthropic API** - Correctly converts to Anthropic's format:
   - Assistant messages with tool_calls → content blocks with `tool_use` type
   - Tool result messages → user messages with `tool_result` content blocks

3. **Current request handling** - The `messages` array passed to the API is structured correctly:
   ```
   1. System message (if present)
   2. ...context messages...
   3. User message
   4. Assistant message with tool_use blocks
   5. User message with tool_result blocks
   ```

## The Bug 🐛

###Location: `src/api.rs` lines 196-234

The bug is in how conversation history is maintained after tool execution. The code adds the tool result to `message_history` but **never adds the assistant message with tool calls**.

### Current Broken Flow:

```rust
// Line 180: User message added to history
self.message_history.push_back(LlmMessage::new("user", message));

// Lines 196-214: Tool execution
match agent_response_result {
    Ok(AgentResponse::NeedsToolExecution { tool_calls, mut messages }) => {
        for tool_call in tool_calls {
            let result = self.execute_tool(&tool_call.name, &args_str).await?;

            // ✅ Correctly adds tool result to messages (for current request)
            messages.push(LlmMessage::tool_result(tool_call.id.clone(), result));

            // ❌ BUG: Only adds tool result to conversation history
            //     Missing: self.message_history.push_back(ASSISTANT_WITH_TOOL_CALLS)
            self.message_history.push_back(LlmMessage::tool_result(tool_call.id, "Tool executed".to_string()));
        }

        // Process with results...
    }
}
```

### Result:

**Conversation history becomes:**
```
[..previous messages.., USER_MESSAGE, TOOL_RESULT]
                                    ↑ Missing assistant message here!
```

**But current request `messages` is correct:**
```
[..previous messages.., USER_MESSAGE, ASSISTANT_WITH_TOOL_CALLS, TOOL_RESULT]
```

### Why This Causes Errors:

When the **next** message is sent in the conversation, the context will be:
```
messages[0]: User: "what's the weather?"
messages[1]: Tool result: {...}  ← Missing the assistant's tool_use message!
messages[2]: User: "thanks, what about tomorrow?"
```

Claude's API then returns error:
```
messages.1.content.0: unexpected `tool_use_id` found in `tool_result` blocks.
Each `tool_result` block must have a corresponding `tool_use` block in the previous message.
```

## The Fix 🔧

### What Needs to Change:

In `src/api.rs` around line 196-214, we need to:

1. **Add the assistant message to conversation history** BEFORE adding tool results
2. **Add the final assistant response** to conversation history AFTER getting the final stream

### Proposed Solution:

```rust
Ok(AgentResponse::NeedsToolExecution { tool_calls, mut messages }) => {
    tracing::info!("Tool execution required: {} tools to execute", tool_calls.len());

    // ✅ NEW: Add assistant message with tool calls to conversation history
    // Extract the assistant message from the messages array (should be the last one)
    if let Some(assistant_msg) = messages.iter().rev().find(|m| m.role == "assistant") {
        self.message_history.push_back(assistant_msg.clone());
    }

    // Execute each tool call sequentially
    for tool_call in tool_calls {
        tracing::info!("Executing tool: {} (ID: {})", tool_call.name, tool_call.id);

        let args_str = tool_call.arguments.to_string();
        let result = self.execute_tool(&tool_call.name, &args_str).await?;

        tracing::info!("Tool {} completed, result length: {} chars", tool_call.name, result.len());

        // Add tool result to messages for the current request
        messages.push(LlmMessage::tool_result(tool_call.id.clone(), result.clone()));

        // ✅ FIXED: Add actual tool result with content to conversation history
        self.message_history.push_back(LlmMessage::tool_result(tool_call.id, result));
    }

    // Make follow-up request...
    let final_stream = ...;

    // ✅ NEW: Add final assistant response to conversation history
    // (This would require collecting the streamed response, which adds complexity)

    Ok(final_stream)
}
```

### Additional Considerations:

1. **Stream collection** - The final assistant response is streamed, so we'd need to collect it to add to history. Options:
   - Tee the stream (clone chunks as they're sent)
   - Buffer the entire response before streaming
   - Add a callback when stream completes

2. **Tool result content** - Currently storing "Tool executed" placeholder. Should store actual result for better context.

3. **History size management** - Already handled by trimming in `send_message()`, should continue to work.

## Test Cases Needed 🧪

1. **Single tool call conversation**
   - User asks question requiring tool
   - Verify tool executes
   - Verify response includes tool result

2. **Multi-turn with tools**
   - User asks question requiring tool
   - Tool executes successfully
   - User asks follow-up question
   - ⚠️ **This currently fails** - will get tool_use_id mismatch error

3. **Multiple tools in one message**
   - User asks question requiring multiple tools
   - All tools execute sequentially
   - Final response incorporates all results

4. **Conversation history persistence**
   - After tool execution, check `message_history`
   - Should contain: user msg → assistant w/ tool calls → tool results
   - Next message should use this as context

## Error Messages Seen 🚨

### Error 1: Empty Content
```
messages.2: all messages must have non-empty content except for the optional final assistant message
```
**Cause**: Tool result with empty content being sent
**Status**: Should be prevented by validation in serialization logic

### Error 2: Tool Use ID Mismatch
```
messages.2.content.0: unexpected `tool_use_id` found in `tool_result` blocks: toolu_vrtx_011f4dtw2pHcnc92YobnXXeN.
Each `tool_result` block must have a corresponding `tool_use` block in the previous message.
```
**Cause**: Missing assistant message with tool_use blocks in conversation history
**Status**: **This is the bug we need to fix**

## Files Involved 📁

- `src/api.rs` - Main API interface, contains the bug (lines 196-234)
- `src/agent/mod.rs` - Agent implementation, correctly creates message structure (lines 369-520)
- `src/llm/openrouter.rs` - Anthropic message serialization, working correctly (lines 247-400)
- `src/llm/types.rs` - Message types and structures (lines 125-175)

## Next Steps 📋

1. ✅ Document the bug (this file)
2. ⬜ Implement the fix in `src/api.rs`
3. ⬜ Test single tool call (should work)
4. ⬜ Test multi-turn with tools (this will validate the fix)
5. ⬜ Add proper error handling for edge cases
6. ⬜ Consider adding debug logging for message history state
7. ⬜ Update tests to cover multi-turn tool conversations

## Success Criteria ✓

- [ ] Single tool call works
- [ ] Multi-turn conversation after tool use works
- [ ] Message history maintains correct structure:
  - User message
  - Assistant message with tool_use
  - Tool result
  - Assistant final response
- [ ] No tool_use_id mismatch errors
- [ ] No empty content errors
- [ ] Conversation history stays within size limits

## Timeline Estimate ⏱️

- **Fix implementation**: 30-45 minutes
- **Testing**: 30 minutes
- **Documentation**: 15 minutes
- **Total**: ~1.5-2 hours

## Additional Notes 📝

- The current working instance (bash 0cea87) shows the app launches successfully
- Tool registry is working (1 tool: web_search)
- Agent loading is working (2 agents: assistant, web_search)
- The serialization logic has comprehensive tests that are passing
- The issue only manifests in multi-turn conversations with tools

## References 🔗

- [Anthropic Tool Use Documentation](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- `docs/TOOL_CALLING_FIX.md` - Previous implementation notes
- `docs/progress/2025-11-13-tool-calling-format-fix.md` - Related fixes
- `docs/progress/2025-11-13-tool-execution-fix.md` - Tool execution implementation
