# MCP Integration Research

**Research Date:** 2025-11-15
**Author:** Research Agent
**Project:** Rustbot MCP Plugin Architecture

## Executive Summary

This research investigates the Model Context Protocol (MCP) and its implementation patterns in Claude Desktop and Goose AI to inform the design of a unified plugin architecture for Rustbot. MCP is an open protocol specification (latest: 2025-06-18) that enables seamless integration between LLM applications and external data sources/tools through standardized JSON-RPC 2.0 communication.

**Key Findings:**

1. **MCP Protocol:** Uses JSON-RPC 2.0 over multiple transports (stdio for local, Streamable HTTP for cloud), with capability negotiation and three core primitives (tools, resources, prompts).

2. **Claude Desktop:** Simple configuration-based approach (`claude_desktop_config.json`) with automatic subprocess management for local MCP servers via stdio transport. Focuses on ease of use with minimal developer overhead.

3. **Goose AI:** More sophisticated extension system using YAML configuration (`config.yaml`) with both bundled and custom extensions, supporting stdio-based MCP servers with timeout management and environment configuration.

4. **Rust Ecosystem:** Official `rmcp` SDK provides production-ready MCP implementation with tokio async support, procedural macros for tool definitions, and transport-agnostic design suitable for Rustbot integration.

**Recommendation:** Adopt a hybrid approach combining Claude Desktop's configuration simplicity with Goose's extension management sophistication, implemented using the official `rmcp` SDK with custom egui UI for plugin management.

---

## 1. MCP Protocol Specification Analysis

### 1.1 Protocol Overview

**Protocol Foundation:**
- **Message Format:** JSON-RPC 2.0 (UTF-8 encoded)
- **Architecture:** Stateful connections between Hosts (LLM apps), Clients (connectors), and Servers (capability providers)
- **Latest Version:** 2025-06-18 specification
- **Official Site:** https://modelcontextprotocol.io

**Key 2025 Updates:**
- OAuth 2.1 support with PKCE flow for authorization
- Resource indicators to prevent token leakage
- Fine-grained access control specifications
- Standardized audit logging patterns
- Enterprise security enhancements

### 1.2 Transport Mechanisms

#### **stdio Transport (Recommended for Local Servers)**

**Architecture:**
```
┌─────────────┐              ┌─────────────┐
│   Client    │              │   Server    │
│             │              │ (subprocess)│
│             │─────stdin───>│             │
│             │<────stdout───│             │
│             │   (stderr)   │             │
└─────────────┘              └─────────────┘
```

**Technical Details:**
- Client launches server as subprocess
- JSON-RPC messages delimited by newlines (`\n`)
- Server reads from stdin, writes to stdout
- stderr reserved for logging (captured by client)
- No non-MCP content allowed on stdin/stdout

**Advantages:**
- Simple implementation
- No network configuration required
- OS-level sandboxing
- Very low latency (no network stack)
- Direct process communication

**Use Cases:**
- Local tools and services
- Development environments
- File system operations
- Database access
- Git operations

#### **Streamable HTTP Transport (Recommended for Cloud Services)**

**Architecture:**
```
┌─────────────┐              ┌─────────────┐
│   Client    │              │   Server    │
│             │              │  (HTTP)     │
│             │──POST/GET───>│             │
│             │<─JSON/SSE────│             │
└─────────────┘              └─────────────┘
```

**Technical Details:**
- Single HTTP endpoint for both POST and GET
- POST requests: Client-to-server JSON-RPC messages
- GET requests: Optional SSE stream for server-to-client messages
- Accept headers: `application/json` and `text/event-stream`
- Session management via cryptographically secure session IDs

**Message Flow:**
- **Requests:** POST → JSON response or SSE stream
- **Notifications/Responses:** POST → 202 Accepted
- **Server-to-Client:** GET → SSE stream (optional)

**Security Requirements:**
- Validate `Origin` header (prevent DNS rebinding)
- Bind to localhost (127.0.0.1) for local deployments
- Implement proper authentication (Bearer tokens, OAuth 2.1)
- Use HTTPS for remote services

**Use Cases:**
- Cloud-based services
- Remote API integrations
- Multi-tenant deployments
- Services requiring authentication
- Rate-limited APIs

#### **Legacy HTTP+SSE Transport (Deprecated)**

**Status:** Deprecated as of protocol version 2024-11-05, replaced by Streamable HTTP.

**Migration Path:** Existing HTTP+SSE implementations should migrate to Streamable HTTP which incorporates SSE as an optional streaming mechanism within a unified endpoint design.

### 1.3 Connection Lifecycle

**Phase 1: Initialization**

```
Client                                Server
  │                                     │
  │────initialize (version, caps)─────>│
  │                                     │
  │<───initialize (version, caps)──────│
  │                                     │
  │────initialized (notification)─────>│
  │                                     │
  │         [Normal Operation]          │
```

**Initialize Request Structure:**
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "id": 1,
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {},
      "elicitation": {}
    },
    "clientInfo": {
      "name": "rustbot",
      "version": "0.2.2"
    }
  }
}
```

**Initialize Response Structure:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "logging": {},
      "prompts": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "tools": { "listChanged": true }
    },
    "serverInfo": {
      "name": "example-server",
      "version": "1.0.0"
    },
    "instructions": "Optional usage instructions"
  }
}
```

**Phase 2: Normal Operation**

After `initialized` notification sent, client and server exchange:
- Tool calls (tools/list, tools/call)
- Resource access (resources/list, resources/read)
- Prompt retrieval (prompts/list, prompts/get)
- Capability notifications (listChanged events)

**Phase 3: Shutdown**

**stdio transport:**
1. Close server's stdin
2. Wait for graceful exit (with timeout)
3. Send SIGTERM if needed
4. Send SIGKILL as last resort

**HTTP transport:**
1. Send DELETE request to session endpoint (if stateful)
2. Close HTTP connections
3. Server responds with 404 on subsequent requests

### 1.4 Core Primitives

#### **Tools: Executable Functions**

**Definition:**
```json
{
  "name": "search_files",
  "title": "Search Files",
  "description": "Search for files matching a pattern",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pattern": { "type": "string" },
      "path": { "type": "string" }
    },
    "required": ["pattern"]
  }
}
```

**Discovery:**
```json
// Request
{ "method": "tools/list", "params": { "cursor": "optional" } }

// Response
{
  "tools": [...],
  "nextCursor": "pagination-token"
}
```

**Execution:**
```json
// Request
{
  "method": "tools/call",
  "params": {
    "name": "search_files",
    "arguments": { "pattern": "*.rs", "path": "/src" }
  }
}

// Response
{
  "content": [
    { "type": "text", "text": "Found 42 files" }
  ],
  "isError": false
}
```

**Error Handling:**
- **Protocol errors:** JSON-RPC errors for unknown tools/invalid args
- **Execution errors:** `isError: true` in response with error details

**Use Cases:**
- API calls
- File operations
- Database queries
- External command execution
- Data transformations

#### **Resources: Contextual Data**

**Definition:**
```json
{
  "uri": "file:///project/README.md",
  "name": "Project README",
  "description": "Main project documentation",
  "mimeType": "text/markdown"
}
```

**Discovery:**
```json
{ "method": "resources/list" }
```

**Access:**
```json
// Request
{ "method": "resources/read", "params": { "uri": "file:///..." } }

// Response
{
  "contents": [
    { "uri": "file:///...", "mimeType": "text/markdown", "text": "..." }
  ]
}
```

**Subscriptions:**
```json
// Subscribe to resource changes
{ "method": "resources/subscribe", "params": { "uri": "file:///..." } }

// Notification when changed
{ "method": "notifications/resources/updated", "params": { "uri": "..." } }
```

**Use Cases:**
- File content access
- Database records
- API responses
- Configuration data
- Documentation

#### **Prompts: Reusable Templates**

**Definition:**
```json
{
  "name": "code_review",
  "description": "Review code for best practices",
  "arguments": [
    { "name": "language", "description": "Programming language", "required": true },
    { "name": "style_guide", "description": "Style guide to follow", "required": false }
  ]
}
```

**Discovery:**
```json
{ "method": "prompts/list" }
```

**Retrieval:**
```json
// Request
{
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": { "language": "rust" }
  }
}

// Response
{
  "description": "Review Rust code",
  "messages": [
    { "role": "user", "content": { "type": "text", "text": "Review this Rust code..." } }
  ]
}
```

**Use Cases:**
- Workflow templates
- Guided interactions
- Standardized prompts
- Multi-step processes

### 1.5 Capability Negotiation

**Client Capabilities:**

| Capability | Description | Sub-capabilities |
|------------|-------------|------------------|
| `roots` | Provide filesystem roots | `listChanged`: Emit notifications when roots change |
| `sampling` | Support LLM sampling requests | None |
| `elicitation` | Handle server elicitation requests | None |

**Server Capabilities:**

| Capability | Description | Sub-capabilities |
|------------|-------------|------------------|
| `logging` | Emit structured log messages | None |
| `prompts` | Provide prompt templates | `listChanged`: Notify when prompts change |
| `resources` | Expose readable resources | `subscribe`: Support subscriptions<br>`listChanged`: Notify when resources change |
| `tools` | Expose callable tools | `listChanged`: Notify when tools change |
| `completions` | Provide argument autocompletion | None |

**Negotiation Process:**
1. Client declares desired capabilities in `initialize` request
2. Server declares available capabilities in `initialize` response
3. Both parties only use negotiated capabilities during operation
4. Unsupported capability use results in JSON-RPC error

### 1.6 Security Principles

**MCP Security Model:**

1. **Explicit User Consent**
   - Users must approve data access
   - Clear permission boundaries
   - Transparent capability exposure

2. **Data Privacy**
   - Access controls on resources
   - Sandboxed execution
   - No ambient authority

3. **Tool Safety**
   - Approval workflows for dangerous operations
   - Input validation (JSON Schema)
   - Error isolation

4. **LLM Sampling Controls**
   - User oversight for sampling requests
   - Rate limiting
   - Token budgets

**Transport Security:**
- stdio: OS-level process isolation
- HTTP: HTTPS, Origin validation, authentication
- OAuth 2.1 with PKCE for cloud services

---

## 2. Claude Desktop MCP Integration Analysis

### 2.1 Configuration Format

**File Location:**
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

**Configuration Schema:**
```json
{
  "mcpServers": {
    "<server_id>": {
      "command": "<executable>",
      "args": ["<arg1>", "<arg2>", "..."],
      "env": {
        "<ENV_VAR>": "<value>"
      }
    }
  }
}
```

**Example: Filesystem Server**
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/Users/username/Desktop",
        "/Users/username/Downloads"
      ]
    }
  }
}
```

**Example: PostgreSQL Server with Environment**
```json
{
  "mcpServers": {
    "postgres": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-postgres",
        "postgresql://localhost/mydb"
      ],
      "env": {
        "PGPASSWORD": "${POSTGRES_PASSWORD}"
      }
    }
  }
}
```

### 2.2 Server Lifecycle Management

**Startup Process:**
1. Claude Desktop reads `claude_desktop_config.json` on launch
2. For each enabled server in `mcpServers`:
   - Spawn subprocess using `command` + `args`
   - Set environment variables from `env`
   - Establish stdio communication (stdin/stdout)
   - Send `initialize` request
   - Wait for `initialize` response
   - Send `initialized` notification
3. Server icon appears in UI (bottom-right of input box)

**Configuration Changes:**
- Require complete app restart (quit + relaunch)
- No hot-reload support
- Configuration loaded once at startup

**Error Handling:**
- Failed server startup: Icon shows error state
- Process crashes: No automatic restart
- Configuration errors: Silent failure (server not loaded)

### 2.3 UI Integration

**Server Status Indicator:**
- Icon in bottom-right corner of chat input
- Hover shows connected server count
- Click shows server list and capabilities

**Tool Availability:**
- Tools automatically available in conversation
- LLM can call tools without explicit user action
- Results displayed inline in chat

**User Experience:**
- Zero-configuration for common servers (npx auto-installs)
- Environment variable substitution (`${VAR_NAME}`)
- Simple JSON configuration
- No UI for adding/removing servers (manual file edit)

### 2.4 Discovery and Tool Registration

**Automatic Discovery:**
1. After initialization, Claude Desktop sends `tools/list`
2. Server responds with available tools
3. Tools registered in Claude's function calling system
4. LLM context includes tool descriptions

**Tool Execution:**
1. LLM generates tool call in response
2. Claude Desktop sends `tools/call` to appropriate server
3. Server executes and returns result
4. Result incorporated into conversation

**Capability Notifications:**
- Server sends `notifications/tools/list_changed`
- Claude Desktop refreshes tool list
- New tools immediately available

### 2.5 Key Design Decisions

**Strengths:**
- ✅ Simple configuration format (JSON)
- ✅ No code required for integration
- ✅ Subprocess isolation
- ✅ Environment variable support
- ✅ Works with any stdio-based MCP server
- ✅ Zero-config npx usage

**Limitations:**
- ❌ No hot-reload (requires restart)
- ❌ No UI for configuration
- ❌ No server enable/disable toggle
- ❌ Limited error reporting
- ❌ No process restart on crash
- ❌ No cloud service support (stdio only)

---

## 3. Goose AI MCP Architecture Analysis

### 3.1 Extension Configuration Format

**File Location:**
- **macOS/Linux:** `~/.config/goose/config.yaml`
- **Windows:** `%APPDATA%\goose\config.yaml`

**Configuration Schema:**
```yaml
extensions:
  <extension_id>:
    # Core settings
    name: "extension_id"
    display_name: "Human Readable Name"  # Optional
    description: "Extension description"
    enabled: true | false
    timeout: 300  # seconds

    # Extension type
    type: "builtin" | "stdio"
    bundled: true | false

    # stdio-specific settings
    cmd: "command"
    args: ["arg1", "arg2"]
    env_keys: ["REQUIRED_VAR"]  # List of required env vars
    envs:  # Environment variable values
      VAR_NAME: "value"
```

**Example: PostgreSQL Extension**
```yaml
extensions:
  postgres:
    name: "postgres"
    display_name: "PostgreSQL Database"
    description: "Query and manage PostgreSQL databases"
    type: "stdio"
    cmd: "uv"
    args: ["run", "/path/to/mcp-postgres/.venv/bin/mcp-postgres"]
    env_keys: ["DATABASE_URL"]
    envs:
      DATABASE_URL: "${DATABASE_URL}"
    timeout: 300
    enabled: true
    bundled: false
```

### 3.2 Extension Types

**Bundled Extensions:**
```yaml
developer:
  bundled: true
  enabled: true
  name: "developer"
  type: "builtin"
  timeout: 300
```
- Shipped with Goose
- Implemented in Go
- Compiled into binary
- No external process

**Custom Extensions:**
```yaml
custom_mcp:
  bundled: false
  enabled: true
  name: "custom_mcp"
  type: "stdio"
  cmd: "node"
  args: ["server.js"]
  timeout: 300
```
- User-provided MCP servers
- Run as subprocesses
- stdio communication
- Full MCP protocol support

### 3.3 Extension Lifecycle Management

**Startup:**
1. Goose reads `config.yaml` on launch
2. For each enabled extension:
   - If `bundled: true`: Use internal implementation
   - If `type: "stdio"`: Spawn subprocess
   - Validate `env_keys` (all required vars present)
   - Send `initialize` request
   - Wait for response with timeout
   - Send `initialized` notification

**Runtime Management:**
- **Timeout handling:** Kill process if operation exceeds `timeout`
- **Error recovery:** Restart extension on crash (automatic)
- **Hot reload:** Restart extension on config change (no full app restart)

**Shutdown:**
1. Send graceful shutdown signal
2. Wait for process exit (with timeout)
3. Force kill if necessary

### 3.4 Extension Discovery

**Extension Allowlist:**
- Goose maintains a community-curated list
- Extensions can be discovered via CLI
- Installation helpers for common extensions

**Tool Registration:**
1. After initialization: `tools/list` request
2. Tools registered in Goose's toolkit
3. LLM context includes tool metadata
4. Tools callable via standard protocol

### 3.5 UI and CLI Integration

**CLI Commands:**
```bash
goose extension list         # List available extensions
goose extension enable <id>  # Enable extension
goose extension disable <id> # Disable extension
goose extension config       # Edit config.yaml
```

**Desktop Application:**
- Extension manager UI
- Enable/disable toggles
- Configuration editor
- Status indicators
- Error logs

**Agent Runtime Integration:**
- Extensions expose tools to agent
- Agent can call tools via MCP protocol
- Results integrated into agent's context
- Tool usage logged and displayed

### 3.6 Key Design Decisions

**Strengths:**
- ✅ YAML configuration (more readable)
- ✅ Extension enable/disable without edit
- ✅ Timeout management
- ✅ Environment variable validation
- ✅ Bundled + custom extension support
- ✅ CLI management commands
- ✅ UI for extension management
- ✅ Automatic restart on crash
- ✅ Hot reload support

**Limitations:**
- ❌ Still stdio-only (no cloud services)
- ❌ More complex configuration
- ❌ Requires Goose-specific tooling

---

## 4. Comparative Analysis

### 4.1 Configuration Approach

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **Format** | JSON | YAML | JSON (easier to parse in Rust with serde) |
| **Location** | App Support dir | ~/.config | App Support dir or ~/.config/rustbot |
| **Schema** | Simple object | Typed fields | Combine: Simple + typed fields |
| **Validation** | None | env_keys check | JSON Schema validation |
| **Hot Reload** | No (requires restart) | Yes (extension-level) | Yes (plugin-level restart) |

**Recommendation:** Use JSON configuration with schema validation, supporting hot-reload at the plugin level (restart individual plugins without restarting Rustbot).

### 4.2 Server/Extension Management

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **Process Spawning** | Automatic (npx) | Explicit (cmd + args) | Explicit with npx shorthand support |
| **Lifecycle** | Spawn at startup | Spawn at startup + restart | Spawn on-demand + automatic restart |
| **Timeout** | None | Configurable per extension | Configurable per plugin (default: 60s) |
| **Crash Recovery** | None | Automatic restart | Automatic restart with backoff |
| **Process Cleanup** | SIGTERM/SIGKILL | Graceful with timeout | Graceful with configurable timeout |

**Recommendation:** Implement robust process management with:
- On-demand spawning (lazy loading)
- Automatic crash recovery with exponential backoff
- Configurable timeouts per plugin
- Graceful shutdown with SIGTERM → SIGKILL escalation

### 4.3 Transport Support

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **stdio** | ✅ Full support | ✅ Full support | ✅ Full support (primary) |
| **HTTP/SSE** | ❌ Not supported | ❌ Not supported | ✅ Add support for cloud services |
| **Streamable HTTP** | ❌ Not supported | ❌ Not supported | ✅ Add support (future-proof) |
| **Custom Transports** | ❌ Not supported | ❌ Not supported | 🔶 Extensible design, but not priority |

**Recommendation:** Support both stdio (local) and Streamable HTTP (cloud) transports through unified trait-based abstraction. Prioritize stdio implementation first, add HTTP support in phase 2.

### 4.4 UI/UX Patterns

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **Server List** | Icon + hover tooltip | Extension manager | Dedicated plugins pane |
| **Enable/Disable** | Manual JSON edit | Toggle in UI/CLI | Toggle in plugins pane |
| **Configuration** | Manual JSON edit | config.yaml + CLI | In-app editor with JSON view |
| **Status Display** | Icon indicator | Status text + errors | Icon + status text + logs |
| **Error Reporting** | Minimal | Error logs in UI | Detailed error panel with logs |
| **Tool Browsing** | Not available | Not available | ✅ Browse available tools per plugin |

**Recommendation:** Build comprehensive plugins pane in egui with:
- List of all configured plugins (scrollable)
- Per-plugin card: name, status, enable/disable toggle
- Configuration editor (JSON with syntax highlighting)
- Error log viewer per plugin
- Tool browser showing available tools

### 4.5 Error Handling and Recovery

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **Config Errors** | Silent failure | Validation errors | Detailed error messages with fix hints |
| **Startup Failures** | Icon error state | Error in UI | Error panel + retry button |
| **Runtime Errors** | Tool call fails | Extension restarts | Auto-retry with exponential backoff |
| **Crash Recovery** | Manual restart | Automatic restart | Automatic restart + error notification |
| **Reconnection** | Not supported | Not supported | ✅ Support reconnection for HTTP transport |

**Recommendation:** Implement comprehensive error handling:
- Configuration validation with helpful error messages
- Automatic retry with exponential backoff (max 3 attempts)
- Error notifications in UI with actionable buttons
- Detailed logs accessible in plugins pane

### 4.6 Security and Sandboxing

| Aspect | Claude Desktop | Goose | Recommendation for Rustbot |
|--------|----------------|-------|---------------------------|
| **Process Isolation** | OS-level (subprocess) | OS-level (subprocess) | OS-level (subprocess) |
| **Environment Vars** | Direct substitution | Validation + substitution | Validation + encrypted storage option |
| **Tool Approval** | Automatic execution | Automatic execution | 🔶 Optional approval mode (future) |
| **Resource Access** | Unrestricted (by server) | Unrestricted (by server) | ✅ Plugin permission system (future) |
| **Cloud Auth** | N/A | N/A | ✅ OAuth 2.1 support for HTTP transport |

**Recommendation:**
- Phase 1: OS-level isolation via subprocesses
- Phase 2: Plugin permission system (declare required resources)
- Phase 3: Optional tool approval mode (user confirms dangerous operations)

---

## 5. Rust Ecosystem Analysis

### 5.1 Official MCP SDK: `rmcp`

**Crate:** `rmcp` (https://docs.rs/rmcp)
**Repository:** https://github.com/modelcontextprotocol/rust-sdk
**License:** MIT (compatible with Rustbot)

**Key Features:**
- ✅ Full MCP protocol implementation
- ✅ Async/await support (tokio-based)
- ✅ Transport-agnostic design
- ✅ Procedural macros for tool definitions
- ✅ JSON-RPC 2.0 message handling
- ✅ JSON Schema generation (via `schemars`)

**Core Dependencies:**
```toml
[dependencies]
rmcp = { version = "0.x", features = ["server", "client", "transport-io", "macros"] }
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "0.8"  # JSON Schema generation
```

**Example Tool Definition with Macros:**
```rust
use rmcp::tool_router;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema)]
struct SearchArgs {
    pattern: String,
    path: Option<String>,
}

#[derive(Serialize)]
struct SearchResult {
    files: Vec<String>,
}

#[tool_router]
async fn search_files(args: SearchArgs) -> Result<SearchResult, String> {
    // Implementation
    Ok(SearchResult { files: vec![] })
}
```

**Verdict:** **HIGHLY RECOMMENDED** - Production-ready, well-maintained, official SDK with all needed features.

### 5.2 JSON-RPC Libraries

**Primary Recommendation: Built into `rmcp`**

The `rmcp` SDK handles JSON-RPC 2.0 internally, no separate library needed.

**Alternative (if custom implementation needed):**
- `jsonrpc-core` (https://docs.rs/jsonrpc-core) - Mature, 2M+ downloads
- `jsonrpsee` (https://docs.rs/jsonrpsee) - Modern, async-first

**Verdict:** Use `rmcp`'s built-in JSON-RPC support.

### 5.3 Process Management: `tokio::process`

**Crate:** Built into `tokio`
**Docs:** https://docs.rs/tokio/latest/tokio/process

**Key Features:**
- ✅ Async subprocess spawning
- ✅ Async stdin/stdout/stderr access
- ✅ Signal handling (SIGTERM, SIGKILL)
- ✅ Exit status monitoring

**Example Usage:**
```rust
use tokio::process::{Command, Child};
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};

async fn spawn_mcp_server(cmd: &str, args: &[String]) -> Result<Child, io::Error> {
    Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
}

async fn communicate(child: &mut Child) -> Result<(), io::Error> {
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    // Write to stdin
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",...}\n").await?;

    // Read from stdout
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await? {
        // Process JSON-RPC message
    }

    Ok(())
}
```

**Verdict:** **RECOMMENDED** - Perfect fit for stdio transport implementation.

### 5.4 HTTP/SSE Client: `reqwest` + `reqwest-eventsource`

**HTTP Client:**
- **Crate:** `reqwest` (already used in Rustbot)
- **Version:** 0.12
- **Features:** async, JSON, streaming

**SSE Support:**
- **Crate:** `reqwest-eventsource`
- **Docs:** https://docs.rs/reqwest-eventsource
- **Features:** Retry logic, reconnection, async streams

**Example Usage:**
```rust
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use futures::StreamExt;

async fn connect_mcp_http(url: &str, token: &str) -> Result<EventSource, reqwest::Error> {
    let client = Client::new();
    let mut es = EventSource::get(url)
        .header("Authorization", format!("Bearer {}", token))
        .build(client)?;

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Message(msg)) => {
                // Process JSON-RPC message
                println!("Message: {}", msg.data);
            }
            Ok(Event::Open) => println!("Connected"),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(es)
}

async fn send_mcp_request(url: &str, token: &str, msg: serde_json::Value) -> Result<serde_json::Value, reqwest::Error> {
    let client = Client::new();
    client.post(url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&msg)
        .send()
        .await?
        .json()
        .await
}
```

**Verdict:** **RECOMMENDED** - Proven libraries, already integrated into Rustbot.

### 5.5 Configuration Management

**Current Rustbot Approach:**
- `serde_json` for JSON deserialization
- Direct file I/O

**Recommendation:**
- Continue using `serde_json`
- Add `schemars` for JSON Schema validation
- Consider `config-rs` for multi-source configuration (files + env vars)

**Enhanced Configuration Example:**
```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, Serialize, JsonSchema)]
struct PluginConfig {
    mcp_plugins: McpPlugins,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct McpPlugins {
    local_servers: Vec<LocalServer>,
    cloud_services: Vec<CloudService>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct LocalServer {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct CloudService {
    id: String,
    name: String,
    url: String,
    auth: AuthConfig,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "type")]
enum AuthConfig {
    Bearer { token: String },
    OAuth { client_id: String, /* ... */ },
}
```

**Verdict:** `serde_json` + `schemars` for type-safe, validated configuration.

### 5.6 Async Runtime

**Current:** `tokio` 1.40 (already integrated)

**Recommendation:** Continue using tokio. All MCP operations are async:
- Subprocess management
- HTTP requests
- SSE streaming
- JSON-RPC message handling

**Consideration:** Ensure MCP plugin manager runs on tokio runtime, integrate with existing event bus for UI updates.

### 5.7 Summary: Recommended Crates

| Purpose | Crate | Version | License | Notes |
|---------|-------|---------|---------|-------|
| **MCP Protocol** | `rmcp` | 0.x | MIT | Official SDK, production-ready |
| **Async Runtime** | `tokio` | 1.40 | MIT | Already integrated |
| **HTTP Client** | `reqwest` | 0.12 | MIT/Apache-2.0 | Already integrated |
| **SSE Client** | `reqwest-eventsource` | 0.6 | MIT | Mature, 158k downloads/mo |
| **JSON** | `serde_json` | 1.0 | MIT/Apache-2.0 | Already integrated |
| **JSON Schema** | `schemars` | 0.8 | MIT | Schema generation |
| **Process** | `tokio::process` | (part of tokio) | MIT | Built-in subprocess support |

**No gaps identified** - All required functionality available in mature, well-maintained crates.

---

## 6. Example MCP Server Configurations

### 6.1 Official MCP Servers

**Filesystem Server:**
```json
{
  "id": "filesystem",
  "name": "Filesystem Access",
  "command": "npx",
  "args": [
    "-y",
    "@modelcontextprotocol/server-filesystem",
    "/Users/username/Projects",
    "/Users/username/Documents"
  ],
  "enabled": true
}
```

**SQLite Server:**
```json
{
  "id": "sqlite",
  "name": "SQLite Database",
  "command": "npx",
  "args": [
    "-y",
    "@modelcontextprotocol/server-sqlite",
    "/path/to/database.db"
  ],
  "enabled": true
}
```

**Git Server:**
```json
{
  "id": "git",
  "name": "Git Repository",
  "command": "npx",
  "args": [
    "-y",
    "@modelcontextprotocol/server-git",
    "/Users/username/Projects/rustbot"
  ],
  "enabled": true
}
```

**GitHub Server:**
```json
{
  "id": "github",
  "name": "GitHub Integration",
  "command": "npx",
  "args": [
    "-y",
    "@modelcontextprotocol/server-github"
  ],
  "env": {
    "GITHUB_TOKEN": "${GITHUB_TOKEN}"
  },
  "enabled": true
}
```

### 6.2 Cloud Service Example

**Hypothetical Weather Service:**
```json
{
  "id": "weather",
  "name": "Weather API",
  "url": "https://mcp.weather-api.example.com",
  "auth": {
    "type": "bearer",
    "token": "${WEATHER_API_KEY}"
  },
  "enabled": true
}
```

### 6.3 Multi-Server Configuration

**Complete Rustbot Plugin Configuration:**
```json
{
  "mcp_plugins": {
    "local_servers": [
      {
        "id": "filesystem",
        "name": "Filesystem",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/masa/Projects"],
        "enabled": true
      },
      {
        "id": "sqlite",
        "name": "SQLite",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-sqlite", "./rustbot.db"],
        "enabled": true
      },
      {
        "id": "git",
        "name": "Git",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-git", "."],
        "enabled": false
      }
    ],
    "cloud_services": [
      {
        "id": "example_cloud",
        "name": "Example Cloud Service",
        "url": "https://mcp.example.com",
        "auth": {
          "type": "bearer",
          "token": "${API_TOKEN}"
        },
        "enabled": false
      }
    ]
  }
}
```

---

## 7. Key Takeaways and Recommendations

### 7.1 Protocol Implementation

**✅ Use Official `rmcp` SDK**
- Production-ready, well-maintained
- Comprehensive protocol support
- Async-first design (tokio)
- Procedural macros for tool definitions

**✅ Implement stdio Transport First (Phase 1)**
- Simpler implementation
- Most common use case (local tools)
- Proven pattern from Claude Desktop/Goose
- Use `tokio::process` for subprocess management

**✅ Add HTTP Transport Later (Phase 2)**
- Future-proof for cloud services
- Use `reqwest` + `reqwest-eventsource`
- Implement OAuth 2.1 authentication
- Support Streamable HTTP spec

### 7.2 Architecture Design

**✅ Unified Plugin Manager**
- Single source of truth for plugin state
- Manages both local and cloud plugins
- Integrates with existing tool registry
- Emits events via Rustbot's event bus

**✅ Transport Abstraction**
- Trait-based design: `McpTransport`
- Implementations: `StdioTransport`, `HttpTransport`
- Allows future custom transports
- Transparent to rest of system

**✅ Configuration-Driven**
- JSON configuration file
- JSON Schema validation
- Environment variable substitution
- Hot-reload support (per-plugin restart)

### 7.3 UI/UX Design

**✅ Dedicated Plugins Pane**
- List all configured plugins
- Per-plugin cards with status/controls
- Enable/disable toggles
- Configuration editor
- Error log viewer
- Tool browser

**✅ egui Implementation**
- Scrollable plugin list
- Collapsible sections per plugin
- Syntax-highlighted JSON editor
- Status icons (Phosphor icons)
- Error notifications

### 7.4 Error Handling

**✅ Comprehensive Error Strategy**
- Configuration validation with helpful messages
- Automatic retry with exponential backoff
- Crash recovery with process restart
- Detailed error logs per plugin
- User-actionable error messages

### 7.5 Development Phases

**Phase 1: Foundation (Week 1)**
- MCP protocol types (JSON-RPC, capabilities)
- Transport trait abstraction
- Configuration schema and loading
- Basic error handling

**Phase 2: stdio Support (Week 2)**
- `StdioTransport` implementation
- Process lifecycle management (spawn, restart, shutdown)
- Tool registration integration
- Testing with official MCP servers

**Phase 3: Plugin Manager (Week 3)**
- Plugin manager core logic
- Hot-reload support
- Configuration persistence
- Event bus integration

**Phase 4: UI (Week 4)**
- Plugins pane in egui
- Plugin cards with controls
- Configuration editor
- Status and error displays

**Phase 5: HTTP Transport (Week 5+)**
- `HttpTransport` implementation
- OAuth 2.1 support
- SSE streaming
- Cloud service testing

---

## 8. References

### Official Documentation
- **MCP Specification:** https://modelcontextprotocol.io/specification/2025-06-18
- **MCP Servers Repository:** https://github.com/modelcontextprotocol/servers
- **rmcp Rust SDK:** https://github.com/modelcontextprotocol/rust-sdk

### Implementation Examples
- **Claude Desktop Config:** https://modelcontextprotocol.io/docs/develop/connect-local-servers
- **Goose Architecture:** https://block.github.io/goose/docs/goose-architecture/
- **Goose Extensions:** https://block.github.io/goose/docs/getting-started/using-extensions/

### Rust Ecosystem
- **rmcp Documentation:** https://docs.rs/rmcp
- **tokio::process:** https://docs.rs/tokio/latest/tokio/process
- **reqwest-eventsource:** https://docs.rs/reqwest-eventsource
- **schemars:** https://docs.rs/schemars

### Community Resources
- **Awesome MCP Servers:** https://github.com/punkpeye/awesome-mcp-servers
- **MCP Best Practices:** https://modelcontextprotocol.io/docs/best-practices

---

**End of Research Document**
