# MCP Plugin System - Quick Start Guide

**For Rustbot Developers**

This guide provides a quick overview of the MCP (Model Context Protocol) plugin system implementation.

---

## What is MCP?

MCP (Model Context Protocol) enables AI applications to connect with external tools and data sources through a standardized interface. Think of it as a universal plugin system for AI assistants.

**Examples:**
- **Filesystem plugin**: Read/write files in allowed directories
- **Git plugin**: Access repository information and history
- **SQLite plugin**: Query local databases
- **GitHub plugin**: Manage repositories, issues, and PRs

---

## Phase 1 Status (Current Implementation)

✅ **Available Now:**
- Plugin configuration loading
- Plugin metadata tracking
- State management (Disabled/Stopped/Starting/Running/Error)
- Configuration validation
- Example configuration with 4 plugins

⏳ **Coming in Phase 2:**
- Actually starting/stopping plugins
- stdio transport for local servers
- Tool discovery and execution
- MCP protocol implementation

---

## Quick Usage

### Loading and Listing Plugins

```rust
use rustbot::mcp::McpPluginManager;
use std::path::Path;

#[tokio::main]
async fn main() {
    // Create manager
    let mut manager = McpPluginManager::new();

    // Load configuration
    manager.load_config(Path::new("mcp_config.json")).await.unwrap();

    // List plugins
    for plugin in manager.list_plugins().await {
        println!("{}: {:?}", plugin.name, plugin.state);
    }
}
```

### Checking Plugin Details

```rust
// Get specific plugin
if let Some(plugin) = manager.get_plugin("filesystem").await {
    println!("Name: {}", plugin.name);
    println!("State: {:?}", plugin.state);
    println!("Tools: {}", plugin.tools.len());

    if plugin.is_running() {
        println!("Plugin is operational!");
    }
}
```

---

## Configuration Format

**File:** `mcp_config.json`

```json
{
  "mcp_plugins": {
    "local_servers": [
      {
        "id": "filesystem",
        "name": "Filesystem Access",
        "description": "Access and manipulate files",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"],
        "env": {
          "ALLOWED_DIRS": "/path"
        },
        "enabled": false,
        "auto_restart": true,
        "timeout": 60
      }
    ],
    "cloud_services": []
  }
}
```

### Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique plugin identifier |
| `name` | Yes | Display name for UI |
| `description` | No | What the plugin does |
| `command` | Yes | Executable to run |
| `args` | No | Command-line arguments |
| `env` | No | Environment variables |
| `enabled` | No | Auto-start on load (default: true) |
| `auto_restart` | No | Restart on failure (default: false) |
| `timeout` | No | Seconds before timeout (default: 60) |

### Environment Variables

Use `${VAR_NAME}` for environment variable substitution:

```json
{
  "env": {
    "GITHUB_TOKEN": "${GITHUB_TOKEN}",
    "API_KEY": "${MY_SECRET_KEY}"
  }
}
```

Then set in shell:
```bash
export GITHUB_TOKEN="ghp_abc123..."
export MY_SECRET_KEY="sk_xyz789..."
```

---

## Plugin State Machine

```
Disabled ──enable──▶ Stopped ──start──▶ Starting ──spawn──▶ Initializing
                        ▲                                         │
                        │                                         │
                        │                                    handshake
                        │                                         │
                     disable                                      ▼
                        │                                      Running
                        │                                         │
                        └────────────── stop/error ───────────────┘
```

**States:**
- `Disabled`: Plugin is configured but not enabled
- `Stopped`: Ready to start (Phase 2+)
- `Starting`: Process spawning / connecting
- `Initializing`: Performing MCP handshake
- `Running`: Operational and ready for tool calls
- `Error`: Failed with error message

---

## Testing

### Run All MCP Tests
```bash
cargo test --lib mcp
```

### Run Demo Example
```bash
cargo run --example mcp_demo
```

### Expected Output
```
✓ Created McpPluginManager
✓ Loaded configuration from mcp_config.json

📊 Plugin Statistics:
   Total plugins: 4

📋 Available Plugins:

1. Filesystem Access
   ID: filesystem
   State: Disabled
   💡 Enable in mcp_config.json to use
```

---

## Module Structure

```
src/mcp/
├── mod.rs           # Module exports and documentation
├── error.rs         # MCP-specific error types
├── config.rs        # Configuration schema and loading
├── plugin.rs        # Plugin state and metadata
└── manager.rs       # Plugin lifecycle coordinator
```

### Public API

```rust
// Re-exported from rustbot::mcp
pub use mcp::{
    // Manager
    McpPluginManager,
    PluginInfo,

    // Configuration
    McpConfig,
    LocalServerConfig,
    CloudServiceConfig,

    // Plugin metadata
    PluginMetadata,
    PluginState,
    PluginType,
    ToolInfo,

    // Errors
    McpError,
    Result,
};
```

---

## Adding a New Plugin

1. **Edit mcp_config.json:**
   ```json
   {
     "id": "my-plugin",
     "name": "My Custom Plugin",
     "command": "node",
     "args": ["./path/to/server.js"],
     "enabled": true
   }
   ```

2. **Reload configuration:**
   ```rust
   manager.load_config(Path::new("mcp_config.json")).await?;
   ```

3. **Verify plugin appears:**
   ```rust
   assert!(manager.has_plugin("my-plugin").await);
   ```

---

## Common Patterns

### Checking if Plugin Exists
```rust
if manager.has_plugin("filesystem").await {
    println!("Filesystem plugin is configured");
}
```

### Getting Plugin Count
```rust
let count = manager.plugin_count().await;
println!("Total plugins: {}", count);
```

### Filtering by State
```rust
let plugins = manager.list_plugins().await;
let running = plugins.iter().filter(|p| p.state == PluginState::Running);
println!("Running plugins: {}", running.count());
```

---

## Error Handling

All MCP operations return `Result<T, McpError>`:

```rust
use rustbot::mcp::McpError;

match manager.load_config(path).await {
    Ok(_) => println!("✓ Config loaded"),
    Err(McpError::Config(msg)) => eprintln!("Config error: {}", msg),
    Err(McpError::Io(e)) => eprintln!("File error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

**Error Types:**
- `Config`: Configuration validation errors
- `PluginNotFound`: Plugin ID doesn't exist
- `PluginAlreadyExists`: Duplicate plugin ID
- `Transport`: Communication errors (Phase 2+)
- `Protocol`: MCP protocol violations (Phase 2+)
- `Io`: File system errors
- `Json`: JSON parsing errors

---

## Phase 2 Preview (Coming Soon)

Once Phase 2 is implemented, you'll be able to:

```rust
// Start a plugin
manager.start_plugin("filesystem").await?;

// Execute a tool
let result = manager.call_tool(
    "filesystem",
    "read_file",
    json!({ "path": "/path/to/file.txt" })
).await?;

// Stop a plugin
manager.stop_plugin("filesystem").await?;
```

---

## Troubleshooting

### "Duplicate plugin ID" error
- Check `mcp_config.json` for duplicate `id` fields
- IDs must be unique across both `local_servers` and `cloud_services`

### "Environment variable not found" error
- Set the required environment variable in your shell
- Or update the config to use a literal value instead of `${VAR_NAME}`

### "Plugin not found" error
- Verify the plugin ID matches exactly (case-sensitive)
- Use `manager.list_plugins()` to see all available IDs

---

## Resources

- **Full Documentation:** See `MCP_PHASE1_SUMMARY.md`
- **Design Document:** See `docs/design/MCP_PLUGIN_ARCHITECTURE.md`
- **Verification Checklist:** See `VERIFICATION_CHECKLIST.md`
- **Example Code:** See `examples/mcp_demo.rs`
- **MCP Specification:** https://spec.modelcontextprotocol.io/

---

## Contributing

When extending the MCP system:

1. **Add tests** for new functionality
2. **Document design decisions** in module comments
3. **Update examples** if API changes
4. **Follow existing patterns** (async-first, Arc<RwLock<>>)
5. **Validate configurations** before using

---

**Questions?** Review the comprehensive documentation in `MCP_PHASE1_SUMMARY.md` or check the example code in `examples/mcp_demo.rs`.
