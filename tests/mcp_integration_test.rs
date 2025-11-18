//! Integration test for MCP plugin and Rustbot API tool integration
//!
//! This test validates the full flow:
//! 1. MCP plugin starts and discovers tools
//! 2. Tools are registered with Rustbot API
//! 3. Agent can discover and execute MCP tools
//! 4. Plugin stops and tools are unregistered

use rustbot::api::RustbotApi;
use rustbot::events::EventBus;
use rustbot::mcp::{McpPluginManager, McpToolDefinition};
use std::sync::{Arc, Once};
use tokio::sync::Mutex;

// Shared runtime for async tests to avoid drop issues
static INIT: Once = Once::new();
static mut TEST_RUNTIME: Option<Arc<tokio::runtime::Runtime>> = None;

fn get_test_runtime() -> Arc<tokio::runtime::Runtime> {
    unsafe {
        INIT.call_once(|| {
            TEST_RUNTIME = Some(Arc::new(tokio::runtime::Runtime::new().unwrap()));
        });
        TEST_RUNTIME.clone().unwrap()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mcp_tool_lifecycle() {
    // Create core components
    let event_bus = Arc::new(EventBus::new());
    let runtime = get_test_runtime();

    // Create MCP plugin manager
    let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
        Arc::clone(&event_bus),
    ))));

    // Create Rustbot API
    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);
    api.set_mcp_manager(Arc::clone(&mcp_manager));

    // Simulate plugin startup with tool discovery
    let mock_tools = vec![
        McpToolDefinition {
            name: "read_file".to_string(),
            description: Some("Read a file from disk".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }),
        },
        McpToolDefinition {
            name: "write_file".to_string(),
            description: Some("Write content to a file".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        },
    ];

    // Register tools (simulating plugin start hook)
    for tool in &mock_tools {
        api.register_mcp_tool(tool.clone(), "filesystem".to_string())
            .await
            .unwrap();
    }

    // Verify tools are registered and discoverable
    let available_tools = api.get_all_tools();
    assert_eq!(available_tools.len(), 2, "Should have 2 tools registered");

    // Verify tool naming convention
    let tool_names: Vec<String> = available_tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert!(
        tool_names.contains(&"mcp:filesystem:read_file".to_string()),
        "Should have read_file tool with proper naming"
    );
    assert!(
        tool_names.contains(&"mcp:filesystem:write_file".to_string()),
        "Should have write_file tool with proper naming"
    );

    // Verify tool metadata
    let read_file_tool = available_tools
        .iter()
        .find(|t| t.function.name == "mcp:filesystem:read_file")
        .expect("read_file tool should exist");
    assert_eq!(
        read_file_tool.function.description, "Read a file from disk",
        "Tool description should match"
    );
    assert!(
        read_file_tool.function.parameters.properties.is_object(),
        "Tool should have parameters schema"
    );

    // Simulate plugin stop
    api.unregister_mcp_tools("filesystem").await.unwrap();

    // Verify tools are removed
    let available_tools_after_stop = api.get_all_tools();
    assert_eq!(
        available_tools_after_stop.len(),
        0,
        "All tools should be unregistered after plugin stop"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mcp_tool_execution_routing() {
    use rustbot::tool_executor::ToolExecutor;

    // Create core components
    let event_bus = Arc::new(EventBus::new());
    let runtime = get_test_runtime();

    // Create MCP plugin manager
    let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
        Arc::clone(&event_bus),
    ))));

    // Create Rustbot API
    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);
    api.set_mcp_manager(Arc::clone(&mcp_manager));

    // Register a test tool
    let tool = McpToolDefinition {
        name: "read_file".to_string(),
        description: Some("Read a file".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    };

    api.register_mcp_tool(tool, "filesystem".to_string())
        .await
        .unwrap();

    // Test tool name parsing and routing logic
    let tool_name = "mcp:filesystem:read_file";
    let arguments = r#"{"path": "/etc/hosts"}"#;

    // Note: execute_tool will fail because MCP manager isn't actually running a plugin
    // But we can verify the routing logic by checking the error message
    let result = api.execute_tool(tool_name, arguments).await;

    // Should fail because plugin not actually running, but error should indicate
    // it was routed to MCP manager (not "agent not found")
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        !error_msg.contains("Specialist agent"),
        "Should not route to agent - error: {}",
        error_msg
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_plugins_with_tools() {
    // Create core components
    let event_bus = Arc::new(EventBus::new());
    let runtime = get_test_runtime();

    let mcp_manager = Arc::new(Mutex::new(McpPluginManager::with_event_bus(Some(
        Arc::clone(&event_bus),
    ))));

    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);
    api.set_mcp_manager(Arc::clone(&mcp_manager));

    // Register tools from multiple plugins
    let filesystem_tool = McpToolDefinition {
        name: "read_file".to_string(),
        description: Some("Read file".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    };

    let web_tool = McpToolDefinition {
        name: "fetch".to_string(),
        description: Some("Fetch URL".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    };

    api.register_mcp_tool(filesystem_tool, "filesystem".to_string())
        .await
        .unwrap();
    api.register_mcp_tool(web_tool, "web".to_string())
        .await
        .unwrap();

    // Verify both tools registered
    assert_eq!(api.get_all_tools().len(), 2);

    // Unregister one plugin
    api.unregister_mcp_tools("filesystem").await.unwrap();

    // Verify only web plugin tools remain
    let remaining_tools = api.get_all_tools();
    assert_eq!(remaining_tools.len(), 1);
    assert_eq!(remaining_tools[0].function.name, "mcp:web:fetch");

    // Unregister second plugin
    api.unregister_mcp_tools("web").await.unwrap();
    assert_eq!(api.get_all_tools().len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_name_collisions() {
    // Test that different plugins can have tools with same name
    let event_bus = Arc::new(EventBus::new());
    let runtime = get_test_runtime();

    let mut api = RustbotApi::new(Arc::clone(&event_bus), Arc::clone(&runtime), 20);

    // Create same-named tool from different plugins
    let tool = McpToolDefinition {
        name: "list".to_string(),
        description: Some("List items".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    };

    // Register from filesystem plugin
    api.register_mcp_tool(tool.clone(), "filesystem".to_string())
        .await
        .unwrap();

    // Register same tool name from database plugin (should succeed due to namespacing)
    api.register_mcp_tool(tool, "database".to_string())
        .await
        .unwrap();

    // Both tools should exist with different namespaced names
    let tools = api.get_all_tools();
    assert_eq!(tools.len(), 2);

    let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
    assert!(tool_names.contains(&"mcp:filesystem:list".to_string()));
    assert!(tool_names.contains(&"mcp:database:list".to_string()));
}
