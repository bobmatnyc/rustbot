# MCP Plugin Architecture Design

**Project:** Rustbot v0.2.2+
**Design Date:** 2025-11-15
**Author:** Research & Design Team
**Status:** Proposed Architecture

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [Configuration Schema](#configuration-schema)
5. [Rust Implementation](#rust-implementation)
6. [Transport Layer Design](#transport-layer-design)
7. [Plugin Lifecycle Management](#plugin-lifecycle-management)
8. [UI Design: Plugins Pane](#ui-design-plugins-pane)
9. [Integration Points](#integration-points)
10. [Error Handling Strategy](#error-handling-strategy)
11. [Security Considerations](#security-considerations)
12. [Implementation Roadmap](#implementation-roadmap)
13. [Testing Strategy](#testing-strategy)
14. [Future Enhancements](#future-enhancements)

---

## 1. Executive Summary

This document proposes a comprehensive MCP (Model Context Protocol) plugin architecture for Rustbot that enables seamless integration with both local MCP servers (via stdio) and cloud MCP services (via HTTP/SSE). The design combines the best practices from Claude Desktop and Goose AI while leveraging Rust's type safety, async capabilities, and the official `rmcp` SDK.

### Key Design Principles

1. **Unified Interface:** Single abstraction layer for both local and cloud MCP services
2. **Type Safety:** Leverage Rust's type system for compile-time guarantees
3. **Async-First:** Built on tokio for efficient I/O and concurrency
4. **User-Friendly:** Intuitive UI for plugin management with minimal configuration
5. **Robust:** Comprehensive error handling, automatic recovery, and process management
6. **Extensible:** Easy to add new transport types or custom plugins

### Architecture Highlights

- **Plugin Manager:** Central coordinator for all MCP plugins
- **Transport Abstraction:** Trait-based design supporting stdio and HTTP transports
- **Configuration System:** JSON-based with schema validation and hot-reload
- **UI Integration:** Dedicated plugins pane in egui with full CRUD operations
- **Event Bus Integration:** Seamless integration with Rustbot's existing event system
- **Tool Registry:** Automatic registration of MCP tools into Rustbot's tool calling system

---

## 2. Architecture Overview

### 2.1 System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Rustbot Application                                │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │                        UI Layer (egui)                             │    │
│  │                                                                    │    │
│  │  ┌──────────────────────────────────────────────────────────┐     │    │
│  │  │              Plugins Pane (McpPluginsView)               │     │    │
│  │  │  ┌────────────┐ ┌────────────┐ ┌────────────┐           │     │    │
│  │  │  │  Plugin    │ │  Plugin    │ │  Plugin    │           │     │    │
│  │  │  │   Card     │ │   Card     │ │   Card     │   ...     │     │    │
│  │  │  │  [Status]  │ │  [Status]  │ │  [Status]  │           │     │    │
│  │  │  │  [Toggle]  │ │  [Toggle]  │ │  [Toggle]  │           │     │    │
│  │  │  │  [Config]  │ │  [Config]  │ │  [Config]  │           │     │    │
│  │  │  └────────────┘ └────────────┘ └────────────┘           │     │    │
│  │  │                                                          │     │    │
│  │  │  [+ Add Plugin]  [⚙️ Settings]  [📋 Logs]               │     │    │
│  │  └──────────────────────────────────────────────────────────┘     │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                                    │                                       │
│                                    │ UI Events                             │
│                                    ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │                    MCP Plugin Manager (Core)                       │    │
│  │                                                                    │    │
│  │  • Plugin Registry: HashMap<PluginId, PluginState>                │    │
│  │  • Configuration Management (load, save, validate)                │    │
│  │  • Lifecycle Coordination (start, stop, restart)                  │    │
│  │  • Tool Registration & Dispatch                                   │    │
│  │  • Event Bus Integration                                          │    │
│  │  • Error Recovery & Retry Logic                                   │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                          │                    │                            │
│                          │                    │                            │
│         ┌────────────────┘                    └────────────────┐           │
│         │                                                      │           │
│         ▼                                                      ▼           │
│  ┌──────────────────────┐                          ┌──────────────────────┐│
│  │  Local MCP Adapter   │                          │  Cloud MCP Adapter   ││
│  │  (StdioTransport)    │                          │  (HttpTransport)     ││
│  │                      │                          │                      ││
│  │  • Process Spawning  │                          │  • HTTP Client       ││
│  │  • stdin/stdout I/O  │                          │  • SSE Streaming     ││
│  │  • Process Cleanup   │                          │  • OAuth Auth        ││
│  │  • Crash Detection   │                          │  • Session Mgmt      ││
│  └──────────────────────┘                          └──────────────────────┘│
│         │                                                      │           │
│         │ JSON-RPC                                 JSON-RPC    │           │
│         │ over stdio                               over HTTP  │           │
│         ▼                                                      ▼           │
│  ┌──────────────────────┐                          ┌──────────────────────┐│
│  │  Local MCP Servers   │                          │  Cloud MCP Services  ││
│  │                      │                          │                      ││
│  │  • Filesystem        │                          │  • Weather API       ││
│  │  • SQLite            │                          │  • External Tools    ││
│  │  • Git               │                          │  • SaaS Integrations ││
│  │  • GitHub            │                          │  • Custom Services   ││
│  └──────────────────────┘                          └──────────────────────┘│
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │                     Rustbot Core Components                        │    │
│  │                                                                    │    │
│  │  • Tool Registry (integrate MCP tools)                            │    │
│  │  • Agent System (use tools in conversations)                      │    │
│  │  • Event Bus (plugin status updates)                              │    │
│  │  • LLM Adapter (send tool calls to OpenRouter)                    │    │
│  └────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Component Interaction Flow

**Plugin Initialization:**
```
1. User Action: Enable plugin in UI
2. UI → Plugin Manager: EnablePlugin(plugin_id)
3. Plugin Manager → Transport Adapter: spawn_server(config)
4. Transport Adapter → MCP Server: spawn process / HTTP connect
5. Transport Adapter ↔ MCP Server: MCP handshake (initialize, initialized)
6. Transport Adapter → Plugin Manager: ServerReady(capabilities, tools)
7. Plugin Manager → Tool Registry: register_tools(plugin_id, tools)
8. Plugin Manager → Event Bus: PluginStatusChanged(plugin_id, Running)
9. Event Bus → UI: Update plugin status icon
```

**Tool Execution:**
```
1. LLM Response: Tool call in assistant message
2. Agent → Tool Registry: execute_tool(tool_name, args)
3. Tool Registry → Plugin Manager: call_tool(plugin_id, tool_name, args)
4. Plugin Manager → Transport Adapter: send_tool_call(tool_name, args)
5. Transport Adapter → MCP Server: JSON-RPC tools/call request
6. MCP Server → Transport Adapter: JSON-RPC response (result/error)
7. Transport Adapter → Plugin Manager: ToolResult(content, is_error)
8. Plugin Manager → Tool Registry: return result
9. Tool Registry → Agent: ToolOutput(content)
10. Agent → LLM: Include result in next request
```

### 2.3 Data Flow

```
Configuration File (JSON)
         ↓
   Plugin Manager (loads config)
         ↓
   Plugin State (in-memory)
         ↓
   Transport Adapters (spawn/connect)
         ↓
   MCP Servers (running processes/connections)
         ↓
   Tool Capabilities (discovered via tools/list)
         ↓
   Tool Registry (registered for agent use)
         ↓
   Agent Runtime (tools available in conversations)
```

---

## 3. Core Components

### 3.1 Plugin Manager (`McpPluginManager`)

**Responsibilities:**
- Load and validate configuration from JSON file
- Maintain plugin state registry
- Coordinate plugin lifecycle (start, stop, restart)
- Route tool calls to appropriate plugins
- Register/unregister tools with tool registry
- Emit events to event bus
- Handle errors and recovery

**Key State:**
```rust
pub struct McpPluginManager {
    /// Map of plugin_id -> PluginState
    plugins: HashMap<PluginId, PluginState>,

    /// Configuration file path
    config_path: PathBuf,

    /// Loaded configuration
    config: PluginConfig,

    /// Tool registry reference
    tool_registry: Arc<Mutex<ToolRegistry>>,

    /// Event bus sender
    event_tx: Sender<AppEvent>,

    /// Async runtime handle
    runtime: tokio::runtime::Handle,
}
```

**Public API:**
```rust
impl McpPluginManager {
    /// Create new plugin manager
    pub fn new(
        config_path: PathBuf,
        tool_registry: Arc<Mutex<ToolRegistry>>,
        event_tx: Sender<AppEvent>,
    ) -> Result<Self>;

    /// Load configuration and initialize plugins
    pub async fn initialize(&mut self) -> Result<()>;

    /// Enable a plugin (start if not running)
    pub async fn enable_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Disable a plugin (stop if running)
    pub async fn disable_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Restart a plugin
    pub async fn restart_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Add a new plugin to configuration
    pub async fn add_plugin(&mut self, plugin: PluginDefinition) -> Result<()>;

    /// Remove a plugin
    pub async fn remove_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Get plugin status
    pub fn get_plugin_status(&self, plugin_id: &str) -> Option<&PluginStatus>;

    /// Get all plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo>;

    /// Reload configuration from disk
    pub async fn reload_config(&mut self) -> Result<()>;

    /// Save current configuration to disk
    pub async fn save_config(&self) -> Result<()>;

    /// Call a tool on a specific plugin
    pub async fn call_tool(
        &self,
        plugin_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult>;

    /// Get available tools from all plugins
    pub fn get_all_tools(&self) -> Vec<ToolInfo>;

    /// Shutdown all plugins gracefully
    pub async fn shutdown(&mut self) -> Result<()>;
}
```

### 3.2 Transport Adapter Trait (`McpTransport`)

**Purpose:** Abstract interface for different MCP communication mechanisms (stdio, HTTP, etc.)

**Trait Definition:**
```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Connect/spawn the MCP server
    async fn connect(&mut self, config: &TransportConfig) -> Result<()>;

    /// Send JSON-RPC request and await response
    async fn request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Send JSON-RPC notification (no response expected)
    async fn notify(&mut self, notification: JsonRpcNotification) -> Result<()>;

    /// Receive server-initiated messages (for HTTP/SSE)
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>>;

    /// Check if connection is healthy
    async fn is_connected(&self) -> bool;

    /// Disconnect/shutdown the transport
    async fn disconnect(&mut self) -> Result<()>;

    /// Get transport type identifier
    fn transport_type(&self) -> TransportType;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Stdio,
    Http,
}
```

### 3.3 Plugin State (`PluginState`)

**Purpose:** Track runtime state of each plugin

```rust
pub struct PluginState {
    /// Unique plugin identifier
    pub id: PluginId,

    /// Plugin configuration
    pub config: PluginDefinition,

    /// Current status
    pub status: PluginStatus,

    /// Transport adapter instance
    pub transport: Option<Box<dyn McpTransport>>,

    /// Server capabilities (from initialize response)
    pub capabilities: Option<ServerCapabilities>,

    /// Available tools
    pub tools: Vec<McpTool>,

    /// Error information (if failed)
    pub error: Option<PluginError>,

    /// Restart count (for backoff)
    pub restart_count: u32,

    /// Last restart timestamp
    pub last_restart: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// Plugin is configured but not started
    Stopped,

    /// Plugin is starting (spawning process, connecting)
    Starting,

    /// Plugin is performing MCP handshake
    Initializing,

    /// Plugin is ready and operational
    Running,

    /// Plugin encountered an error
    Error,

    /// Plugin is shutting down
    Stopping,
}
```

### 3.4 Configuration Types

**Top-Level Configuration:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginConfig {
    pub mcp_plugins: McpPlugins,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpPlugins {
    #[serde(default)]
    pub local_servers: Vec<LocalServerConfig>,

    #[serde(default)]
    pub cloud_services: Vec<CloudServiceConfig>,
}
```

**Local Server Configuration:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalServerConfig {
    /// Unique identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Executable command
    pub command: String,

    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Whether plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 60 }
```

**Cloud Service Configuration:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CloudServiceConfig {
    /// Unique identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Service URL
    pub url: String,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Whether plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    /// No authentication
    None,

    /// Bearer token authentication
    Bearer {
        token: String,
    },

    /// OAuth 2.1 authentication
    OAuth {
        client_id: String,
        client_secret: Option<String>,
        token_url: String,
        scopes: Vec<String>,
    },
}
```

### 3.5 MCP Protocol Types

**JSON-RPC Messages:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String, // "2.0"
    pub id: RequestId,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String, // "2.0"
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String, // "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}
```

**MCP-Specific Types:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String, // "2025-06-18"
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value, // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, #[serde(rename = "mimeType")] mime_type: String },
    Resource { resource: ResourceReference },
}
```

---

## 4. Configuration Schema

### 4.1 Complete Configuration Example

**File:** `~/.rustbot/mcp_plugins.json`

```json
{
  "mcp_plugins": {
    "local_servers": [
      {
        "id": "filesystem",
        "name": "Filesystem Access",
        "description": "Access and manipulate files in allowed directories",
        "command": "npx",
        "args": [
          "-y",
          "@modelcontextprotocol/server-filesystem",
          "/Users/masa/Projects",
          "/Users/masa/Documents"
        ],
        "env": {},
        "enabled": true,
        "timeout": 60
      },
      {
        "id": "sqlite",
        "name": "SQLite Database",
        "description": "Query and manage SQLite databases",
        "command": "npx",
        "args": [
          "-y",
          "@modelcontextprotocol/server-sqlite",
          "/Users/masa/Projects/rustbot/rustbot.db"
        ],
        "env": {},
        "enabled": true,
        "timeout": 60
      },
      {
        "id": "git",
        "name": "Git Repository",
        "description": "Read and search Git repositories",
        "command": "npx",
        "args": [
          "-y",
          "@modelcontextprotocol/server-git",
          "/Users/masa/Projects/rustbot"
        ],
        "env": {},
        "enabled": false,
        "timeout": 60
      },
      {
        "id": "github",
        "name": "GitHub Integration",
        "description": "Manage GitHub repositories, issues, and PRs",
        "command": "npx",
        "args": [
          "-y",
          "@modelcontextprotocol/server-github"
        ],
        "env": {
          "GITHUB_TOKEN": "${GITHUB_TOKEN}"
        },
        "enabled": false,
        "timeout": 60
      }
    ],
    "cloud_services": [
      {
        "id": "weather_api",
        "name": "Weather API",
        "description": "Get current weather and forecasts",
        "url": "https://mcp.weather-service.example.com",
        "auth": {
          "type": "bearer",
          "token": "${WEATHER_API_KEY}"
        },
        "enabled": false,
        "timeout": 30
      },
      {
        "id": "custom_service",
        "name": "Custom MCP Service",
        "description": "Example cloud service with OAuth",
        "url": "https://mcp.example.com",
        "auth": {
          "type": "oauth",
          "client_id": "rustbot-client",
          "client_secret": "${OAUTH_CLIENT_SECRET}",
          "token_url": "https://auth.example.com/oauth/token",
          "scopes": ["mcp:read", "mcp:write"]
        },
        "enabled": false,
        "timeout": 30
      }
    ]
  }
}
```

### 4.2 Environment Variable Substitution

**Pattern:** `${VAR_NAME}`

**Resolution:**
1. Check environment variables
2. If not found, return error during validation
3. Substitute at runtime (not persisted in config)

**Example:**
```json
{
  "env": {
    "GITHUB_TOKEN": "${GITHUB_TOKEN}",
    "API_KEY": "${MY_API_KEY}"
  }
}
```

If `GITHUB_TOKEN` env var = `ghp_abc123`, then at runtime:
```json
{
  "env": {
    "GITHUB_TOKEN": "ghp_abc123",
    "API_KEY": "<resolved_value>"
  }
}
```

### 4.3 Configuration Validation

**Validation Rules:**
1. All `id` fields must be unique across all plugins
2. Local servers: `command` must be valid executable
3. Cloud services: `url` must be valid HTTPS URL
4. Environment variable references must resolve
5. `timeout` must be > 0
6. JSON Schema validation for entire config

**Validation Errors:**
```rust
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Duplicate plugin ID: {0}")]
    DuplicateId(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Unresolved environment variable: {0}")]
    UnresolvedEnvVar(String),

    #[error("JSON schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

---

## 5. Rust Implementation

### 5.1 Module Structure

```
src/
├── mcp/
│   ├── mod.rs                  # Public API exports
│   ├── manager.rs              # McpPluginManager
│   ├── plugin.rs               # PluginState, PluginStatus
│   ├── config.rs               # Configuration types
│   ├── transport/
│   │   ├── mod.rs              # Transport trait
│   │   ├── stdio.rs            # StdioTransport implementation
│   │   └── http.rs             # HttpTransport implementation (Phase 2)
│   ├── protocol.rs             # MCP protocol types (JSON-RPC, etc.)
│   ├── error.rs                # Error types
│   └── utils.rs                # Utility functions
└── ui/
    └── mcp_plugins_view.rs     # Plugins pane UI
```

### 5.2 Plugin Manager Implementation

**`src/mcp/manager.rs`:**

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde_json;

use crate::events::{AppEvent, EventSender};
use crate::tools::ToolRegistry;
use super::{
    config::*,
    plugin::*,
    transport::*,
    protocol::*,
    error::*,
};

pub struct McpPluginManager {
    /// Plugin state registry
    plugins: Arc<RwLock<HashMap<PluginId, PluginState>>>,

    /// Configuration file path
    config_path: PathBuf,

    /// Loaded configuration
    config: Arc<RwLock<PluginConfig>>,

    /// Tool registry reference
    tool_registry: Arc<Mutex<ToolRegistry>>,

    /// Event bus sender
    event_tx: EventSender,

    /// Async runtime handle
    runtime: tokio::runtime::Handle,
}

impl McpPluginManager {
    pub fn new(
        config_path: PathBuf,
        tool_registry: Arc<Mutex<ToolRegistry>>,
        event_tx: EventSender,
    ) -> Result<Self> {
        Ok(Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config_path,
            config: Arc::new(RwLock::new(PluginConfig::default())),
            tool_registry,
            event_tx,
            runtime: tokio::runtime::Handle::current(),
        })
    }

    /// Load configuration and initialize enabled plugins
    pub async fn initialize(&mut self) -> Result<()> {
        // Load configuration from disk
        let config = self.load_config().await?;
        *self.config.write().await = config.clone();

        // Validate configuration
        self.validate_config(&config).await?;

        // Initialize enabled local servers
        for server in &config.mcp_plugins.local_servers {
            if server.enabled {
                self.start_local_server(server.clone()).await?;
            }
        }

        // Initialize enabled cloud services
        for service in &config.mcp_plugins.cloud_services {
            if service.enabled {
                self.start_cloud_service(service.clone()).await?;
            }
        }

        Ok(())
    }

    /// Load configuration from disk
    async fn load_config(&self) -> Result<PluginConfig> {
        let content = tokio::fs::read_to_string(&self.config_path).await?;
        let config: PluginConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Validate configuration
    async fn validate_config(&self, config: &PluginConfig) -> Result<()> {
        // Check for duplicate IDs
        let mut ids = std::collections::HashSet::new();

        for server in &config.mcp_plugins.local_servers {
            if !ids.insert(&server.id) {
                return Err(McpError::Config(ConfigError::DuplicateId(server.id.clone())));
            }
        }

        for service in &config.mcp_plugins.cloud_services {
            if !ids.insert(&service.id) {
                return Err(McpError::Config(ConfigError::DuplicateId(service.id.clone())));
            }
        }

        // Validate environment variables
        for server in &config.mcp_plugins.local_servers {
            for (key, value) in &server.env {
                self.resolve_env_var(value)?;
            }
        }

        Ok(())
    }

    /// Resolve environment variable reference
    fn resolve_env_var(&self, value: &str) -> Result<String> {
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len()-1];
            std::env::var(var_name)
                .map_err(|_| McpError::Config(ConfigError::UnresolvedEnvVar(var_name.to_string())))
        } else {
            Ok(value.to_string())
        }
    }

    /// Start a local MCP server
    async fn start_local_server(&mut self, config: LocalServerConfig) -> Result<()> {
        let plugin_id = config.id.clone();

        // Create plugin state
        let mut state = PluginState::new_local(config.clone());
        state.status = PluginStatus::Starting;

        // Insert into registry
        self.plugins.write().await.insert(plugin_id.clone(), state);
        self.emit_status_event(&plugin_id, PluginStatus::Starting).await;

        // Spawn initialization task
        let plugins = self.plugins.clone();
        let tool_registry = self.tool_registry.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            match Self::initialize_local_plugin(config, tool_registry).await {
                Ok((transport, capabilities, tools)) => {
                    // Update plugin state
                    let mut guard = plugins.write().await;
                    if let Some(state) = guard.get_mut(&plugin_id) {
                        state.transport = Some(transport);
                        state.capabilities = Some(capabilities);
                        state.tools = tools;
                        state.status = PluginStatus::Running;
                        state.error = None;
                    }

                    event_tx.send(AppEvent::McpPluginStatusChanged {
                        plugin_id: plugin_id.clone(),
                        status: PluginStatus::Running,
                    });
                }
                Err(e) => {
                    // Update plugin state with error
                    let mut guard = plugins.write().await;
                    if let Some(state) = guard.get_mut(&plugin_id) {
                        state.status = PluginStatus::Error;
                        state.error = Some(PluginError::from(e));
                    }

                    event_tx.send(AppEvent::McpPluginStatusChanged {
                        plugin_id: plugin_id.clone(),
                        status: PluginStatus::Error,
                    });
                }
            }
        });

        Ok(())
    }

    /// Initialize a local plugin (async)
    async fn initialize_local_plugin(
        config: LocalServerConfig,
        tool_registry: Arc<Mutex<ToolRegistry>>,
    ) -> Result<(Box<dyn McpTransport>, ServerCapabilities, Vec<McpTool>)> {
        // Create stdio transport
        let mut transport = StdioTransport::new();

        // Connect (spawn process)
        let transport_config = TransportConfig::Stdio {
            command: config.command.clone(),
            args: config.args.clone(),
            env: Self::resolve_env_vars(&config.env)?,
        };
        transport.connect(&transport_config).await?;

        // Perform MCP handshake
        let (capabilities, tools) = Self::perform_handshake(&mut transport).await?;

        // Register tools with tool registry
        {
            let mut registry = tool_registry.lock().await;
            for tool in &tools {
                registry.register_mcp_tool(config.id.clone(), tool.clone())?;
            }
        }

        Ok((Box::new(transport), capabilities, tools))
    }

    /// Perform MCP handshake
    async fn perform_handshake(
        transport: &mut dyn McpTransport,
    ) -> Result<(ServerCapabilities, Vec<McpTool>)> {
        // Send initialize request
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(InitializeRequest {
                protocol_version: "2025-06-18".to_string(),
                capabilities: ClientCapabilities::default(),
                client_info: ClientInfo {
                    name: "rustbot".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            })?),
        };

        let response = transport.request(init_request).await?;
        let init_response: InitializeResponse = serde_json::from_value(
            response.result.ok_or(McpError::Protocol("No result in initialize response".to_string()))?
        )?;

        // Send initialized notification
        let initialized_notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
            params: None,
        };
        transport.notify(initialized_notif).await?;

        // Request tool list
        let tools_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(2),
            method: "tools/list".to_string(),
            params: None,
        };

        let tools_response = transport.request(tools_request).await?;
        let tools_list: ToolsListResponse = serde_json::from_value(
            tools_response.result.ok_or(McpError::Protocol("No result in tools/list response".to_string()))?
        )?;

        Ok((init_response.capabilities, tools_list.tools))
    }

    /// Resolve all environment variables in a map
    fn resolve_env_vars(env: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        let mut resolved = HashMap::new();
        for (key, value) in env {
            let resolved_value = if value.starts_with("${") && value.ends_with("}") {
                let var_name = &value[2..value.len()-1];
                std::env::var(var_name)
                    .map_err(|_| McpError::Config(ConfigError::UnresolvedEnvVar(var_name.to_string())))?
            } else {
                value.clone()
            };
            resolved.insert(key.clone(), resolved_value);
        }
        Ok(resolved)
    }

    /// Enable a plugin
    pub async fn enable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        // Update config
        let mut config = self.config.write().await;

        if let Some(server) = config.mcp_plugins.local_servers.iter_mut()
            .find(|s| s.id == plugin_id) {
            server.enabled = true;
            drop(config);

            // Save config
            self.save_config().await?;

            // Start plugin
            self.start_local_server(server.clone()).await?;

            Ok(())
        } else if let Some(service) = config.mcp_plugins.cloud_services.iter_mut()
            .find(|s| s.id == plugin_id) {
            service.enabled = true;
            drop(config);

            // Save config
            self.save_config().await?;

            // Start plugin
            self.start_cloud_service(service.clone()).await?;

            Ok(())
        } else {
            Err(McpError::PluginNotFound(plugin_id.to_string()))
        }
    }

    /// Disable a plugin
    pub async fn disable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        // Update config
        let mut config = self.config.write().await;

        if let Some(server) = config.mcp_plugins.local_servers.iter_mut()
            .find(|s| s.id == plugin_id) {
            server.enabled = false;
        } else if let Some(service) = config.mcp_plugins.cloud_services.iter_mut()
            .find(|s| s.id == plugin_id) {
            service.enabled = false;
        } else {
            return Err(McpError::PluginNotFound(plugin_id.to_string()));
        }

        drop(config);

        // Save config
        self.save_config().await?;

        // Stop plugin
        self.stop_plugin(plugin_id).await?;

        Ok(())
    }

    /// Stop a plugin
    async fn stop_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(state) = plugins.get_mut(plugin_id) {
            state.status = PluginStatus::Stopping;

            if let Some(transport) = &mut state.transport {
                transport.disconnect().await?;
            }

            state.transport = None;
            state.status = PluginStatus::Stopped;

            // Unregister tools
            let mut registry = self.tool_registry.lock().await;
            registry.unregister_plugin_tools(plugin_id)?;
        }

        self.emit_status_event(plugin_id, PluginStatus::Stopped).await;

        Ok(())
    }

    /// Save configuration to disk
    async fn save_config(&self) -> Result<()> {
        let config = self.config.read().await;
        let json = serde_json::to_string_pretty(&*config)?;
        tokio::fs::write(&self.config_path, json).await?;
        Ok(())
    }

    /// Emit status event to event bus
    async fn emit_status_event(&self, plugin_id: &str, status: PluginStatus) {
        self.event_tx.send(AppEvent::McpPluginStatusChanged {
            plugin_id: plugin_id.to_string(),
            status,
        });
    }

    /// Call a tool on a specific plugin
    pub async fn call_tool(
        &self,
        plugin_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult> {
        let plugins = self.plugins.read().await;

        let state = plugins.get(plugin_id)
            .ok_or_else(|| McpError::PluginNotFound(plugin_id.to_string()))?;

        let transport = state.transport.as_ref()
            .ok_or_else(|| McpError::PluginNotRunning(plugin_id.to_string()))?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(rand::random()),
            method: "tools/call".to_string(),
            params: Some(serde_json::to_value(ToolCallRequest {
                name: tool_name.to_string(),
                arguments,
            })?),
        };

        let response = transport.request(request).await?;
        let result: ToolCallResult = serde_json::from_value(
            response.result.ok_or(McpError::Protocol("No result in tool call response".to_string()))?
        )?;

        Ok(result)
    }

    /// Get all available tools from all running plugins
    pub async fn get_all_tools(&self) -> Vec<(PluginId, McpTool)> {
        let plugins = self.plugins.read().await;
        let mut tools = Vec::new();

        for (id, state) in plugins.iter() {
            if state.status == PluginStatus::Running {
                for tool in &state.tools {
                    tools.push((id.clone(), tool.clone()));
                }
            }
        }

        tools
    }

    /// List all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.iter().map(|(id, state)| PluginInfo {
            id: id.clone(),
            name: state.config.name().clone(),
            status: state.status.clone(),
            tool_count: state.tools.len(),
            error: state.error.as_ref().map(|e| e.to_string()),
        }).collect()
    }

    /// Shutdown all plugins
    pub async fn shutdown(&mut self) -> Result<()> {
        let plugin_ids: Vec<_> = self.plugins.read().await.keys().cloned().collect();

        for id in plugin_ids {
            let _ = self.stop_plugin(&id).await; // Ignore errors during shutdown
        }

        Ok(())
    }
}
```

### 5.3 stdio Transport Implementation

**`src/mcp/transport/stdio.rs`:**

```rust
use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::process::Stdio;

use super::{McpTransport, TransportConfig, TransportType};
use crate::mcp::{protocol::*, error::*};

pub struct StdioTransport {
    /// Child process
    process: Option<Child>,

    /// Process stdin
    stdin: Option<tokio::process::ChildStdin>,

    /// Buffered stdout reader
    stdout: Option<BufReader<tokio::process::ChildStdout>>,

    /// Process stderr (for logging)
    stderr: Option<BufReader<tokio::process::ChildStderr>>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            process: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    /// Read a single JSON-RPC message from stdout
    async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        let stdout = self.stdout.as_mut()
            .ok_or(McpError::Transport("Not connected".to_string()))?;

        let mut line = String::new();
        stdout.read_line(&mut line).await?;

        if line.is_empty() {
            return Err(McpError::Transport("EOF from server".to_string()));
        }

        let message: JsonRpcMessage = serde_json::from_str(&line)?;
        Ok(message)
    }

    /// Write a JSON-RPC message to stdin
    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<()> {
        let stdin = self.stdin.as_mut()
            .ok_or(McpError::Transport("Not connected".to_string()))?;

        let json = serde_json::to_string(message)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn connect(&mut self, config: &TransportConfig) -> Result<()> {
        let (command, args, env) = match config {
            TransportConfig::Stdio { command, args, env } => (command, args, env),
            _ => return Err(McpError::Transport("Invalid config for stdio transport".to_string())),
        };

        // Spawn process
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(env);

        let mut child = cmd.spawn()?;

        // Take stdio handles
        let stdin = child.stdin.take()
            .ok_or(McpError::Transport("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or(McpError::Transport("Failed to get stdout".to_string()))?;
        let stderr = child.stderr.take()
            .ok_or(McpError::Transport("Failed to get stderr".to_string()))?;

        self.process = Some(child);
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.stderr = Some(BufReader::new(stderr));

        Ok(())
    }

    async fn request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Write request
        self.write_message(&JsonRpcMessage::Request(request.clone())).await?;

        // Read response (may need to skip notifications)
        loop {
            let message = self.read_message().await?;
            match message {
                JsonRpcMessage::Response(response) => {
                    if response.id == request.id {
                        return Ok(response);
                    }
                }
                JsonRpcMessage::Notification(_) => {
                    // Skip notifications, keep reading
                    continue;
                }
                _ => {
                    return Err(McpError::Protocol("Unexpected message type".to_string()));
                }
            }
        }
    }

    async fn notify(&mut self, notification: JsonRpcNotification) -> Result<()> {
        self.write_message(&JsonRpcMessage::Notification(notification)).await
    }

    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>> {
        // For stdio, we don't typically receive unsolicited messages
        // This is mainly for HTTP/SSE transport
        Ok(None)
    }

    async fn is_connected(&self) -> bool {
        self.process.is_some()
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Close stdin (signals server to shutdown)
        self.stdin = None;

        // Wait for process to exit (with timeout)
        if let Some(mut process) = self.process.take() {
            tokio::select! {
                result = process.wait() => {
                    match result {
                        Ok(status) => {
                            if !status.success() {
                                eprintln!("MCP server exited with status: {}", status);
                            }
                        }
                        Err(e) => eprintln!("Error waiting for MCP server: {}", e),
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    // Timeout, kill process
                    let _ = process.kill().await;
                }
            }
        }

        Ok(())
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }
}
```

### 5.4 Error Types

**`src/mcp/error.rs`:**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Plugin not running: {0}")]
    PluginNotRunning(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Duplicate plugin ID: {0}")]
    DuplicateId(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Unresolved environment variable: {0}")]
    UnresolvedEnvVar(String),

    #[error("JSON schema validation failed: {0}")]
    SchemaValidation(String),
}

pub type Result<T> = std::result::Result<T, McpError>;
```

---

## 6. Transport Layer Design

(Covered in sections 5.3 for stdio, HTTP implementation similar using reqwest-eventsource)

---

## 7. Plugin Lifecycle Management

### 7.1 State Machine

```
                    ┌─────────┐
                    │ Stopped │
                    └─────────┘
                         │
                    enable_plugin()
                         │
                         ▼
                   ┌──────────┐
                   │ Starting │
                   └──────────┘
                         │
              subprocess spawned / HTTP connected
                         │
                         ▼
                  ┌─────────────┐
                  │Initializing │
                  └─────────────┘
                         │
                MCP handshake complete
                         │
                         ▼
                   ┌──────────┐
                   │ Running  │◄──────────┐
                   └──────────┘           │
                         │              restart
                         │                │
                    tool calls         ┌───────┐
                    work normally      │ Error │
                         │             └───────┘
                    disable_plugin()      ▲
                    or error occurs       │
                         │                │
                         ▼            error occurs
                   ┌──────────┐           │
                   │ Stopping │───────────┘
                   └──────────┘
                         │
                process killed / connection closed
                         │
                         ▼
                    ┌─────────┐
                    │ Stopped │
                    └─────────┘
```

### 7.2 Automatic Restart Logic

**Crash Detection:**
- stdio: Process exit detected via `child.wait()`
- HTTP: Connection loss detected via failed requests

**Backoff Strategy:**
```rust
fn calculate_backoff(restart_count: u32) -> Duration {
    let base_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let delay = base_delay * 2_u32.pow(restart_count);
    delay.min(max_delay)
}
```

**Max Retries:** 5 attempts, then mark as permanently errored

**Implementation:**
```rust
async fn auto_restart_plugin(plugin_id: String, manager: Arc<Mutex<McpPluginManager>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let should_restart = {
            let mgr = manager.lock().await;
            let plugins = mgr.plugins.read().await;

            if let Some(state) = plugins.get(&plugin_id) {
                state.status == PluginStatus::Error && state.restart_count < 5
            } else {
                false
            }
        };

        if should_restart {
            let mut mgr = manager.lock().await;
            let backoff = {
                let plugins = mgr.plugins.read().await;
                let state = plugins.get(&plugin_id).unwrap();
                calculate_backoff(state.restart_count)
            };

            tokio::time::sleep(backoff).await;

            if let Err(e) = mgr.restart_plugin(&plugin_id).await {
                eprintln!("Failed to restart plugin {}: {}", plugin_id, e);
            }
        }
    }
}
```

---

## 8. UI Design: Plugins Pane

### 8.1 Layout Design

```
┌──────────────────────────────────────────────────────┐
│  MCP Plugins                        [+ Add] [⚙️]     │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ 🟢 Filesystem Access                 [Disable] │ │
│  │    ID: filesystem                               │ │
│  │    Status: Running · 12 tools                   │ │
│  │    ▼ Tools: read_file, write_file, list_dir... │ │
│  │    ▼ Config                                     │ │
│  │       Command: npx                              │ │
│  │       Args: -y, @modelcontextprotocol/server... │ │
│  │    📋 View Logs                                 │ │
│  └────────────────────────────────────────────────┘ │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ 🟢 SQLite Database                   [Disable] │ │
│  │    ID: sqlite                                   │ │
│  │    Status: Running · 8 tools                    │ │
│  │    ▶ Tools                                      │ │
│  │    ▶ Config                                     │ │
│  └────────────────────────────────────────────────┘ │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ 🔴 Git Repository                     [Enable] │ │
│  │    ID: git                                      │ │
│  │    Status: Stopped                              │ │
│  │    ▶ Config                                     │ │
│  └────────────────────────────────────────────────┘ │
│                                                      │
│  ┌────────────────────────────────────────────────┐ │
│  │ ⚠️ GitHub Integration              [Restart]   │ │
│  │    ID: github                                   │ │
│  │    Status: Error - Failed to initialize         │ │
│  │    ▶ Error Details                              │ │
│  │    ▶ Config                                     │ │
│  └────────────────────────────────────────────────┘ │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### 8.2 egui Implementation Sketch

**`src/ui/mcp_plugins_view.rs`:**

```rust
use egui::{Ui, ScrollArea, CollapsingHeader, RichText, Color32};
use egui_phosphor::regular::{CHECK_CIRCLE, X_CIRCLE, WARNING_CIRCLE, GEAR, PLUS};

pub struct McpPluginsView {
    plugin_manager: Arc<Mutex<McpPluginManager>>,
    selected_plugin: Option<String>,
    show_add_dialog: bool,
}

impl McpPluginsView {
    pub fn new(plugin_manager: Arc<Mutex<McpPluginManager>>) -> Self {
        Self {
            plugin_manager,
            selected_plugin: None,
            show_add_dialog: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        // Header with add button
        ui.horizontal(|ui| {
            ui.heading("MCP Plugins");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{} Add Plugin", PLUS)).clicked() {
                    self.show_add_dialog = true;
                }
                if ui.button(format!("{} Settings", GEAR)).clicked() {
                    // Open settings
                }
            });
        });

        ui.separator();

        // Scrollable plugin list
        ScrollArea::vertical().show(ui, |ui| {
            // Fetch plugin list (async, need to use runtime)
            let plugins = {
                let mgr = self.plugin_manager.blocking_lock();
                tokio::runtime::Handle::current().block_on(mgr.list_plugins())
            };

            for plugin in plugins {
                self.render_plugin_card(ui, &plugin);
                ui.add_space(8.0);
            }
        });

        // Add plugin dialog
        if self.show_add_dialog {
            self.render_add_dialog(ui.ctx());
        }
    }

    fn render_plugin_card(&mut self, ui: &mut Ui, plugin: &PluginInfo) {
        egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Status icon
                    let (icon, color) = match plugin.status {
                        PluginStatus::Running => (CHECK_CIRCLE, Color32::GREEN),
                        PluginStatus::Error => (X_CIRCLE, Color32::RED),
                        PluginStatus::Stopped => (X_CIRCLE, Color32::GRAY),
                        _ => (WARNING_CIRCLE, Color32::YELLOW),
                    };
                    ui.label(RichText::new(icon.to_string()).color(color).size(20.0));

                    // Plugin name
                    ui.label(RichText::new(&plugin.name).strong().size(16.0));

                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Enable/Disable button
                        let button_text = match plugin.status {
                            PluginStatus::Running => "Disable",
                            PluginStatus::Stopped => "Enable",
                            PluginStatus::Error => "Restart",
                            _ => "...",
                        };

                        if ui.button(button_text).clicked() {
                            self.handle_plugin_action(&plugin.id, button_text);
                        }
                    });
                });

                // Plugin ID and status
                ui.label(format!("ID: {}", plugin.id));
                let status_text = match &plugin.status {
                    PluginStatus::Running => format!("Running · {} tools", plugin.tool_count),
                    PluginStatus::Error => format!("Error: {}", plugin.error.as_ref().unwrap_or(&"Unknown".to_string())),
                    PluginStatus::Stopped => "Stopped".to_string(),
                    _ => format!("{:?}", plugin.status),
                };
                ui.label(status_text);

                // Collapsible sections
                if plugin.status == PluginStatus::Running && plugin.tool_count > 0 {
                    CollapsingHeader::new(format!("▶ Tools ({})", plugin.tool_count))
                        .show(ui, |ui| {
                            self.render_tools_list(ui, &plugin.id);
                        });
                }

                CollapsingHeader::new("▶ Config")
                    .show(ui, |ui| {
                        self.render_config(ui, &plugin.id);
                    });

                if plugin.status == PluginStatus::Error {
                    CollapsingHeader::new("▶ Error Details")
                        .show(ui, |ui| {
                            if let Some(error) = &plugin.error {
                                ui.label(RichText::new(error).color(Color32::RED));
                            }
                        });
                }
            });
    }

    fn render_tools_list(&self, ui: &mut Ui, plugin_id: &str) {
        let tools = {
            let mgr = self.plugin_manager.blocking_lock();
            tokio::runtime::Handle::current().block_on(async {
                let plugins = mgr.plugins.read().await;
                if let Some(state) = plugins.get(plugin_id) {
                    state.tools.clone()
                } else {
                    Vec::new()
                }
            })
        };

        for tool in tools {
            ui.label(format!("• {} - {}", tool.name, tool.description));
        }
    }

    fn render_config(&self, ui: &mut Ui, plugin_id: &str) {
        // Display JSON config in monospace font
        let config_json = {
            let mgr = self.plugin_manager.blocking_lock();
            tokio::runtime::Handle::current().block_on(async {
                let config = mgr.config.read().await;
                // Find plugin config
                if let Some(server) = config.mcp_plugins.local_servers.iter()
                    .find(|s| s.id == plugin_id) {
                    serde_json::to_string_pretty(server).unwrap_or_default()
                } else if let Some(service) = config.mcp_plugins.cloud_services.iter()
                    .find(|s| s.id == plugin_id) {
                    serde_json::to_string_pretty(service).unwrap_or_default()
                } else {
                    "{}".to_string()
                }
            })
        };

        ui.add(egui::TextEdit::multiline(&mut config_json.as_str())
            .font(egui::TextStyle::Monospace)
            .desired_rows(8)
            .interactive(false));
    }

    fn handle_plugin_action(&self, plugin_id: &str, action: &str) {
        let mgr = self.plugin_manager.clone();
        let plugin_id = plugin_id.to_string();
        let action = action.to_string();

        // Spawn async task
        tokio::spawn(async move {
            let mut mgr = mgr.lock().await;
            let result = match action.as_str() {
                "Enable" => mgr.enable_plugin(&plugin_id).await,
                "Disable" => mgr.disable_plugin(&plugin_id).await,
                "Restart" => mgr.restart_plugin(&plugin_id).await,
                _ => Ok(()),
            };

            if let Err(e) = result {
                eprintln!("Plugin action failed: {}", e);
            }
        });
    }

    fn render_add_dialog(&mut self, ctx: &egui::Context) {
        // Modal window for adding new plugin
        egui::Window::new("Add MCP Plugin")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Add a new MCP plugin configuration");
                // Form fields for plugin config
                // ...
            });
    }
}
```

---

## 9. Integration Points

### 9.1 Tool Registry Integration

**Register MCP tools:**
```rust
// In ToolRegistry
impl ToolRegistry {
    pub fn register_mcp_tool(&mut self, plugin_id: String, tool: McpTool) -> Result<()> {
        let tool_def = ToolDefinition {
            name: format!("{}_{}", plugin_id, tool.name), // Prefix with plugin ID
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
            handler: ToolHandler::Mcp {
                plugin_id,
                tool_name: tool.name,
            },
        };

        self.tools.insert(tool_def.name.clone(), tool_def);
        Ok(())
    }

    pub fn unregister_plugin_tools(&mut self, plugin_id: &str) -> Result<()> {
        self.tools.retain(|name, tool| {
            !matches!(&tool.handler, ToolHandler::Mcp { plugin_id: pid, .. } if pid == plugin_id)
        });
        Ok(())
    }
}

pub enum ToolHandler {
    BuiltIn(Box<dyn Fn(serde_json::Value) -> Result<serde_json::Value>>),
    Mcp { plugin_id: String, tool_name: String },
}
```

**Execute MCP tool:**
```rust
// In ToolRegistry::execute()
match &tool_def.handler {
    ToolHandler::BuiltIn(func) => func(arguments),
    ToolHandler::Mcp { plugin_id, tool_name } => {
        // Call plugin manager
        let result = plugin_manager.call_tool(plugin_id, tool_name, arguments).await?;

        // Convert ToolCallResult to JSON
        Ok(serde_json::to_value(result)?)
    }
}
```

### 9.2 Event Bus Integration

**New events:**
```rust
pub enum AppEvent {
    // ... existing events ...

    /// MCP plugin status changed
    McpPluginStatusChanged {
        plugin_id: String,
        status: PluginStatus,
    },

    /// MCP plugin tools updated
    McpPluginToolsUpdated {
        plugin_id: String,
        tools: Vec<McpTool>,
    },

    /// MCP plugin error
    McpPluginError {
        plugin_id: String,
        error: String,
    },
}
```

**UI subscription:**
```rust
// In McpPluginsView::update()
for event in event_rx.try_iter() {
    match event {
        AppEvent::McpPluginStatusChanged { plugin_id, status } => {
            // Update UI state
            self.request_repaint();
        }
        AppEvent::McpPluginError { plugin_id, error } => {
            // Show error notification
            self.show_error_toast(plugin_id, error);
        }
        _ => {}
    }
}
```

### 9.3 Agent System Integration

**Tool availability in agent context:**
```rust
// Before sending message to LLM, include MCP tools in context
let available_tools = plugin_manager.get_all_tools().await;
let tool_schemas: Vec<_> = available_tools.iter()
    .map(|(plugin_id, tool)| {
        json!({
            "name": format!("{}_{}", plugin_id, tool.name),
            "description": tool.description,
            "input_schema": tool.input_schema,
        })
    })
    .collect();

// Include in OpenRouter API request
let request = json!({
    "model": agent.model,
    "messages": conversation_history,
    "tools": tool_schemas,
    // ...
});
```

---

## 10. Error Handling Strategy

### 10.1 Error Categories

1. **Configuration Errors** (user-fixable)
   - Invalid JSON
   - Duplicate IDs
   - Missing environment variables
   - **UI Feedback:** Red error message with fix instructions

2. **Connection Errors** (transient)
   - Process spawn failed
   - HTTP connection timeout
   - **Strategy:** Auto-retry with exponential backoff

3. **Protocol Errors** (developer error)
   - Invalid JSON-RPC
   - Unsupported protocol version
   - **Strategy:** Log error, mark plugin as errored, notify user

4. **Runtime Errors** (server-side)
   - Tool execution failed
   - Server crashed
   - **Strategy:** Return error to agent, attempt auto-restart

### 10.2 Error Recovery

**Automatic:**
- Connection failures → retry with backoff (max 5 attempts)
- Process crashes → auto-restart with backoff
- Protocol errors → mark as errored, manual intervention required

**Manual:**
- Configuration errors → user must fix config
- Persistent failures → user can restart plugin from UI

### 10.3 User-Facing Error Messages

**Examples:**

❌ **Configuration Error:**
> **Plugin "github" failed to start**
>
> Environment variable `GITHUB_TOKEN` is not set.
>
> **Fix:** Set `GITHUB_TOKEN` in your environment or update the plugin configuration.

⚠️ **Connection Error:**
> **Plugin "filesystem" is temporarily unavailable**
>
> Failed to spawn process: `npx` command not found.
>
> **Retrying automatically...** (Attempt 2 of 5)

🔴 **Runtime Error:**
> **Tool execution failed: read_file**
>
> Permission denied: /etc/shadow
>
> The server reported an error during tool execution.

---

## 11. Security Considerations

### 11.1 Process Isolation

**stdio servers:**
- Run as child processes
- OS-level process isolation
- Limited access to Rustbot's memory
- Cannot access other plugins

### 11.2 File System Access

**Current approach:** Trust MCP servers to implement their own restrictions

**Future enhancement:**
- Plugin permissions system
- Declare allowed paths in configuration
- Rustbot validates resource URIs before allowing access

**Example future config:**
```json
{
  "id": "filesystem",
  "permissions": {
    "file_read": ["/Users/masa/Projects/**"],
    "file_write": ["/Users/masa/Projects/rustbot/**"],
    "file_delete": []
  }
}
```

### 11.3 Environment Variable Security

**Current:**
- Environment variables stored in plain text config
- Resolved at runtime

**Recommendation:**
- Warn users not to commit `.env` files
- Consider encrypted config storage (future)

**Best practice for users:**
```bash
# .env file (not committed)
export GITHUB_TOKEN="ghp_..."
export API_KEY="sk_..."

# Load before running Rustbot
source .env && rustbot
```

### 11.4 OAuth Token Storage

**For HTTP transport:**
- Store OAuth tokens securely (keychain/credential manager)
- Never log tokens
- Expire tokens appropriately
- Support token refresh

---

## 12. Implementation Roadmap

### Phase 1: Foundation (Week 1)

**Goals:**
- Basic MCP protocol types
- Configuration schema
- Plugin manager skeleton

**Deliverables:**
- [ ] `src/mcp/protocol.rs` - JSON-RPC types, MCP message types
- [ ] `src/mcp/config.rs` - Configuration structs with serde
- [ ] `src/mcp/error.rs` - Error types
- [ ] `src/mcp/manager.rs` - Plugin manager (stub implementation)
- [ ] Unit tests for config loading and validation

**Testing:**
- Load sample configuration
- Validate configuration (duplicate IDs, env vars)
- Serialize/deserialize protocol types

---

### Phase 2: stdio Transport (Week 2)

**Goals:**
- Implement stdio transport
- Process lifecycle management
- MCP handshake

**Deliverables:**
- [ ] `src/mcp/transport/mod.rs` - Transport trait
- [ ] `src/mcp/transport/stdio.rs` - Full stdio implementation
- [ ] Process spawning with tokio::process
- [ ] stdin/stdout communication
- [ ] MCP initialize/initialized handshake
- [ ] tools/list request
- [ ] Integration tests with real MCP server (filesystem)

**Testing:**
- Spawn official `@modelcontextprotocol/server-filesystem`
- Perform handshake
- List tools
- Call a simple tool (e.g., `list_directory`)
- Clean shutdown

---

### Phase 3: Plugin Manager Core (Week 3)

**Goals:**
- Complete plugin manager implementation
- Tool registration
- Error handling and recovery

**Deliverables:**
- [ ] Complete `McpPluginManager` implementation
- [ ] Plugin state management
- [ ] Tool registration with ToolRegistry
- [ ] Auto-restart logic with backoff
- [ ] Event bus integration
- [ ] Configuration hot-reload
- [ ] Integration tests

**Testing:**
- Start multiple plugins simultaneously
- Enable/disable plugins
- Simulate plugin crash and verify auto-restart
- Test tool execution through manager
- Verify tool registry integration

---

### Phase 4: UI Integration (Week 4)

**Goals:**
- Plugins pane in egui
- Plugin management UI
- Status display

**Deliverables:**
- [ ] `src/ui/mcp_plugins_view.rs` - Plugins pane
- [ ] Plugin cards with status icons
- [ ] Enable/disable buttons
- [ ] Configuration viewer
- [ ] Tool list display
- [ ] Error message display
- [ ] Add plugin dialog (basic)

**Testing:**
- Manual testing of all UI interactions
- Enable/disable plugins from UI
- View tools and configuration
- Display error states
- Test with multiple plugins

---

### Phase 5: HTTP Transport (Week 5+)

**Goals:**
- HTTP/SSE transport for cloud services
- OAuth authentication
- Session management

**Deliverables:**
- [ ] `src/mcp/transport/http.rs` - HTTP transport
- [ ] reqwest-eventsource integration
- [ ] OAuth 2.1 flow implementation
- [ ] Session ID management
- [ ] Streamable HTTP support
- [ ] Integration tests with test server

**Testing:**
- Mock HTTP MCP server
- Test OAuth flow
- Test SSE streaming
- Test session management
- Test reconnection logic

---

### Phase 6: Polish and Documentation (Ongoing)

**Goals:**
- User documentation
- Error message improvements
- Performance optimization

**Deliverables:**
- [ ] User guide: How to add MCP plugins
- [ ] Developer guide: How to create MCP servers
- [ ] Example configurations for popular servers
- [ ] Error message refinement
- [ ] Performance profiling and optimization
- [ ] Security audit

---

## 13. Testing Strategy

### 13.1 Unit Tests

**Components to test:**
- Configuration loading/validation
- Protocol message serialization
- Error handling
- Environment variable resolution

**Example test:**
```rust
#[tokio::test]
async fn test_config_validation() {
    let config = PluginConfig {
        mcp_plugins: McpPlugins {
            local_servers: vec![
                LocalServerConfig {
                    id: "test1".to_string(),
                    // ...
                },
                LocalServerConfig {
                    id: "test1".to_string(), // Duplicate!
                    // ...
                },
            ],
            cloud_services: vec![],
        },
    };

    let result = validate_config(&config);
    assert!(matches!(result, Err(McpError::Config(ConfigError::DuplicateId(_)))));
}
```

### 13.2 Integration Tests

**Test with real MCP servers:**
```rust
#[tokio::test]
async fn test_filesystem_server() {
    let mut transport = StdioTransport::new();

    transport.connect(&TransportConfig::Stdio {
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
            "/tmp".to_string(),
        ],
        env: HashMap::new(),
    }).await.unwrap();

    // Handshake
    let (capabilities, tools) = perform_handshake(&mut transport).await.unwrap();

    // Verify tools
    assert!(tools.iter().any(|t| t.name == "read_file"));

    // Clean shutdown
    transport.disconnect().await.unwrap();
}
```

### 13.3 Manual Testing Checklist

**Plugin Management:**
- [ ] Add new plugin via UI
- [ ] Enable plugin
- [ ] Disable plugin
- [ ] Restart plugin after error
- [ ] Remove plugin

**Tool Execution:**
- [ ] Execute filesystem tools (read_file, write_file)
- [ ] Execute SQLite tools (query)
- [ ] Execute Git tools (log, diff)
- [ ] Handle tool execution errors gracefully

**Error Scenarios:**
- [ ] Missing environment variable
- [ ] Invalid command
- [ ] Server crash during operation
- [ ] Duplicate plugin IDs
- [ ] Network timeout (HTTP transport)

**Performance:**
- [ ] Multiple plugins running concurrently
- [ ] Rapid enable/disable cycles
- [ ] Large tool result payloads
- [ ] Memory usage with many plugins

---

## 14. Future Enhancements

### 14.1 Plugin Marketplace (Phase 6+)

**Concept:** Built-in marketplace for discovering and installing MCP plugins

**Features:**
- Browse curated MCP servers
- One-click installation
- Automatic configuration
- Version management
- Update notifications

**Implementation:**
- Curated JSON catalog (GitHub-hosted)
- Installation script generator
- npm/cargo integration

### 14.2 Plugin Permissions System (Phase 7+)

**Concept:** Fine-grained control over what plugins can access

**Configuration:**
```json
{
  "id": "filesystem",
  "permissions": {
    "file_read": ["/Users/masa/Projects/**"],
    "file_write": ["/Users/masa/Projects/rustbot/**"],
    "network_access": false,
    "env_access": ["HOME", "USER"]
  }
}
```

**Enforcement:**
- Rustbot intercepts resource/tool requests
- Validates against permission policy
- Denies unauthorized access

### 14.3 Plugin Analytics (Phase 8+)

**Metrics:**
- Tool usage frequency
- Plugin uptime
- Error rates
- Performance metrics

**UI:**
- Dashboard showing plugin health
- Usage charts
- Performance trends

### 14.4 Custom Transport Plugins (Phase 9+)

**Concept:** Allow users to implement custom transport mechanisms

**Example use cases:**
- Unix socket transport
- Named pipe transport
- gRPC transport
- WebSocket transport

**Implementation:**
- Dynamic library loading
- Transport plugin API
- Registration mechanism

### 14.5 Plugin Sandboxing (Phase 10+)

**Concept:** Run plugins in isolated containers/VMs

**Technologies:**
- Docker containers
- WASM sandboxing
- seccomp filters

**Benefits:**
- Enhanced security
- Resource limits
- Network isolation

---

## 15. Appendix

### A. MCP Protocol Version Compatibility

| Protocol Version | Release Date | Changes | Support Status |
|------------------|--------------|---------|----------------|
| 2024-11-05 | Nov 2024 | Initial stable release | ✅ Supported |
| 2025-06-18 | Jun 2025 | OAuth 2.1, Resource indicators | ✅ **Recommended** |

**Strategy:** Support latest version, maintain backward compatibility

---

### B. Official MCP Servers Reference

| Server | Package | Purpose | Tools |
|--------|---------|---------|-------|
| Filesystem | `@modelcontextprotocol/server-filesystem` | File operations | read_file, write_file, create_directory, list_directory, move_file, search_files, get_file_info |
| SQLite | `@modelcontextprotocol/server-sqlite` | Database queries | read_query, write_query, create_table, list_tables, describe_table, append_insight |
| Git | `@modelcontextprotocol/server-git` | Repository access | git_status, git_diff_unstaged, git_diff_staged, git_commit, git_add, git_reset, git_log, search_files, read_file, list_directory |
| GitHub | `@modelcontextprotocol/server-github` | GitHub API | create_or_update_file, search_repositories, create_repository, get_file_contents, push_files, create_issue, create_pull_request, fork_repository, create_branch |

---

### C. Rust Crate Dependencies Summary

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `rmcp` | 0.x | MCP SDK | MIT |
| `tokio` | 1.40 | Async runtime | MIT |
| `reqwest` | 0.12 | HTTP client | MIT/Apache-2.0 |
| `reqwest-eventsource` | 0.6 | SSE support | MIT |
| `serde` | 1.0 | Serialization | MIT/Apache-2.0 |
| `serde_json` | 1.0 | JSON support | MIT/Apache-2.0 |
| `schemars` | 0.8 | JSON Schema | MIT |
| `async-trait` | 0.1 | Async traits | MIT/Apache-2.0 |
| `thiserror` | 1.0 | Error types | MIT/Apache-2.0 |

---

**End of Design Document**
