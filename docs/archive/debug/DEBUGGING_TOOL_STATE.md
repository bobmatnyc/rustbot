# Tool State Debugging Guide

## Problem

The GUI app shows the assistant responding with "I don't have browsing capabilities" even though:
- Tool registry shows "Tool registry updated: 1 tools available" at startup
- Assistant config has `isPrimary: true`
- Code logic should pass `available_tools` to primary agents

## Debug Logging Added

We've added comprehensive debug logging to track the tool state throughout the message processing flow.

### Location: `src/api.rs`

#### 1. At Start of `send_message()` (lines 134-162)

**What it logs:**
- Total count of `available_tools`
- Total count of `agent_configs`
- Current `active_agent_id`
- Names of all available tools (if any)
- Details of each agent config (id, name, isPrimary, enabled)

**Example output:**
```
🔍 [DEBUG] send_message called - available_tools.len() = 1, agent_configs.len() = 2, active_agent_id = 'assistant'
🔍 [DEBUG] Available tools: ["web_search"]
🔍 [DEBUG] Agent config: id='assistant', name='Assistant', isPrimary=true, enabled=true
🔍 [DEBUG] Agent config: id='web_search', name='Web Search', isPrimary=false, enabled=true
```

#### 2. During Agent Config Lookup (lines 193-230)

**What it logs:**
- Which agent ID is being looked up
- Whether the lookup succeeded or failed
- If found: the config's isPrimary and enabled status
- Whether tools will be passed based on isPrimary flag
- How many tools are being cloned

**Example output (success):**
```
🔍 [DEBUG] Looking for agent config with id = 'assistant'
🔍 [DEBUG] Found agent config: id='assistant', isPrimary=true, enabled=true
🔍 [DEBUG] Agent is PRIMARY, cloning 1 tools
```

**Example output (failure):**
```
🔍 [DEBUG] Looking for agent config with id = 'assistant'
🔍 [DEBUG] CRITICAL: No agent config found for active_agent_id='assistant'!
🔍 [DEBUG] No agent config found, no tools will be passed
```

#### 3. During Tool Registry Update (lines 87-105)

**What it logs:**
- When `update_tools()` is called
- Which agent configs are enabled specialists
- How many tools were built
- Names of all tools after update

**Example output:**
```
🔍 [DEBUG] update_tools called
🔍 [DEBUG] build_tool_definitions called with 2 agent configs
🔍 [DEBUG] Enabled specialist: id='web_search', name='Web Search'
🔍 [DEBUG] build_tool_definitions returning 1 tools
🔍 [DEBUG] Tool registry updated: 1 tools available
🔍 [DEBUG] Tools after update: ["web_search"]
```

## Testing Instructions

### 1. Run the Application

```bash
cargo run --release
```

### 2. Look for Startup Logs

You should see:
```
🔍 [DEBUG] update_tools called
🔍 [DEBUG] build_tool_definitions called with X agent configs
🔍 [DEBUG] Tool registry updated: X tools available
🔍 [DEBUG] Tools after update: [...]
```

### 3. Send a Message

Send any message like "search for rust programming"

### 4. Check the Debug Output

Look for the debug logs in this sequence:

```
🔍 [DEBUG] send_message called - available_tools.len() = X, ...
🔍 [DEBUG] Available tools: [...]
🔍 [DEBUG] Agent config: id='assistant', ...
🔍 [DEBUG] Looking for agent config with id = 'assistant'
🔍 [DEBUG] Found agent config: ...
🔍 [DEBUG] Agent is PRIMARY, cloning X tools
🔧 [API] Passing X tools to agent 'assistant': [...]
```

## What to Look For

### Scenario 1: available_tools is Empty

If you see:
```
🔍 [DEBUG] WARNING: available_tools is EMPTY!
```

This means `update_tools()` didn't populate the list. Check:
- Is `web_search` agent config enabled?
- Is `web_search` agent config isPrimary=false?
- Is `update_tools()` being called at the right time?

### Scenario 2: Agent Config Not Found

If you see:
```
🔍 [DEBUG] CRITICAL: No agent config found for active_agent_id='assistant'!
```

This means the ID lookup is failing. Check:
- Does an agent config with `id='assistant'` exist?
- Are `agent_configs` being properly stored during initialization?

### Scenario 3: Agent Not Primary

If you see:
```
🔍 [DEBUG] Agent is NOT primary, no tools
```

But the agent should be primary, check:
- Is `assistant` config's `is_primary` field set to `true`?
- Was the config modified somewhere?

### Scenario 4: Tools Being Passed But Not Used

If you see:
```
🔧 [API] Passing 1 tools to agent 'assistant': ["web_search"]
```

But the agent still says it doesn't have tools, check:
- Is the `tools` parameter actually being passed to `process_message_nonblocking()`?
- Is the LLM adapter correctly formatting the tools in the API request?
- Check the OpenRouter request logs to see if tools are in the JSON payload

## Next Steps

Based on the debug output, we can determine:
1. **Is the problem in tool registry initialization?** (empty at startup)
2. **Is the problem in agent config lookup?** (ID mismatch)
3. **Is the problem in tool passing logic?** (isPrimary check failing)
4. **Is the problem downstream?** (tools passed but not used by LLM)

## Cleanup After Debugging

Once the issue is found and fixed, you can remove or reduce these debug logs:
- Change `tracing::info!` to `tracing::debug!` for less noise
- Remove the `🔍 [DEBUG]` prefix logs entirely
- Keep only the essential logging for production

The existing logs at DEBUG and INFO level (without 🔍) are useful for production monitoring.
