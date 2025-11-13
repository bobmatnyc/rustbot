// Comprehensive tests for RustbotApi
// Tests all API functionality without requiring UI

use rustbot::api::{RustbotApi, RustbotApiBuilder};
use rustbot::agent::AgentConfig;
use rustbot::events::{Event, EventBus, EventKind, AgentStatus};
use rustbot::llm::{create_adapter, AdapterType, LlmAdapter, Message as LlmMessage};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create a test API instance with mock LLM adapter
fn create_test_api() -> RustbotApi {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| "test-key".to_string());

    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .max_history_size(10)
        .system_instructions("You are a test assistant.".to_string())
        .build()
        .expect("Failed to build test API")
}

#[test]
fn test_api_creation() {
    let api = create_test_api();

    // Verify initial state
    assert_eq!(api.active_agent(), "assistant");
    assert!(api.list_agents().contains(&"assistant".to_string()));
}

#[test]
fn test_agent_registration() {
    let mut api = create_test_api();

    // Create a custom agent config
    let mut custom_config = AgentConfig::default_assistant();
    custom_config.id = "researcher".to_string();
    custom_config.name = "Research Assistant".to_string();
    custom_config.instructions = "You are a research specialist.".to_string();

    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(Runtime::new().unwrap());
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, "test-key".to_string()));

    let agent = rustbot::agent::Agent::new(
        custom_config,
        llm_adapter,
        event_bus,
        runtime,
        "System instructions".to_string(),
    );

    api.register_agent(agent);

    // Verify agent was registered
    let agents = api.list_agents();
    assert_eq!(agents.len(), 2); // assistant + researcher
    assert!(agents.contains(&"researcher".to_string()));
}

#[test]
fn test_agent_switching() {
    let mut api = create_test_api();

    // Add a second agent
    let mut custom_config = AgentConfig::default_assistant();
    custom_config.id = "coder".to_string();
    custom_config.name = "Coding Assistant".to_string();

    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(Runtime::new().unwrap());
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, "test-key".to_string()));

    let agent = rustbot::agent::Agent::new(
        custom_config,
        llm_adapter,
        event_bus,
        runtime,
        String::new(),
    );

    api.register_agent(agent);

    // Test switching
    assert_eq!(api.active_agent(), "assistant");

    api.switch_agent("coder").expect("Failed to switch agent");
    assert_eq!(api.active_agent(), "coder");

    // Test invalid agent
    let result = api.switch_agent("invalid");
    assert!(result.is_err());
    assert_eq!(api.active_agent(), "coder"); // Should remain on coder
}

#[test]
fn test_history_management() {
    let mut api = create_test_api();

    // Initially empty
    assert_eq!(api.get_history().len(), 0);

    // Note: We can't easily test send_message_blocking without a real API key
    // and making actual LLM calls, so we focus on testing the history structure

    // Clear history (should be no-op when empty)
    api.clear_history();
    assert_eq!(api.get_history().len(), 0);
}

#[test]
fn test_agent_status() {
    let api = create_test_api();

    // Check status of active agent
    let status = api.current_agent_status();
    assert!(status.is_some());

    // Status should be Idle initially
    match status {
        Some(AgentStatus::Idle) => {}, // Expected
        _ => panic!("Expected agent to be Idle initially"),
    }
}

#[test]
fn test_event_subscription() {
    let api = create_test_api();

    // Subscribe to events
    let mut event_rx = api.subscribe_events();

    // Publish a test event
    let test_event = Event::new(
        "test_source".to_string(),
        "test_target".to_string(),
        EventKind::AgentStatusChange {
            agent_id: "assistant".to_string(),
            status: AgentStatus::Idle,
        },
    );

    api.publish_event(test_event.clone())
        .expect("Failed to publish event");

    // Try to receive the event (non-blocking)
    let runtime = Runtime::new().unwrap();
    let received = runtime.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            event_rx.recv()
        ).await
    });

    // Should receive the event we published
    assert!(received.is_ok());
}

#[test]
fn test_builder_pattern() {
    let api_key = "test-key".to_string();
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Test builder with various configurations
    let api = RustbotApiBuilder::new()
        .llm_adapter(Arc::clone(&llm_adapter))
        .max_history_size(50)
        .system_instructions("Custom instructions".to_string())
        .build();

    assert!(api.is_ok());

    // Test builder with custom agent
    let custom_config = AgentConfig {
        id: "custom".to_string(),
        name: "Custom Agent".to_string(),
        instructions: "Custom agent instructions".to_string(),
        personality: Some("Professional".to_string()),
        model: "anthropic/claude-sonnet-4.5".to_string(),
        enabled: true,
        is_primary: false,
        web_search_enabled: false,
    };

    let api2 = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .add_agent(custom_config)
        .build();

    assert!(api2.is_ok());
}

#[test]
fn test_builder_requires_llm_adapter() {
    // Builder should fail if LLM adapter is not provided
    let result = RustbotApiBuilder::new()
        .max_history_size(20)
        .build();

    assert!(result.is_err());
}

#[test]
fn test_max_history_size() {
    let api_key = "test-key".to_string();
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .max_history_size(3) // Very small history
        .build()
        .expect("Failed to build API");

    // History should start empty
    assert_eq!(api.get_history().len(), 0);

    // Note: Testing actual message history trimming requires sending messages
    // which requires a real API key and making LLM calls
    // This is more of an integration test than a unit test
}

#[test]
fn test_event_bus_integration() {
    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(Runtime::new().unwrap());
    let api_key = "test-key".to_string();
    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let api = RustbotApiBuilder::new()
        .event_bus(Arc::clone(&event_bus))
        .runtime(Arc::clone(&runtime))
        .llm_adapter(llm_adapter)
        .build()
        .expect("Failed to build API");

    // Verify API can publish events
    let test_event = Event::new(
        "api_test".to_string(),
        "broadcast".to_string(),
        EventKind::AgentStatusChange {
            agent_id: "test".to_string(),
            status: AgentStatus::Idle,
        },
    );

    let result = api.publish_event(test_event);
    assert!(result.is_ok());
}

/// Integration test: Verify message sending works end-to-end
/// This test requires OPENROUTER_API_KEY to be set and will make a real API call
#[test]
#[ignore] // Ignore by default since it requires API key and makes real calls
fn test_send_message_integration() {
    // Load .env file
    dotenvy::dotenv().ok();

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set for integration tests");

    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .system_instructions("You are a helpful assistant. Be very brief.".to_string())
        .build()
        .expect("Failed to build API");

    // Send a simple message
    let response = api.send_message_blocking("Say 'Hello' in one word")
        .expect("Failed to send message");

    // Verify we got a response
    assert!(!response.is_empty());
    println!("Response: {}", response);

    // Verify message is in history
    let history = api.get_history();
    assert_eq!(history.len(), 2); // User message + assistant response
    assert_eq!(history[0].role, "user");
    assert_eq!(history[1].role, "assistant");
}

/// Integration test: Verify streaming works end-to-end
#[test]
#[ignore] // Ignore by default since it requires API key and makes real calls
fn test_streaming_integration() {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set for integration tests");

    let llm_adapter: Arc<dyn LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .build()
        .expect("Failed to build API");

    let runtime = Runtime::new().unwrap();

    // Send message with streaming
    let mut result_rx = api.send_message("Count to 3")
        .expect("Failed to send message");

    // Collect streaming chunks
    let chunks = runtime.block_on(async {
        let mut collected = Vec::new();

        if let Some(Ok(mut stream_rx)) = result_rx.recv().await {
            while let Some(chunk) = stream_rx.recv().await {
                collected.push(chunk);
            }
        }

        collected
    });

    // Verify we received chunks
    assert!(!chunks.is_empty());

    let full_response: String = chunks.concat();
    println!("Streamed response: {}", full_response);
    assert!(!full_response.is_empty());
}
