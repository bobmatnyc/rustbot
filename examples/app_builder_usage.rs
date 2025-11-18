// Example demonstrating AppBuilder usage for production and testing
//
// This example shows how to use AppBuilder to construct application
// dependencies with proper configuration and validation.

use rustbot::{AppBuilder, Result};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== AppBuilder Usage Example ===\n");

    // Get API key from environment
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("⚠️  OPENROUTER_API_KEY not set in environment");
            println!("   This example requires a valid API key to demonstrate production mode.");
            println!("   Set the environment variable and try again.");
            println!("\nExample: export OPENROUTER_API_KEY=sk-or-v1-...\n");
            return Ok(());
        }
    };

    // Example 1: Production configuration
    println!("1. Building production dependencies...");
    match AppBuilder::new()
        .with_api_key(api_key.clone())
        .with_base_path(PathBuf::from("."))
        .with_system_instructions("Production system instructions".to_string())
        .with_production_deps()
        .await
    {
        Ok(builder) => match builder.build() {
            Ok(mut deps) => {
                println!("   ✓ Filesystem configured");
                println!("   ✓ Storage service configured");
                println!("   ✓ Config service configured");
                println!(
                    "   ✓ Agent service configured ({} agents loaded)",
                    deps.agent_service.list_agents().len()
                );
                println!("   ✓ Runtime configured");
                println!(
                    "   ✓ Event bus configured ({} subscribers)",
                    deps.event_bus.subscriber_count()
                );
                println!(
                    "   ✓ LLM adapter configured: {}",
                    if deps.llm_adapter.is_some() {
                        "Yes"
                    } else {
                        "No"
                    }
                );

                // Show loaded agents
                let agents = deps.agent_service.list_agents();
                if !agents.is_empty() {
                    println!("\n   Loaded agents:");
                    for agent_id in agents {
                        println!("   - {}", agent_id);
                    }
                }

                // Demonstrate current agent access
                let current = deps.agent_service.current_agent();
                println!("\n   Current agent: {}", current.id());

                // Prevent runtime drop panic by taking ownership and forgetting
                // In a real app, the runtime would live for the entire program duration
                let _runtime = deps.runtime.take();
                std::mem::forget(_runtime);
            }
            Err(e) => {
                println!("   ✗ Failed to build dependencies: {}", e);
            }
        },
        Err(e) => {
            println!("   ✗ Failed to configure production deps: {}", e);
        }
    }

    println!("\n2. Builder pattern features:");
    println!("   - Method chaining for readable configuration");
    println!("   - Compile-time validation of dependency types");
    println!("   - Runtime validation of required dependencies");
    println!("   - Easy override of any dependency for testing");

    println!("\n3. Testing configuration:");
    println!("   In tests, use .with_test_deps() for mock implementations:");
    println!("   ```rust");
    println!("   let deps = AppBuilder::new()");
    println!("       .with_test_deps()");
    println!("       .with_api_key(\"test_key\".to_string())");
    println!("       .with_agent_service(mock_agent_service)");
    println!("       .build()?;");
    println!("   ```");

    println!("\n4. Custom overrides:");
    println!("   Override specific dependencies while keeping others:");
    println!("   ```rust");
    println!("   let custom_storage = Arc::new(MyCustomStorage::new());");
    println!("   let deps = AppBuilder::new()");
    println!("       .with_production_deps().await?");
    println!("       .with_storage(custom_storage)  // Override just storage");
    println!("       .build()?;");
    println!("   ```");

    println!("\n=== Example Complete ===");

    Ok(())
}
