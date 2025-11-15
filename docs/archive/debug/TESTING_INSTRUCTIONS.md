# Quick Testing Instructions for Web Search Tool Calling

## What Was Done

Enhanced logging has been added to diagnose why the web_search tool isn't being called by the LLM.

## What You Need to Do

### Step 1: Build
```bash
cargo build
```

### Step 2: Run with Logging
```bash
RUST_LOG=info ./target/debug/rustbot > /tmp/rustbot_tool_debug.log 2>&1
```

### Step 3: Test with Query
In the rustbot GUI, ask one of these:
- "What are today's top news stories?"
- "What's the latest AI news?"
- "What's happening with SpaceX this week?"

### Step 4: Close App
Close the rustbot window when done.

### Step 5: Analyze Logs
```bash
./analyze_logs.sh
```

## What to Look For

The key output line will be:

**✅ SUCCESS** - If you see:
```
📞 [LLM] Response contains 1 tool call(s)
   - Tool call: web_search (id: call_xyz123)
```

**❌ BUG CONFIRMED** - If you see:
```
📞 [LLM] Response contains NO tool calls - LLM responded directly
```

## Next Steps

### If SUCCESS
Great! The tool calling works. We just need to:
1. Add UI indication for delegations
2. Test more extensively

### If BUG CONFIRMED
I'll implement one of three fixes:

**Option A: Force Tool Use** (simple)
- Change one line to force LLM to always use tools when available
- Downside: Might force tools for non-tool queries

**Option B: Enhanced Instructions** (gentle)
- Make assistant instructions even more explicit
- Downside: Relies on LLM following instructions

**Option C: Smart Detection** (sophisticated)
- Analyze query for trigger words, force tool use selectively
- Best of both worlds but more complex

## Full Documentation

See `docs/progress/2025-11-15-tool-calling-debug.md` for complete details.

## Questions?

Just share the output of `./analyze_logs.sh` and I'll know exactly what to do next!
