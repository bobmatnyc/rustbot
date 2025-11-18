//! MCP Plugin Manager Demo
//!
//! This example demonstrates MCP functionality through Phase 2:
//! - Loading configuration from JSON
//! - Listing available plugins
//! - Starting/stopping plugins (Phase 2)
//! - Tool discovery (Phase 2)
//! - Tool execution (Phase 2)
//!
//! Run with: cargo run --example mcp_demo
//!
//! Note: Requires actual MCP servers to be installed for Phase 2 features.
//! Example: npm install -g @modelcontextprotocol/server-filesystem

use rustbot::mcp::{McpPluginManager, PluginState};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MCP Plugin Manager Demo (Phase 1 + 2) ===\n");

    // Create manager
    let mut manager = McpPluginManager::new();
    println!("✓ Created McpPluginManager");

    // Load configuration
    let config_path = Path::new("mcp_config.json");
    match manager.load_config(config_path).await {
        Ok(_) => println!("✓ Loaded configuration from {}", config_path.display()),
        Err(e) => {
            eprintln!("✗ Failed to load config: {}", e);
            eprintln!("\nMake sure mcp_config.json exists in the project root.");
            return Err(e.into());
        }
    }

    // Get plugin count
    let count = manager.plugin_count().await;
    println!("\n📊 Plugin Statistics:");
    println!("   Total plugins: {}", count);

    // List all plugins
    let plugins = manager.list_plugins().await;
    println!("\n📋 Available Plugins:\n");

    for (i, plugin) in plugins.iter().enumerate() {
        println!("{}. {}", i + 1, plugin.name);
        println!("   ID: {}", plugin.id);
        if let Some(desc) = &plugin.description {
            println!("   Description: {}", desc);
        }
        println!("   Type: {:?}", plugin.plugin_type);
        println!("   State: {:?}", plugin.state);
        println!("   Tools: {}", plugin.tool_count);

        if let Some(err) = &plugin.error_message {
            println!("   ⚠️  Error: {}", err);
        }

        // Show state-specific information
        match plugin.state {
            PluginState::Disabled => println!("   💡 Enable in mcp_config.json to use"),
            PluginState::Stopped => println!("   ⏸️  Ready to start (Phase 2+)"),
            PluginState::Running => println!("   ✅ Operational"),
            PluginState::Error { .. } => println!("   ❌ Needs attention"),
            _ => println!("   ⏳ Transitioning..."),
        }

        println!();
    }

    // Get detailed metadata for a specific plugin
    if let Some(filesystem) = manager.get_plugin("filesystem").await {
        println!("🔍 Detailed View: {}", filesystem.name);
        println!("   Plugin Type: {:?}", filesystem.plugin_type);
        println!("   Current State: {:?}", filesystem.state);
        println!("   Restart Count: {}", filesystem.restart_count);
        println!("   Tools Available: {}", filesystem.tools.len());
        println!("   Resources Available: {}", filesystem.resources.len());
        println!("   Prompts Available: {}", filesystem.prompts.len());

        if filesystem.is_running() {
            println!("   ✅ Plugin is operational");
        } else if filesystem.is_error() {
            println!("   ❌ Plugin has error: {:?}", filesystem.error_message());
        } else {
            println!("   ⏸️  Plugin is not running");
        }
    }

    // Phase 2: Try to start a plugin (requires MCP server to be installed)
    println!("\n🚀 Phase 2: Starting plugins...\n");

    // Try to start filesystem plugin (if configured)
    if manager.has_plugin("filesystem").await {
        println!("Attempting to start 'filesystem' plugin...");
        match manager.start_plugin("filesystem").await {
            Ok(_) => {
                println!("✓ Started filesystem plugin successfully!\n");

                // List tools
                if let Some(plugin) = manager.get_plugin("filesystem").await {
                    println!("📦 Available tools:");
                    for tool in &plugin.tools {
                        println!("   - {}", tool.name);
                        if let Some(desc) = &tool.description {
                            println!("     {}", desc);
                        }
                    }
                    println!();
                }

                // Try to execute a tool (example: list files in /tmp)
                println!("🔧 Executing tool: list_directory");
                match manager
                    .execute_tool(
                        "filesystem",
                        "list_directory",
                        Some(serde_json::json!({"path": "/tmp"})),
                    )
                    .await
                {
                    Ok(result) => {
                        println!("✓ Tool execution successful!");
                        println!(
                            "   Result preview: {}...",
                            result.chars().take(200).collect::<String>()
                        );
                    }
                    Err(e) => {
                        println!("⚠️  Tool execution failed: {}", e);
                        println!("   (This is expected if the tool/path doesn't exist)");
                    }
                }

                // Stop plugin
                println!("\n⏸️  Stopping plugin...");
                manager.stop_plugin("filesystem").await?;
                println!("✓ Plugin stopped successfully");
            }
            Err(e) => {
                println!("⚠️  Could not start filesystem plugin: {}", e);
                println!("   This is expected if MCP server is not installed.");
                println!("   Install with: npm install -g @modelcontextprotocol/server-filesystem");
            }
        }
    } else {
        println!("⚠️  No 'filesystem' plugin configured");
        println!("   Add to mcp_config.json to test Phase 2 functionality");
    }

    println!("\n=== Demo Complete ===");
    println!("\nImplementation Status:");
    println!("  ✅ Phase 1: Configuration & Metadata");
    println!("  ✅ Phase 2: stdio Transport & Tool Execution");
    println!("  ⏳ Phase 3: Auto-restart & Tool Registry");
    println!("  ⏳ Phase 4: UI Integration");
    println!("  ⏳ Phase 5: HTTP Transport & Cloud Services");

    Ok(())
}
