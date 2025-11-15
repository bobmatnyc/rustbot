use rustbot::agent;
use rustbot::api::RustbotApiBuilder;
use rustbot::events::EventBus;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🧪 Testing GUI API Configuration\n");

    // Load environment
    dotenvy::from_filename(".env.local").ok();
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set");

    // Create components exactly like the GUI does
    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(Runtime::new().unwrap());
    let llm_adapter = Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Load agents exactly like GUI
    let agent_loader = agent::AgentLoader::new();
    let agent_configs = agent_loader.load_all()?;

    println!("📋 Loaded {} agents", agent_configs.len());
    for config in &agent_configs {
        println!("   - {} (primary: {}, enabled: {})", 
                 config.id, config.is_primary, config.enabled);
    }

    // Build API exactly like GUI does
    let mut api_builder = RustbotApiBuilder::new()
        .event_bus(Arc::clone(&event_bus))
        .runtime(Arc::clone(&runtime))
        .llm_adapter(Arc::clone(&llm_adapter))
        .max_history_size(20)
        .system_instructions("You are a helpful AI assistant.".to_string());

    for config in agent_configs {
        api_builder = api_builder.add_agent(config);
    }

    let mut api = api_builder.build()?;

    println!("\n🚀 API built, sending test message...\n");

    // Send message like GUI does
    let mut rx = api.send_message("What are today's top news stories?").await?;

    // Collect response
    let mut response = String::new();
    while let Some(chunk) = rx.recv().await {
        print!("{}", chunk);
        response.push_str(&chunk);
    }

    println!("\n\n✅ Test complete");
    println!("Response length: {} chars", response.len());

    Ok(())
}
