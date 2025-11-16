//! MCP Plugin Manager Demo
//!
//! This example demonstrates the Phase 1 MCP functionality:
//! - Loading configuration from JSON
//! - Listing available plugins
//! - Querying plugin states
//! - Checking plugin metadata
//!
//! Run with: cargo run --example mcp_demo

use rustbot::mcp::{McpPluginManager, PluginState};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MCP Plugin Manager Demo (Phase 1) ===\n");

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

    println!("\n=== Phase 1 Demo Complete ===");
    println!("\nPhase 1 Status:");
    println!("  ✅ Configuration loading");
    println!("  ✅ Plugin metadata tracking");
    println!("  ✅ State management");
    println!("  ⏳ Plugin starting/stopping (Phase 2)");
    println!("  ⏳ Tool execution (Phase 2)");
    println!("  ⏳ UI integration (Phase 4)");

    Ok(())
}
