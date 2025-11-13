use rustbot::api::{RustbotApi, RustbotApiBuilder};
use rustbot::llm::adapters::{create_adapter, AdapterType};
use rustbot::llm::LlmAdapter;
use rustbot::Result;
use std::sync::Arc;

fn create_test_api() -> RustbotApi {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| panic!("OPENROUTER_API_KEY must be set for this test"));

    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .max_history_size(10)
        .system_instructions("You are a helpful AI assistant with access to web search.".to_string())
        .build()
        .expect("Failed to build test API")
}

#[tokio::test]
async fn test_tool_calling_flow() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_test_writer()
        .try_init()
        .ok();

    // Create API instance
    let mut api = create_test_api();

    // Test message that requires tool use
    let test_message = "What's the weather in New York?";

    println!("\n==================== TEST START ====================");
    println!("Sending message: {}", test_message);
    println!("====================================================\n");

    // Send message and get streaming response
    let mut stream_rx = api.send_message(test_message).await?;

    // Collect response chunks
    let mut full_response = String::new();
    while let Some(chunk_result) = stream_rx.recv().await {
        match chunk_result {
            Ok(mut chunk_rx) => {
                while let Some(chunk) = chunk_rx.recv().await {
                    print!("{}", chunk);
                    full_response.push_str(&chunk);
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error: {}", e);
                return Err(e);
            }
        }
    }

    println!("\n\n==================== TEST COMPLETE ====================");
    println!("Full response length: {} chars", full_response.len());
    println!("=======================================================\n");

    // Verify we got some response
    assert!(!full_response.is_empty(), "Response should not be empty");

    Ok(())
}

#[tokio::test]
async fn test_simple_message_without_tools() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_test_writer()
        .try_init()
        .ok();

    // Create API instance
    let mut api = create_test_api();

    // Test message that doesn't require tools
    let test_message = "What is 2 + 2?";

    println!("\n==================== SIMPLE TEST START ====================");
    println!("Sending message: {}", test_message);
    println!("==========================================================\n");

    // Send message and get streaming response
    let mut stream_rx = api.send_message(test_message).await?;

    // Collect response chunks
    let mut full_response = String::new();
    while let Some(chunk_result) = stream_rx.recv().await {
        match chunk_result {
            Ok(mut chunk_rx) => {
                while let Some(chunk) = chunk_rx.recv().await {
                    print!("{}", chunk);
                    full_response.push_str(&chunk);
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error: {}", e);
                return Err(e);
            }
        }
    }

    println!("\n\n==================== SIMPLE TEST COMPLETE ====================");
    println!("Full response length: {} chars", full_response.len());
    println!("===============================================================\n");

    // Verify we got some response
    assert!(!full_response.is_empty(), "Response should not be empty");
    assert!(full_response.contains("4"), "Response should contain the answer");

    Ok(())
}
