/// Test to verify web_search tool calling behavior with news queries
/// This test will show us whether the LLM actually calls the web_search tool or responds directly

use rustbot::agent::config::AgentConfig;
use rustbot::agent::loader::AgentLoader;
use rustbot::api::RustbotApi;
use rustbot::events::EventBus;
use rustbot::llm::adapters::{create_adapter, AdapterType};
use rustbot::Result;
use std::sync::Arc;

#[tokio::test]
async fn test_news_query_tool_calling() -> Result<()> {
    // Initialize logging with INFO level to see our enhanced logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .try_init()
        .ok();

    println!("\n🧪 Testing News Query Tool Calling");
    println!("====================================\n");

    // Load environment
    dotenvy::from_filename(".env.local").ok();
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set");

    // Create LLM adapter
    let llm_adapter = Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Load agent configurations from actual preset files
    let loader = AgentLoader::new();
    let agent_configs = loader.load_presets("agents/presets")?;

    println!("📋 Loaded {} agent configurations:", agent_configs.len());
    for config in &agent_configs {
        println!("   - {} (primary: {}, web_search: {})",
                 config.id,
                 config.is_primary,
                 config.web_search_enabled);
    }
    println!();

    // Create event bus
    let event_bus = Arc::new(EventBus::new());

    // Create API with loaded agents
    let mut api = RustbotApi::new(
        llm_adapter,
        agent_configs,
        event_bus,
        100,
        String::new(),
    )?;

    // Test with a clear news query
    let test_message = "What are today's top news stories?";

    println!("📤 Sending test query: \"{}\"", test_message);
    println!("🔍 Watch for these log patterns:");
    println!("   🔧 [LLM] Sending X tools to API");
    println!("   🎯 [LLM] tool_choice: ...");
    println!("   📞 [LLM] Response contains X tool call(s) OR NO tool calls");
    println!();
    println!("==================== API CALL START ====================\n");

    // Send message
    let mut stream_rx = api.send_message(test_message).await?;

    // Collect response
    let mut full_response = String::new();
    let mut chunk_count = 0;

    while let Some(chunk_result) = stream_rx.recv().await {
        match chunk_result {
            Ok(mut chunk_rx) => {
                while let Some(chunk) = chunk_rx.recv().await {
                    print!("{}", chunk);
                    full_response.push_str(&chunk);
                    chunk_count += 1;
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error during streaming: {}", e);
                return Err(e);
            }
        }
    }

    println!("\n\n==================== API CALL COMPLETE ====================");
    println!("📊 Response Statistics:");
    println!("   - Total chunks: {}", chunk_count);
    println!("   - Total length: {} chars", full_response.len());
    println!("   - First 200 chars: {}",
             &full_response.chars().take(200).collect::<String>());
    println!("===========================================================\n");

    // Verify we got a response
    assert!(!full_response.is_empty(), "Response should not be empty");

    println!("✅ Test completed - check logs above for tool calling behavior");
    println!();
    println!("🔍 Key Diagnostic Points:");
    println!("   1. Did you see '🔧 [LLM] Sending 1 tools to API'?");
    println!("   2. What was the tool_choice value?");
    println!("   3. Did you see '📞 [LLM] Response contains 1 tool call(s)' or 'NO tool calls'?");
    println!();

    Ok(())
}
