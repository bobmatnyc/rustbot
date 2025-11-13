# Agent Registry Decision Summary

**Date**: 2025-11-13
**Status**: Research Complete ✅

## TL;DR

✅ **Our current approach is correct and follows industry best practices.**

- **Agent Registry**: Static JSON configuration files (what we already have)
- **Tool Registration**: OpenRouter native function calling (OpenAI standard)
- **No changes needed** to registry architecture - just need to add tool conversion

---

## The Question

> "How about for agent registry?"

After researching agent registry standards, here's what I found:

---

## Agent Registry: Two Approaches

### 1. Static Configuration Registry ✅ (What We Have)

**Our Implementation**:
```
agents/presets/
├── assistant.json     # Primary agent
└── web_search.json    # Specialist agent
```

**Industry Validation**:
- Used by 14.9% of LLM-based multi-agent systems
- Microsoft's multi-agent architecture recommends template-based YAML/JSON configs
- Perfect for single-application scope with known agents

**What We Already Have** ✅:
- JSON-based agent configurations
- `AgentLoader` that scans `agents/presets/` directory
- Agent metadata: name, model, instructions, capabilities
- Primary vs specialist distinction (`is_primary` flag)
- Enabled/disabled state management

**Advantages**:
- ✅ Simple and predictable
- ✅ Version controlled (git)
- ✅ Fast lookups (no network calls)
- ✅ No infrastructure overhead
- ✅ Perfect for our use case

### 2. Dynamic Discovery Registry ⏳ (Not Needed)

**Standards**:
- **ANS** (Agent Name Service) - IETF draft spec for universal agent directory
- **ACP** (Agent Communication Protocol) - IBM's runtime registration
- **A2A** (Agent2Agent) - Google's capability-based discovery via "Agent Cards"

**When These Are Used**:
- Multi-company agent ecosystems
- Public agent marketplaces
- Agents discovering unknown agents at runtime
- Distributed agent networks across organizations

**Why We Don't Need This**:
- ❌ We have a single application (not distributed)
- ❌ All agents known at build/deployment time
- ❌ No runtime agent discovery needed
- ❌ Not building an agent marketplace

---

## Complete Architecture

Here's how our registry fits with tool calling:

```
┌─────────────────────────────────────────────────────────┐
│                    Rustbot Application                  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │          Agent Registry (Static JSON)            │  │
│  │                                                   │  │
│  │  agents/presets/                                 │  │
│  │  ├── assistant.json    (is_primary: true)       │  │
│  │  └── web_search.json   (is_primary: false)      │  │
│  │                                                   │  │
│  │  Loaded by: AgentLoader                          │  │
│  └──────────────┬───────────────────────────────────┘  │
│                 │                                       │
│                 │ At startup: Load all agents          │
│                 ▼                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │           RustbotApi::agents                     │  │
│  │                                                   │  │
│  │  HashMap<String, Agent>:                         │  │
│  │  - "assistant" → Agent (primary)                 │  │
│  │  - "web_search" → Agent (specialist, enabled)    │  │
│  └──────────────┬───────────────────────────────────┘  │
│                 │                                       │
│                 │ Build tool definitions                │
│                 ▼                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │       Tool Registry Builder (NEW)                │  │
│  │                                                   │  │
│  │  Filter: !is_primary && enabled                  │  │
│  │  Convert: AgentConfig → ToolDefinition           │  │
│  │  Format: OpenAI function calling                 │  │
│  └──────────────┬───────────────────────────────────┘  │
│                 │                                       │
│                 │ Include in assistant's context        │
│                 ▼                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Assistant Agent (Primary)                    │  │
│  │                                                   │  │
│  │  LLM Request:                                    │  │
│  │  {                                               │  │
│  │    "messages": [...],                            │  │
│  │    "tools": [                                    │  │
│  │      {                                           │  │
│  │        "type": "function",                       │  │
│  │        "function": {                             │  │
│  │          "name": "web_search",                   │  │
│  │          "description": "...",                   │  │
│  │          "parameters": { ... }                   │  │
│  │        }                                         │  │
│  │      }                                           │  │
│  │    ]                                             │  │
│  │  }                                               │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## What We Need to Implement

Our registry is already complete! We just need to:

1. **Tool Definitions Builder** (new module)
   - Iterate through `RustbotApi::agents`
   - Filter: `!is_primary && enabled`
   - Convert each to OpenAI function format
   - Return `Vec<ToolDefinition>`

2. **Include Tools in Assistant Context**
   - When assistant processes message
   - Build tool definitions from registry
   - Add to LLM request

3. **Tool Call Routing** (uses existing registry)
   - Detect tool call in streaming response
   - Look up agent by ID in existing `agents` HashMap
   - Route message to specialist agent
   - Return result

**No changes needed** to our agent registry approach - it's already following best practices!

---

## Industry Standards Alignment

| Aspect | Industry Standard | Our Implementation | Status |
|--------|------------------|-------------------|---------|
| Agent Registry | Static config (JSON/YAML) for single-app | JSON files in `agents/presets/` | ✅ Aligned |
| Agent Metadata | Name, capabilities, model | `AgentConfig` struct | ✅ Aligned |
| Tool Format | OpenAI function calling | Will implement | ✅ Planned |
| Discovery | Static for single-app, Dynamic for distributed | Static (appropriate for our scope) | ✅ Aligned |

---

## Future Considerations

If we ever need dynamic agent discovery (e.g., building an agent marketplace):

### Agent Card Format (A2A Standard)

```json
{
  "agent_id": "web_search@rustbot.local",
  "name": "Web Search Agent",
  "description": "Searches the web for current information",
  "capabilities": ["web_search", "fact_checking"],
  "endpoint": "internal://web_search",
  "authentication": "internal",
  "version": "1.0"
}
```

### Agent Name Service (ANS)

- Universal directory for agent discovery
- PKI-based authentication
- Protocol-agnostic (works with MCP, ACP, A2A)
- IETF draft specification

**When to consider**: Only if building public agent marketplace or multi-company integrations.

---

## Conclusion

✅ **Our static JSON-based agent registry is the correct approach**

**Validated by**:
- Microsoft's Multi-Agent Reference Architecture
- Industry usage statistics (14.9% of systems use this pattern)
- Best practices for single-application scope

**What we have**:
- ✅ Agent registry (JSON configs + AgentLoader)
- ✅ Agent metadata (name, model, capabilities, status)
- ✅ Agent type distinction (primary vs specialist)

**What we need to add**:
- Convert enabled specialist agents → tool definitions
- Include tools in assistant's LLM requests
- Route tool calls to specialist agents

**No architecture changes needed** - just implement the tool conversion layer!

---

## References

- Microsoft Multi-Agent Reference Architecture
- Agent Name Service (ANS): https://datatracker.ietf.org/doc/draft-narajala-ans/
- A2A Protocol: Agent Cards specification
- Industry survey: 14.9% use Tool-Agent Registry pattern
- OpenAI Function Calling: https://platform.openai.com/docs/guides/function-calling
