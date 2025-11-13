# Agent Delegation Architecture Design

**Date**: 2025-11-13
**Status**: Design Phase
**Version**: 1.0

## Research Summary

### Protocols Investigated

1. **Model Context Protocol (MCP)** - Anthropic, November 2024
   - Focus: Connecting LLMs to data sources and tools
   - Use case: Progressive tool discovery, reducing token usage by 98.7%
   - Rust SDK: Official `rmcp` crate available
   - Best for: Client-server communication with external data sources

2. **Agent Communication Protocol (ACP)** - IBM Research
   - Status: Merging with A2A under Linux Foundation
   - Focus: Agent-to-agent REST-based communication
   - Use case: Multi-agent systems with asynchronous tasks
   - Best for: Distributed agent networks

3. **Agent2Agent Protocol (A2A)** - Google, 2024
   - Status: Linux Foundation stewardship, 100+ companies supporting
   - Focus: Secure, intelligent agent communication
   - Use case: Enterprise multi-agent systems

### Decision: Native Tool Calling Approach

**For Rustbot v0.2.0**, we will implement a **simpler, native tool-calling architecture** rather than adopting MCP/ACP/A2A because:

1. **Scope**: Single-application, not distributed agents
2. **OpenRouter Native Support**: OpenRouter already supports tool calling via function calling
3. **Simplicity**: Avoid protocol overhead for internal agent delegation
4. **Future-proofing**: Can layer MCP on top later for external integrations

## Architecture Design

### Agent Types

1. **Primary Agent (Assistant)**
   - Always active
   - Handles all user messages
   - Has access to tool calling
   - Can delegate to specialist agents via tools

2. **Specialist Agents (Tools)**
   - Enabled/disabled (not "active")
   - Called by assistant via tool/function calling
   - Examples: web_search, code_execution, image_generation
   - Return results to assistant

### Agent States

```rust
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub instructions: String,
    pub personality: Option<String>,
    pub model: String,
    pub enabled: bool,              // Can this agent be called?
    pub is_primary: bool,            // Is this the main assistant?
    pub web_search_enabled: bool,
}
```

**State Logic**:
- `is_primary = true`: This is the assistant (always handles user messages)
- `is_primary = false, enabled = true`: Callable specialist agent (tool)
- `is_primary = false, enabled = false`: Disabled, not available

### Delegation Flow

```
User Message
    ↓
Primary Assistant (always receives)
    ↓
Intent Detection
    ↓
    ├─ Can answer directly → Direct response
    │
    └─ Needs specialist → Tool call to specialist agent
           ↓
       Specialist Agent (if enabled)
           ↓
       Returns result
           ↓
       Assistant synthesizes response
           ↓
       User receives final answer
```

### Implementation Approach

#### Phase 1: Tool Definition (Current Sprint)

1. **Define agents as tools** in assistant's context
2. Assistant sees available agents as callable functions
3. When assistant calls a tool, route to specialist agent
4. Return agent response as tool result

#### Phase 2: OpenRouter Tool Calling

OpenRouter supports function calling via:

```json
{
  "model": "anthropic/claude-sonnet-4.5",
  "messages": [...],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "web_search",
        "description": "Search the web for current information",
        "parameters": {
          "type": "object",
          "properties": {
            "query": {
              "type": "string",
              "description": "The search query"
            }
          },
          "required": ["query"]
        }
      }
    }
  ]
}
```

When Claude decides to call a tool:

```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "web_search",
        "arguments": "{\"query\": \"current weather in Tokyo\"}"
      }
    }
  ]
}
```

#### Phase 3: Agent Tool Routing

```rust
// Pseudo-code
match tool_call.function.name {
    "web_search" => {
        let agent = agents.get("web_search").unwrap();
        let result = agent.process_message(tool_call.arguments).await;

        // Return as tool result
        Message {
            role: "tool",
            content: result,
            tool_call_id: tool_call.id,
        }
    }
    _ => { /* handle unknown tool */ }
}
```

### UI Changes

**Before** (incorrect):
```
[Assistant] ● Active    [Activate]
[Web Search]            [Activate]
```

**After** (correct):
```
[Assistant] ● Primary   [Always Active]
[Web Search]            [✓ Enabled] [Toggle]
```

**UI Elements**:
- Primary agent: Shows "● Primary" badge, no toggle
- Specialist agents: Show enabled/disabled state with toggle switch
- When enabled: Agent appears as available tool to assistant
- When disabled: Agent not available for delegation

## Benefits

1. **Simpler Implementation**: Use existing OpenRouter tool calling
2. **Natural Delegation**: Assistant naturally routes to specialists
3. **Extensible**: Easy to add new specialist agents
4. **Future MCP Integration**: Can expose agents as MCP servers later
5. **Standards-Aligned**: Follows tool calling patterns used industry-wide

## Migration Path

### Current State (v0.2.0)
- Multiple agents loaded from JSON
- Switch between agents manually

### Target State (v0.3.0)
- One primary assistant
- Specialist agents as tools
- Assistant delegates automatically

### Future (v0.4.0+)
- MCP server integration for external tools
- A2A support for multi-instance communication
- Agent discovery via MCP registry

## Implementation Tasks

1. ✅ Research protocols (ACP, MCP, A2A)
2. ✅ Design architecture
3. 🔲 Update AgentConfig to include `is_primary` flag
4. 🔲 Modify UI to show Primary vs. Enabled states
5. 🔲 Implement tool registration system
6. 🔲 Build tool call routing in RustbotApi
7. 🔲 Add tool response handling
8. 🔲 Test end-to-end delegation flow

## Open Questions

1. Should we support agent-to-agent direct communication? (No for v0.3.0)
2. Maximum delegation depth? (1 level for v0.3.0 - assistant → specialist)
3. Tool timeout handling? (Use tokio timeout, default 30s)
4. Cost tracking for delegated calls? (Yes, aggregate in token stats)

## References

- MCP Specification: https://github.com/modelcontextprotocol/rust-sdk
- A2A Project: https://www.linuxfoundation.org/press/agent2agent
- OpenRouter Tool Calling: https://openrouter.ai/docs#tool-calling
- Survey Paper: https://arxiv.org/html/2505.02279v1
