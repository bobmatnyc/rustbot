use rustbot::agent;
use rustbot::api::RustbotApiBuilder;
use rustbot::llm::{create_adapter, AdapterType, LlmAdapter};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (simple format without env_filter)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("\n==================== TOOL CALLING TEST ====================\n");

    // Get API key
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set");

    // Create LLM adapter
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Load agents from JSON presets
    let agent_loader = agent::AgentLoader::new();
    let agent_configs = agent_loader.load_all()
        .expect("Failed to load agents from presets");

    println!("Loaded {} agent(s)", agent_configs.len());
    for config in &agent_configs {
        println!("  - {}", config.name);
    }

    // Build API instance with loaded agents
    let mut builder = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .max_history_size(10)
        .system_instructions("You are a helpful AI assistant with access to web search.".to_string());

    for config in agent_configs {
        builder = builder.add_agent(config);
    }

    let mut api = builder.build()?;

    println!("API initialized successfully");
    println!("Active agent: {}", api.active_agent());
    println!("Available agents: {:?}", api.list_agents());
    println!("\n-----------------------------------------------------------\n");

    // Test message that requires tool use
    let test_message = "What's the weather in New York?";
    println!("Sending message: {}", test_message);
    println!("\n-----------------------------------------------------------\n");

    // Send message
    let mut stream_rx = api.send_message(test_message).await?;

    // Collect response
    let mut full_response = String::new();
    println!("Response:");
    while let Some(chunk) = stream_rx.recv().await {
        print!("{}", chunk);
        full_response.push_str(&chunk);
    }

    println!("\n\n==================== TEST COMPLETE ====================");
    println!("Response length: {} chars", full_response.len());
    println!("========================================================\n");

    Ok(())
}
