// Example demonstrating programmatic API usage of Rustbot
// This shows how to use Rustbot without the GUI

use rustbot::api::{RustbotApi, RustbotApiBuilder};
use rustbot::llm::{create_adapter, AdapterType};
use std::env;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get API key from environment
    let api_key = env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    // Create LLM adapter
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Build the API with configuration
    println!("🤖 Initializing Rustbot API...");
    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .max_history_size(20)
        .system_instructions("You are a helpful AI assistant.".to_string())
        .build()?;

    println!("✅ Rustbot API initialized");
    println!("📋 Available agents: {:?}", api.list_agents());
    println!("🎯 Active agent: {}", api.active_agent());
    println!();

    // Example 1: Send a message and wait for response (blocking)
    println!("Example 1: Blocking API call");
    println!("─────────────────────────────");
    let question = "What is Rust programming language?";
    println!("User: {}", question);
    print!("Assistant: ");

    match api.send_message_blocking(question) {
        Ok(response) => {
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    println!();

    // Example 2: Send message with streaming (non-blocking)
    println!("Example 2: Streaming API call");
    println!("─────────────────────────────");
    let question2 = "List 3 benefits of Rust in bullet points.";
    println!("User: {}", question2);
    print!("Assistant: ");

    // Create runtime for async operations
    let runtime = tokio::runtime::Runtime::new()?;

    match api.send_message(question2) {
        Ok(mut result_rx) => {
            // Wait for the stream channel
            runtime.block_on(async {
                if let Some(Ok(mut stream_rx)) = result_rx.recv().await {
                    // Receive and print chunks as they arrive
                    while let Some(chunk) = stream_rx.recv().await {
                        print!("{}", chunk);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                    println!();
                }
            });
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    println!();

    // Example 3: View conversation history
    println!("Example 3: Conversation history");
    println!("─────────────────────────────");
    let history = api.get_history();
    println!("Total messages in history: {}", history.len());
    for (i, msg) in history.iter().enumerate() {
        println!("{}: {} ({})", i + 1, msg.role, msg.content.len());
    }
    println!();

    // Example 4: Clear history
    println!("Example 4: Clear history");
    println!("─────────────────────────────");
    api.clear_history();
    println!("History cleared. New message count: {}", api.get_history().len());
    println!();

    println!("✅ Demo completed!");

    Ok(())
}
