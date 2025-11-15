# Tool Calling Debug Enhancement

**Date**: 2025-11-15
**Status**: 🔍 Enhanced logging added, ready for testing
**Issue**: Web search tool not being called by LLM despite proper configuration

## Problem Summary

User reported that when asking "What's news?", the assistant responds with:
```
I'll search for the latest news updates for you. Please hold on a moment
while I gather the most current information...
```

Instead of actually calling the `web_search` tool to get real results.

## Investigation Completed

### ✅ Verified Components

1. **Agent Configuration**: Both `assistant` and `web_search` agents load successfully
2. **Tool Registration**: Tool registry shows "1 tools available"
3. **Tool Definitions**: `web_search` tool is properly defined with clear description
4. **Tool Provisioning**: Primary agent receives tools in `send_message`
5. **Tool Execution Pipeline**: Complete infrastructure exists for tool execution

### ❓ Unknown: LLM Behavior

The one thing we haven't verified yet is:
- **Does the LLM actually call the tool when it should?**
- **Or does it respond directly instead?**

## Enhanced Logging Added

### Changes Made

**File**: `src/llm/openrouter.rs`

Added comprehensive logging at three critical points:

#### 1. Before Sending Request (Lines 209-225)

```rust
// Log detailed tool information for debugging
if let Some(ref tools) = api_request.tools {
    tracing::info!("🔧 [LLM] Sending {} tools to API:", tools.len());
    for tool in tools {
        tracing::info!("   - Tool: {}", tool.function.name);
        tracing::info!("     Description: {}", tool.function.description);
    }
} else {
    tracing::info!("🔧 [LLM] No tools in request");
}

if let Some(ref choice) = api_request.tool_choice {
    tracing::info!("🎯 [LLM] tool_choice: {:?}", choice);
} else {
    tracing::info!("🎯 [LLM] tool_choice: auto (default)");
}
```

**What This Shows**:
- Exactly which tools are being sent to OpenRouter API
- The tool_choice parameter value
- Confirms tools are in the request

#### 2. After Receiving Response (Lines 255-283)

```rust
// Convert OpenRouter API format to our internal format
let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
    tracing::info!("📞 [LLM] Response contains {} tool call(s)", calls.len());
    calls
        .iter()
        .filter_map(|api_call| {
            tracing::info!("   - Tool call: {} (id: {})", api_call.function.name, api_call.id);
            // ... rest of parsing ...
        })
        .collect()
});

if tool_calls.is_none() {
    tracing::info!("📞 [LLM] Response contains NO tool calls - LLM responded directly");
}
```

**What This Shows**:
- Whether the LLM decided to call tools or respond directly
- If tools were called, which ones and with what IDs
- **This is the KEY diagnostic for the issue**

### Log Symbols Guide

| Symbol | Meaning | What to Look For |
|--------|---------|------------------|
| 🔧 | Tools being sent | Should show "Sending 1 tools to API" with web_search |
| 🎯 | Tool choice config | Should show "auto" (LLM can choose) |
| 📞 | LLM response analysis | **CRITICAL**: Shows if tool was called or not |

## Testing Instructions

### Manual Test

1. **Build the updated code**:
   ```bash
   cargo build
   ```

2. **Run with INFO logging**:
   ```bash
   RUST_LOG=info ./target/debug/rustbot > /tmp/rustbot_tool_debug.log 2>&1
   ```

3. **In the GUI, ask a news question**:
   - "What's the latest AI news?"
   - "What's happening with SpaceX?"
   - "Tell me today's top news stories"

4. **Close the app and analyze logs**:
   ```bash
   ./analyze_logs.sh
   ```

### Expected Log Output (Success)

If tool calling works correctly, you should see:

```
🔧 [LLM] Sending 1 tools to API:
   - Tool: web_search
     Description: Search the web for current, real-time information...

🎯 [LLM] tool_choice: auto

📞 [LLM] Response contains 1 tool call(s)
   - Tool call: web_search (id: call_xyz123)

Executing tool: web_search with args: {"query": "latest AI news"}
```

### Expected Log Output (Current Bug)

If the bug persists, you'll see:

```
🔧 [LLM] Sending 1 tools to API:
   - Tool: web_search
     Description: Search the web for current, real-time information...

🎯 [LLM] tool_choice: auto

📞 [LLM] Response contains NO tool calls - LLM responded directly
```

## Diagnostic Decision Tree

```
Run test → Check logs → See result
                ↓
        ┌───────┴────────┐
        ↓                ↓
    NO TOOLS         TOOLS SENT
    IN REQUEST       TO API (✅)
        ↓                ↓
   Check tool     ┌──────┴───────┐
   registration   ↓              ↓
   in api.rs   NO TOOL       TOOL CALLS
               CALLS         FOUND (✅)
                  ↓              ↓
            LLM CHOSE      Tool execution
            NOT TO USE     successful!
            TOOLS          Test web search
                ↓          results
          Root causes:
          1. tool_choice = "auto" (LLM can ignore)
          2. Instructions not strong enough
          3. Query phrasing issue
          4. Model behavior change
```

## Potential Root Causes & Solutions

### If "NO tool calls" in logs:

**Cause 1: tool_choice = "auto"**
- Current: LLM can choose to use tools or not
- Solution: Try `tool_choice = "required"` (forces tool use)
- Location: `src/agent/mod.rs:425`
- **Downside**: Would force tool use even for non-tool queries

**Cause 2: Instructions not strong enough**
- Current instructions emphasize tool use
- Solution: Make even more explicit or use system-level enforcement
- Already quite strong in `assistant.json`

**Cause 3: Query interpretation**
- "What's news?" might be ambiguous
- LLM might think user is asking "what does 'news' mean?"
- Solution: Test with more explicit queries:
  - "Search the web for latest AI news"
  - "What are today's top news stories?"

**Cause 4: Model behavior**
- Claude Sonnet 4.5 might have different tool calling behavior
- Solution: Test with different model or check OpenRouter docs

### If "TOOLS SENT" not in logs:

**Cause**: Tools not reaching LLM API
- Check `src/api.rs:167-177` tool provisioning
- Check `src/agent/mod.rs:422-425` tool request building
- Verify `is_primary = true` for assistant

### If "Tool execution failed":

**Cause**: Specialist agent not working
- Check web_search agent configuration
- Verify OpenRouter API key
- Check web search plugin format

## Next Steps

### Immediate (User Action Required)

1. **Run the test** with enhanced logging
2. **Analyze logs** using `./analyze_logs.sh`
3. **Report findings**: Which scenario from above occurred?

### If Tool Calls Not Happening

**Option A: Force tool use (aggressive)**
```rust
// In src/agent/mod.rs:425, change:
request.tool_choice = Some("auto".to_string());
// To:
request.tool_choice = Some("required".to_string());
```

**Option B: Enhanced instructions (gentle)**
Add to assistant instructions:
```
IMPORTANT: When you see keywords like "latest", "today", "current", "news",
you MUST call the web_search tool. DO NOT respond without searching first.
```

**Option C: Query-based detection (smart)**
Analyze user query for trigger words and set tool_choice dynamically:
```rust
let requires_web_search = user_message.to_lowercase()
    .contains("latest") || user_message.contains("today") || ...;

request.tool_choice = if requires_web_search {
    Some("required".to_string())
} else {
    Some("auto".to_string())
};
```

### If Tool Calls Work But Execution Fails

1. Check specialist agent web_search configuration
2. Verify OpenRouter web search plugin format
3. Test with explicit web search via API directly

## Files Modified

- `src/llm/openrouter.rs`: Added enhanced logging (lines 209-283)

## Files Created

- `test_tool_calling_debug.sh`: Interactive test script
- `analyze_logs.sh`: Log analysis script
- This documentation file

## Related Documents

- `docs/progress/2025-11-14-web-search-verification.md` - System architecture verification
- `docs/progress/2025-11-13-tool-execution-complete.md` - Tool execution implementation
- `agents/presets/assistant.json` - Assistant configuration with tool instructions
- `agents/presets/web_search.json` - Web search specialist configuration

## Conclusion

Enhanced logging is now in place to diagnose exactly why the web_search tool isn't being called. The logs will show us:

1. ✅ Tools are being sent (we expect yes)
2. ✅ Tool choice is set to "auto"
3. ❓ **Whether LLM calls the tool or responds directly** ← KEY UNKNOWN

Once we run the test and analyze logs, we'll know the exact root cause and can implement the appropriate fix.

**Action Required**: User needs to run rustbot with the test query and share the log analysis results.
