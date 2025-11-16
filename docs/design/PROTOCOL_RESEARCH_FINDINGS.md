# Protocol Research Findings: Agent and Tool Registration Standards

**Date**: 2025-11-13
**Purpose**: Research standard protocols for agent and tool registration before implementing custom solution

## Executive Summary

After researching current industry standards for agent communication and tool registration (2024-2025), I found that:

1. **Tool Registration**: OpenRouter follows **OpenAI Function Calling Standard** (widely adopted)
2. **Agent Registry**: Multiple patterns exist - static JSON config approach is valid for single-app scope
3. **Agent Discovery**: Dynamic discovery protocols (ANS, ACP, A2A) are for distributed/multi-company systems
4. **Implementation Strategy**:
   - Use OpenRouter's native function calling for tool registration
   - Use static JSON-based agent registry for our single-application scope
   - Consider dynamic discovery (ANS/A2A) only if building agent marketplace

---

## 0. Agent Registry Standards

### Industry Patterns for Agent Registry (2024)

**Usage**: Agent registry pattern used in 14.9% of LLM-based multi-agent systems

### Registry Approaches

#### 1. **Static Configuration Registry** (Our Current Approach ✅)

**Description**: Agents defined in configuration files (JSON/YAML), loaded at startup

**Advantages**:
- Simple and predictable
- Version controlled
- Fast lookups (no network calls)
- Perfect for single-application scope
- No infrastructure overhead

**Example** (what we already have):
```
agents/presets/
├── assistant.json  (primary agent)
└── web_search.json (specialist agent)
```

**When to Use**:
- ✅ Single application with known agents at deployment time
- ✅ Small to medium number of agents (< 100)
- ✅ Agents don't need to discover each other dynamically
- ✅ All agents deployed together

**Industry Validation**: Microsoft's multi-agent patterns recommend template-based agent definition using YAML/JSON for agents that vary only in metadata.

#### 2. **Dynamic Discovery Registry** (For Distributed Systems)

**Protocols**:
- **ANS (Agent Name Service)**: Universal directory for agent discovery, IETF draft spec
- **ACP (Agent Communication Protocol)**: IBM's dynamic agent discovery and registration
- **A2A (Agent2Agent)**: Google's agent cards for capability-based discovery

**Features**:
- Runtime agent registration
- Capability-based discovery
- Cross-organization agent collaboration
- Protocol adapters for interoperability

**When to Use**:
- ⏳ Multi-company agent ecosystems
- ⏳ Public agent marketplaces
- ⏳ Agents need to discover unknown agents at runtime
- ⏳ Distributed agent networks

**Our Verdict**: ❌ Overkill for our use case

### Agent Registry Architecture Components

Based on Microsoft's Multi-Agent Reference Architecture:

```
┌─────────────────────────────────────────────────┐
│            Agent Orchestrator                   │
│  (Primary assistant agent in our case)          │
└──────────────┬──────────────────────────────────┘
               │
               │ Queries registry for available agents
               ▼
┌─────────────────────────────────────────────────┐
│           Agent Registry                        │
│  (AgentLoader + JSON configs in our case)       │
│                                                  │
│  - Agent metadata (name, description, model)    │
│  - Agent capabilities (web_search, etc.)        │
│  - Agent status (enabled/disabled)              │
│  - Agent type (primary/specialist)              │
└──────────────┬──────────────────────────────────┘
               │
               │ Returns enabled specialist agents
               ▼
┌─────────────────────────────────────────────────┐
│         Tool Definitions Builder                │
│  (Converts enabled agents → function calls)     │
└─────────────────────────────────────────────────┘
```

### Agent Card Format (A2A Protocol)

If we ever need dynamic discovery, A2A defines "Agent Cards":

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

**Note**: We don't need this complexity now, but it's the standard format if we ever build a public agent registry.

### Our Implementation Decision

✅ **Use Static JSON Configuration Registry** (what we already have)

**Rationale**:
1. We have a **single application** with known agents at deployment
2. Agents are **defined at build time**, not discovered at runtime
3. **Simple file-based registry** is sufficient (agents/presets/*.json)
4. **No network calls** needed for agent lookup
5. **Version controlled** agent definitions
6. Can upgrade to dynamic discovery later if needed

**What We Already Have**:
- ✅ JSON-based agent configs in `agents/presets/`
- ✅ `AgentLoader` that scans and loads all agents
- ✅ Agent metadata: name, model, capabilities, enabled status
- ✅ Primary vs specialist agent distinction

**What We Need to Add**:
- Convert enabled specialist agents → tool definitions
- Provide tool list to assistant agent

---

## 1. Tool Registration Standards

### OpenAI Function Calling Standard (Industry Standard)

**Status**: De facto standard - nearly all LLM providers support it
**Adoption**: OpenRouter, Anthropic, OpenAI, Ollama, and most third-party providers

**Specification**:
```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "function_name",
        "description": "Clear description of what the function does",
        "parameters": {
          "type": "object",
          "properties": {
            "param_name": {
              "type": "string",
              "description": "Parameter description"
            }
          },
          "required": ["param_name"]
        }
      }
    }
  ]
}
```

**OpenRouter Implementation**:
- Fully compatible with OpenAI function calling format
- Supports `tool_choice` parameter: `"none"`, `"auto"`, `"required"`, or specific function
- Supports `parallel_tool_calls` for concurrent function execution
- Documentation: https://openrouter.ai/docs/features/tool-calling

**Key Features**:
- JSON-RPC style tool definitions
- Automatic tool call detection in streaming responses
- Tool result messages with `tool_call_id` for correlation
- Clear separation between tool call request and execution

**Recommendation**: ✅ **Use OpenRouter's OpenAI-compatible function calling**
- Already supported by our LLM provider
- Industry standard format
- No additional dependencies needed
- Well-documented and battle-tested

---

## 2. Agent Communication Protocols

### Overview of Current Landscape (2024-2025)

Four major protocols are competing for standardization:

| Protocol | Organization | Status | Focus | Use Case |
|----------|-------------|--------|-------|----------|
| **MCP** | Anthropic | Released Nov 2024 | Tool/Data access | LLM-to-external-tools |
| **ACP** | IBM Research | Merging with A2A | Multimodal messaging | Agent-to-agent REST |
| **A2A** | Google | Linux Foundation | Task outsourcing | Enterprise workflows |
| **ANP** | Various | Proposed | Decentralized discovery | Agent marketplaces |

### Model Context Protocol (MCP) - Recommended for Future

**Organization**: Anthropic
**Released**: November 2024
**Status**: Open standard, open source
**Rust SDK**: Available - `rmcp` v0.8.0

**Purpose**: Standardize how AI models connect to external data sources and tools

**Architecture**:
- Client-server model (similar to Language Server Protocol - LSP)
- JSON-RPC 2.0 transport
- Standard transports: stdio, HTTP with SSE

**Primitives**:
- **Tools**: Executable functions the LLM can call
- **Resources**: Data sources the LLM can access
- **Prompts**: Reusable prompt templates
- **Sampling**: LLM-driven workflows
- **Roots**: Context boundaries

**Tool Registration in MCP**:
```json
{
  "method": "tools/list",
  "result": {
    "tools": [
      {
        "name": "tool_name",
        "description": "Tool description",
        "inputSchema": {
          "type": "object",
          "properties": { ... }
        }
      }
    ]
  }
}
```

**Advantages**:
- Official Anthropic standard
- Designed for Claude and modern LLMs
- Progressive tool discovery
- Well-suited for external integrations

**Limitations for Our Use Case**:
- Overhead for internal agent delegation
- Requires MCP server infrastructure
- More complex than needed for single-app scope

**Recommendation**: 🔄 **Consider for future external integrations**
- Not needed for internal assistant-to-specialist delegation
- Useful when connecting to external MCP servers (databases, APIs, file systems)
- Can layer on top of native tool calling later

### Agent Communication Protocol (ACP)

**Organization**: IBM Research
**Status**: Merging with A2A under Linux Foundation

**Focus**: REST-native multimodal messaging
**Features**:
- Multi-part messages
- Asynchronous streaming
- Multimodal agent responses

**Recommendation**: ⏳ **Monitor for future adoption**
- Still evolving standard
- Better suited for distributed systems
- Not needed for single-application delegation

### Agent-to-Agent Protocol (A2A)

**Organization**: Google (2024)
**Status**: Linux Foundation stewardship, 100+ companies

**Focus**: Enterprise-scale collaborative workflows
**Features**:
- Capability-based Agent Cards
- Peer-to-peer task outsourcing
- Workflow orchestration

**Recommendation**: ⏳ **Monitor for enterprise use cases**
- Designed for multi-company agent collaboration
- Overkill for single-app internal delegation
- Future consideration if building agent marketplace

### Agent Network Protocol (ANP)

**Organization**: Various contributors
**Status**: Proposed standard

**Focus**: Decentralized agent discovery
**Features**:
- Decentralized identifiers (DIDs)
- JSON-LD graphs
- Open-network discovery

**Recommendation**: ❌ **Not applicable to our use case**
- Designed for public agent networks
- We have static agent registry (JSON configs)

---

## 3. Industry Best Practices

### Phased Adoption Roadmap (Recommended by Research)

1. **Phase 1**: MCP for tool access (external data sources)
2. **Phase 2**: ACP for multimodal messaging (if needed)
3. **Phase 3**: A2A for collaborative task execution (enterprise)
4. **Phase 4**: ANP for decentralized agent marketplaces (future)

### Tool Registration Library: ToolRegistry

**Research Finding**: Academic work on protocol-agnostic tool management

**Features**:
- 60-80% reduction in tool integration code
- 3.1x performance improvements through concurrent execution
- 100% compatibility with OpenAI function calling standards
- Unified interface for tool registration and execution

**Relevance**: Validates that OpenAI function calling format is the industry standard

---

## 4. Recommended Implementation Strategy

### For Our Rustbot Architecture

#### Immediate Implementation (v0.3.0)

**Use OpenRouter Native Function Calling**:

✅ **Rationale**:
- Already supported by our LLM provider (OpenRouter)
- Industry standard format (OpenAI compatible)
- No additional dependencies or infrastructure
- Simple for single-application internal delegation
- Well-documented with examples

✅ **Implementation**:
1. Convert enabled specialist agents → OpenAI function call format
2. Include tools array in assistant's LLM requests
3. Detect tool calls in streaming responses
4. Route tool calls to specialist agents
5. Return tool results as "tool" role messages
6. Assistant synthesizes final response

**Example Flow**:
```
User: "What's the weather in Tokyo?"
    ↓
Assistant receives request with tools: [web_search]
    ↓
Assistant decides to call web_search tool
    ↓
Tool call: { "name": "web_search", "arguments": { "query": "Tokyo weather" } }
    ↓
Rustbot routes to web_search specialist agent
    ↓
Web search agent returns results
    ↓
Rustbot sends tool result back to assistant
    ↓
Assistant: "The current weather in Tokyo is..."
```

#### Future Enhancements (v0.4.0+)

**Consider MCP for External Integrations**:

🔄 **When to Add MCP**:
- Connecting to external MCP servers (file systems, databases)
- Progressive tool discovery from external sources
- Standardized integration with MCP ecosystem

🔄 **Architecture**:
- Keep OpenRouter function calling for internal delegation
- Add MCP client to discover external tools
- Convert MCP tools → OpenAI function format
- Merge internal + external tools in assistant context

**Hybrid Approach**:
```
Internal Agents (JSON configs) → OpenAI Function Format
        ↓
External MCP Servers → MCP Tools → Convert to OpenAI Format
        ↓
Combined Tool Registry → Assistant Context
```

---

## 5. Implementation Details

### OpenRouter Tool Definition (OpenAI Format)

Based on our agent architecture:

```rust
// src/agent/tools.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,  // "function"
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: FunctionParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameters {
    #[serde(rename = "type")]
    pub param_type: String,  // "object"
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

impl ToolDefinition {
    /// Convert an enabled specialist agent to tool definition
    pub fn from_agent(agent: &AgentConfig) -> Self {
        // Only enabled specialist agents become tools
        assert!(!agent.is_primary && agent.enabled);

        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: agent.id.clone(),
                description: Self::extract_tool_description(agent),
                parameters: Self::build_parameters(agent),
            },
        }
    }

    fn extract_tool_description(agent: &AgentConfig) -> String {
        // Extract first paragraph from instructions as tool description
        // Or use agent.name as fallback
        agent.instructions
            .lines()
            .take_while(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    fn build_parameters(agent: &AgentConfig) -> FunctionParameters {
        // For now, all agents take a "query" or "message" parameter
        // Can be customized per agent type in future

        if agent.id == "web_search" {
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "query": {
                        "type": "string",
                        "description": "The search query to execute"
                    }
                }),
                required: vec!["query".to_string()],
            }
        } else {
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "message": {
                        "type": "string",
                        "description": "The message to send to the agent"
                    }
                }),
                required: vec!["message".to_string()],
            }
        }
    }
}
```

### LLM Request with Tools

```rust
// src/llm/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,  // "auto", "none", "required"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<bool>,
}
```

### Tool Call Detection

OpenRouter returns tool calls in streaming chunks:

```json
{
  "choices": [{
    "delta": {
      "tool_calls": [{
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "web_search",
          "arguments": "{\"query\": \"Tokyo weather\"}"
        }
      }]
    }
  }]
}
```

### Tool Result Format

```json
{
  "role": "tool",
  "content": "Tokyo weather: 72°F, sunny...",
  "tool_call_id": "call_abc123"
}
```

---

## 6. Comparison: Native vs MCP

| Aspect | OpenRouter Native | MCP |
|--------|------------------|-----|
| **Scope** | Internal delegation | External integrations |
| **Complexity** | Low - direct API calls | Medium - server infrastructure |
| **Dependencies** | None (already using OpenRouter) | MCP server + Rust SDK |
| **Setup** | Tool definitions only | Server setup, JSON-RPC transport |
| **Discovery** | Static (JSON configs) | Progressive discovery |
| **Standards** | OpenAI function calling | MCP specification |
| **Use Case** | Assistant → Specialists | LLM → External tools |
| **Recommendation** | ✅ Use now | 🔄 Consider later |

---

## 7. Decision Matrix

### When to Use OpenRouter Native Function Calling

✅ **Use for**:
- Internal agent-to-agent delegation (our current need)
- Static agent registry (known at startup)
- Single-application scope
- Simplicity and minimal dependencies

### When to Use MCP

🔄 **Consider for**:
- External tool integration (databases, file systems, APIs)
- Progressive tool discovery
- Connecting to MCP ecosystem (when available)
- Future extensibility

### When to Use ACP/A2A

⏳ **Future consideration for**:
- Multi-company agent collaboration
- Enterprise workflow orchestration
- Distributed agent systems

---

## 8. Conclusion

**Primary Recommendation**:
✅ **Implement OpenRouter native function calling** (OpenAI format) for assistant-to-specialist agent delegation.

**Rationale**:
1. Industry standard format (OpenAI compatible)
2. Already supported by OpenRouter (no additional dependencies)
3. Simple and well-documented
4. Perfect fit for internal delegation
5. Can layer MCP on top later for external integrations

**Implementation Path**:
1. **Phase 1** (Now): OpenRouter function calling for internal agents
2. **Phase 2** (Future): Add MCP client for external tool discovery
3. **Phase 3** (Future): Hybrid approach merging internal + external tools

**Standards Compliance**:
- ✅ Following OpenAI function calling standard
- ✅ Compatible with MCP tool format (similar structure)
- ✅ Aligned with industry best practices
- ✅ Can migrate to MCP later without breaking changes

---

## References

- OpenRouter Tool Calling: https://openrouter.ai/docs/features/tool-calling
- MCP Documentation: https://docs.anthropic.com/en/docs/agents-and-tools/mcp
- MCP GitHub: https://github.com/modelcontextprotocol
- Agent Protocol Survey: https://arxiv.org/html/2505.02279v1
- ToolRegistry: https://arxiv.org/html/2507.10593v1
- OpenAI Function Calling: https://platform.openai.com/docs/guides/function-calling
