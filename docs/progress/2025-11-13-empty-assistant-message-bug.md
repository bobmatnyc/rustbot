# Empty Assistant Message Bug - 2025-11-13

## Issue

After implementing the conversation history fix, we're now hitting a different error:

```
messages.2: all messages must have non-empty content except for the optional final assistant message
```

## Root Cause

The error occurs in the second API call (`process_with_results`) after tool execution.

### Message Flow

1. **First API call** (`process_message_nonblocking` with tools):
   - ✅ Works correctly
   - Returns: `NeedsToolExecution { tool_calls, messages }`
   - `messages` includes: `[system, user, assistant_with_tool_calls]`

2. **Tool execution**:
   - ✅ Our fix correctly adds assistant message to conversation history
   - ✅ Executes tools and adds results to `messages`
   - `messages` now: `[system, user, assistant_with_tool_calls, tool_result]`

3. **Second API call** (`process_with_results`):
   - ❌ Fails with "messages.2: all messages must have non-empty content"
   - messages[2] = assistant message (index 0=system, 1=user, 2=assistant, 3=tool_result)
   - **Problem**: messages[2] apparently has empty content

## Analysis

From the debug logs:
- First request shows assistant message with content: "I'll search for today's top news for you."
- Tool result (message 3) has content: 1392 chars
- Second request fails saying messages[2] has empty content

**Hypothesis**: The `messages` array being passed to `process_with_results` contains an assistant message with empty content, separate from the one with tool_use blocks.

## Investigation Needed

Need to add debug logging to show:
1. Exact contents of `messages` array before calling `process_with_results`
2. What the assistant message contains (content + tool_calls)
3. Whether there are multiple assistant messages in the array

## Potential Causes

1. **Double assistant message**: The agent might be adding TWO assistant messages - one empty, one with tool_use
2. **Empty content in tool_use message**: The assistant message with tool_calls might have empty content despite our placeholder
3. **Serialization issue**: The serialization logic might be creating an empty message somehow

## Next Steps

1. Add debug logging to print entire `messages` array before `process_with_results`
2. Check if the assistant message in `messages` matches what we added to conversation history
3. Verify the agent's `process_message_nonblocking` is creating the assistant message correctly

## Code Locations

- `src/api.rs:196-228` - Tool execution and `process_with_results` call
- `src/agent/mod.rs:369-520` - `process_message_nonblocking` (creates assistant message)
- `src/agent/mod.rs:523-580` - `process_with_results` (receives messages array)

## Temporary Workaround

Could filter out empty messages before calling `process_with_results`:

```rust
// Before calling process_with_results
let filtered_messages: Vec<LlmMessage> = messages
    .into_iter()
    .filter(|m| !m.content.is_empty() || m.tool_calls.is_some())
    .collect();

let mut final_result_rx = agent.process_with_results(filtered_messages);
```

But this is a band-aid - we need to understand WHY there's an empty message.

## Status

- ✅ Conversation history fix working (assistant message added)
- ❌ Empty assistant message causing API rejection
- ⬜ Root cause not yet identified
- ⬜ Fix pending investigation
