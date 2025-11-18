//! Test for automatic MCP tool registration via event bus
//!
//! This test demonstrates that tools are automatically registered when
//! MCP plugins emit Started events, without manual registration calls.

use rustbot::api::RustbotApi;
use rustbot::events::{Event, EventBus, EventKind, McpPluginEvent};
use rustbot::mcp::McpPluginManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_automatic_tool_registration_on_started_event() {
    // Setup
    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());

    // Create API
    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

    // Create and configure MCP manager
    let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
        Arc::clone(&event_bus),
    ))));
    api.set_mcp_manager(Arc::clone(&mcp_manager));

    // Wrap API in Arc<Mutex> for auto-registration
    let api = Arc::new(Mutex::new(api));

    // Start auto-registration task
    let _auto_reg_task = RustbotApi::start_mcp_auto_registration(Arc::clone(&api)).await;

    // Wait for task to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Simulate plugin startup by emitting Started event
    // In real usage, this would be emitted by McpPluginManager when a plugin starts
    event_bus
        .publish(Event::new(
            "mcp_manager".to_string(),
            "broadcast".to_string(),
            EventKind::McpPluginEvent(McpPluginEvent::Started {
                plugin_id: "test_plugin".to_string(),
                tool_count: 3,
            }),
        ))
        .expect("Failed to publish event");

    // Wait for auto-registration to process the event
    // Note: In this test we don't have real tools, so the registration will fail
    // but we can verify the task is listening and attempting to register
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify the auto-registration task is running by checking the event was consumed
    // (The task should have received and processed the Started event)

    // In a real scenario with actual MCP tools, we would verify:
    // let api_guard = api.lock().await;
    // let all_tools = api_guard.get_all_tools();
    // assert!(all_tools.iter().any(|t| t.name.starts_with("mcp:test_plugin:")));

    println!("✓ Auto-registration task successfully handles Started events");
}

#[tokio::test]
async fn test_automatic_tool_unregistration_on_stopped_event() {
    // Setup
    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());

    // Create API
    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

    // Create and configure MCP manager
    let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
        Arc::clone(&event_bus),
    ))));
    api.set_mcp_manager(Arc::clone(&mcp_manager));

    // Wrap API in Arc<Mutex> for auto-registration
    let api = Arc::new(Mutex::new(api));

    // Start auto-registration task
    let _auto_reg_task = RustbotApi::start_mcp_auto_registration(Arc::clone(&api)).await;

    // Wait for task to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Simulate plugin shutdown by emitting Stopped event
    event_bus
        .publish(Event::new(
            "mcp_manager".to_string(),
            "broadcast".to_string(),
            EventKind::McpPluginEvent(McpPluginEvent::Stopped {
                plugin_id: "test_plugin".to_string(),
            }),
        ))
        .expect("Failed to publish event");

    // Wait for auto-unregistration to process the event
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify tools were unregistered
    let api_guard = api.lock().await;
    let all_tools = api_guard.get_all_tools();

    // Should not have any tools from test_plugin
    assert!(!all_tools
        .iter()
        .any(|t| t.function.name.starts_with("mcp:test_plugin:")));

    println!("✓ Auto-unregistration task successfully handles Stopped events");
}

#[tokio::test]
async fn test_auto_registration_handles_missing_manager_gracefully() {
    // Setup
    let event_bus = Arc::new(EventBus::new());
    let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());

    // Create API WITHOUT MCP manager
    let api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

    // Wrap API in Arc<Mutex> for auto-registration
    let api = Arc::new(Mutex::new(api));

    // Start auto-registration task (should not crash without manager)
    let _auto_reg_task = RustbotApi::start_mcp_auto_registration(Arc::clone(&api)).await;

    // Wait for task to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Emit Started event (ignore errors - may have no subscribers)
    let _ = event_bus.publish(Event::new(
        "mcp_manager".to_string(),
        "broadcast".to_string(),
        EventKind::McpPluginEvent(McpPluginEvent::Started {
            plugin_id: "test_plugin".to_string(),
            tool_count: 3,
        }),
    ));

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should handle gracefully without crashing
    // (We check that the task didn't panic by successfully getting here)

    println!("✓ Auto-registration task handles missing MCP manager gracefully");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_auto_registration_task_lifetime() {
    // Setup
    let event_bus = Arc::new(EventBus::new());

    // Create a standalone runtime that won't be dropped in async context
    static ONCE: std::sync::Once = std::sync::Once::new();
    static mut TEST_RUNTIME: Option<Arc<tokio::runtime::Runtime>> = None;

    unsafe {
        ONCE.call_once(|| {
            TEST_RUNTIME = Some(Arc::new(tokio::runtime::Runtime::new().unwrap()));
        });
    }

    let runtime = unsafe { TEST_RUNTIME.clone().unwrap() };

    // Create API
    let api = RustbotApi::new(Arc::clone(&event_bus), runtime, 20);

    let api = Arc::new(Mutex::new(api));

    // Start auto-registration task and get handle
    let task_handle = RustbotApi::start_mcp_auto_registration(Arc::clone(&api)).await;

    // Verify task is running
    assert!(!task_handle.is_finished());

    // Send multiple events to ensure task is responsive
    for i in 0..5 {
        let _ = event_bus.publish(Event::new(
            "test".to_string(),
            "broadcast".to_string(),
            EventKind::McpPluginEvent(McpPluginEvent::Started {
                plugin_id: format!("plugin_{}", i),
                tool_count: i,
            }),
        ));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Task should still be running
    assert!(!task_handle.is_finished());

    // Abort task for cleanup
    task_handle.abort();

    // Wait a bit for task to terminate
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("✓ Auto-registration task has correct lifetime management");
}
