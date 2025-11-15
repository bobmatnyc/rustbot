# Tool Calling Diagnostic Test

**Date**: 2025-11-15
**Issue**: Assistant not calling web_search tool in GUI
**Status**: Diagnostic logging added, ready for testing

## Quick Test

### Step 1: Rebuild
```bash
cargo build
```

### Step 2: Run with Logging
```bash
RUST_LOG=info ./target/debug/rustbot 2>&1 | tee /tmp/rustbot_gui_test.log
```

### Step 3: Test Query
In the GUI, send this message:
```
What's news of the day on the web?
```

### Step 4: Check Logs
After the response, close the app and check:
```bash
grep "🔧 \[API\] Passing" /tmp/rustbot_gui_test.log
grep "🔧 \[LLM\] Sending" /tmp/rustbot_gui_test.log
grep "📞 \[LLM\] Response contains" /tmp/rustbot_gui_test.log
```

## What to Look For

### Scenario A: Tools ARE Being Passed ✅
```
🔧 [API] Passing 1 tools to agent 'assistant': ["web_search"]
🔧 [LLM] Sending 1 tools to API:
   - Tool: web_search
🎯 [LLM] tool_choice: "auto"
📞 [LLM] Response contains NO tool calls - LLM responded directly
```

**Meaning**: Tools are passed correctly, but Claude is choosing not to use them.
**Fix**: Need to adjust query phrasing, tool_choice setting, or model selection.

### Scenario B: Tools NOT Being Passed ❌
```
🔧 [API] No tools passed to agent 'assistant'
```

**Meaning**: Tools aren't reaching the agent at all.
**Fix**: Bug in tool provisioning logic - need to investigate why primary agent isn't getting tools.

### Scenario C: Tools Called Successfully 🎉
```
🔧 [API] Passing 1 tools to agent 'assistant': ["web_search"]
🔧 [LLM] Sending 1 tools to API:
📞 [LLM] Response contains 1 tool call(s)
   - Tool call: web_search (id: call_xyz...)
Executing tool: web_search with args: {"query":"..."}
```

**Meaning**: Everything works!
**Action**: No fix needed, just query phrasing matters.

## Analysis Script

Run this to get a summary:
```bash
echo "=== TOOL PASSING ANALYSIS ==="
echo ""
echo "1. Tools Registered at Startup:"
grep "Tool registry updated" /tmp/rustbot_gui_test.log
echo ""
echo "2. Tools Passed to Agent:"
grep "🔧 \[API\] Passing" /tmp/rustbot_gui_test.log
echo ""
echo "3. Tools Sent to LLM:"
grep "🔧 \[LLM\] Sending" /tmp/rustbot_gui_test.log
echo ""
echo "4. LLM Response:"
grep "📞 \[LLM\] Response" /tmp/rustbot_gui_test.log
echo ""
echo "5. Tool Execution:"
grep "Executing tool:" /tmp/rustbot_gui_test.log
```

## Comparison: Test vs GUI

### Test Example (WORKED) ✅
- Model: `openai/gpt-4o`
- Query: "What's the weather in New York?"
- Result: Tool called successfully

### GUI App (FAILED) ❌
- Model: `anthropic/claude-sonnet-4.5`
- Query: "What's news of the day on the web?"
- Result: Direct response without tool call

### Hypothesis

**Possible Causes**:
1. **Query ambiguity**: "What's news" vs "What are today's top news stories"
2. **Model difference**: GPT-4o vs Claude Sonnet behavior
3. **Tool not passed**: Bug in GUI app (to be confirmed by this test)
4. **System prompt conflict**: Instructions might discourage tool use

## Next Steps Based on Results

### If Scenario A (Tools passed, not called)

**Option 1**: Test with explicit query
```
Search the web for today's top news stories
```

**Option 2**: Force tool use (edit `src/agent/mod.rs:425`)
```rust
request.tool_choice = Some("required".to_string());
```

**Option 3**: Try different model
Edit `agents/presets/assistant.json`:
```json
"model": "openai/gpt-4o"
```

### If Scenario B (Tools not passed)

**Debug**: Check why tools aren't reaching primary agent
- Verify `is_primary = true` in assistant.json
- Check tool registry logic in api.rs
- Verify agent configs are loaded correctly

### If Scenario C (Works!)

**Action**: Document that explicit queries work better
- Update instructions with query examples
- Add UI hints for better queries

## Save Logs

After testing, save the complete log for analysis:
```bash
cp /tmp/rustbot_gui_test.log docs/progress/2025-11-15-gui-tool-test.log
```

## Report Back

Share these specific lines:
1. The "🔧 [API] Passing" line
2. The "📞 [LLM] Response" line
3. The assistant's actual response text

This will tell us exactly where the issue is!
