//! MCP Auto-Registration Demo
//!
//! This example demonstrates the automatic tool registration feature.
//! When MCP plugins start, their tools are automatically registered
//! via the event bus without manual registration calls.
//!
//! Run with: cargo run --example mcp_auto_registration_demo

use rustbot::api::RustbotApi;
use rustbot::events::EventBus;
use rustbot::mcp::McpPluginManager;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for log output
    tracing_subscriber::fmt::init();

    println!("=== MCP Auto-Registration Demo ===\n");

    // Create shared event bus
    let event_bus = Arc::new(EventBus::new());

    // Create runtime
    let runtime = Arc::new(tokio::runtime::Runtime::new()?);

    // Create API instance
    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

    // Create MCP manager with event bus integration
    let mut mcp_manager = McpPluginManager::with_event_bus(Some(Arc::clone(&event_bus)));

    // Load MCP configuration
    let config_path = Path::new("mcp_config.json");
    match mcp_manager.load_config(config_path).await {
        Ok(_) => println!("✓ Loaded MCP configuration"),
        Err(e) => {
            eprintln!("⚠️  Could not load MCP config: {}", e);
            eprintln!("   This demo requires mcp_config.json");
            return Ok(());
        }
    }

    // Connect API to MCP manager
    api.set_mcp_manager(Arc::new(Mutex::new(mcp_manager.clone())));

    // Wrap API in Arc<Mutex> for auto-registration
    let api = Arc::new(Mutex::new(api));

    // Start automatic tool registration task
    println!("Starting automatic tool registration...");
    let _auto_reg_task = RustbotApi::start_mcp_auto_registration(Arc::clone(&api)).await;
    println!("✓ Auto-registration task started\n");

    // List available plugins
    let plugins = mcp_manager.list_plugins().await;
    println!("📋 Available Plugins ({}):\n", plugins.len());

    for plugin in &plugins {
        println!("  • {}", plugin.name);
        println!("    ID: {}", plugin.id);
        println!("    State: {:?}", plugin.state);
        println!();
    }

    // Try to start a plugin (if configured)
    if mcp_manager.has_plugin("filesystem").await {
        println!("🚀 Starting 'filesystem' plugin...\n");

        match mcp_manager.start_plugin("filesystem").await {
            Ok(_) => {
                println!("✓ Plugin started successfully");

                // Wait briefly for auto-registration to complete
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Check registered tools
                let api_guard = api.lock().await;
                let all_tools = api_guard.get_all_tools();

                let mcp_tools: Vec<_> = all_tools
                    .iter()
                    .filter(|t| t.function.name.starts_with("mcp:filesystem:"))
                    .collect();

                println!("\n📦 Auto-registered Tools ({}):\n", mcp_tools.len());

                for tool in mcp_tools {
                    println!("  • {}", tool.function.name);
                    println!("    {}", tool.function.description);
                }

                drop(api_guard);

                // Stop plugin to demonstrate auto-unregistration
                println!("\n⏸️  Stopping plugin...\n");
                mcp_manager.stop_plugin("filesystem").await?;

                // Wait for auto-unregistration
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Verify tools were removed
                let api_guard = api.lock().await;
                let all_tools_after = api_guard.get_all_tools();
                let remaining_mcp_tools: Vec<_> = all_tools_after
                    .iter()
                    .filter(|t| t.function.name.starts_with("mcp:filesystem:"))
                    .collect();

                println!("✓ Plugin stopped");
                println!(
                    "✓ Tools auto-unregistered (remaining: {})\n",
                    remaining_mcp_tools.len()
                );
            }
            Err(e) => {
                println!("⚠️  Could not start plugin: {}", e);
                println!("   Make sure the MCP server is installed:");
                println!("   npm install -g @modelcontextprotocol/server-filesystem\n");
            }
        }
    } else {
        println!("⚠️  No 'filesystem' plugin configured in mcp_config.json\n");
    }

    println!("=== Demo Complete ===\n");
    println!("Key Features Demonstrated:");
    println!("  ✓ Event-driven architecture");
    println!("  ✓ Automatic tool registration on plugin start");
    println!("  ✓ Automatic tool unregistration on plugin stop");
    println!("  ✓ No manual registration calls required");
    println!("  ✓ Decoupled API and MCP manager lifecycles\n");

    Ok(())
}
