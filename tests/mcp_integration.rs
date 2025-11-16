//! MCP Integration Tests
//!
//! These tests verify the end-to-end MCP plugin functionality.
//! Note: Actual MCP server processes are required for full integration testing.
//!
//! Test Organization:
//! - Unit tests: In individual module files (transport.rs, client.rs, etc.)
//! - Integration tests: Here - test complete workflows
//! - Manual tests: examples/mcp_demo.rs - for interactive testing

use rustbot::mcp::{McpPluginManager, McpConfig};
use std::path::Path;

#[tokio::test]
async fn test_manager_lifecycle() {
    // Create a new plugin manager
    let manager = McpPluginManager::new();

    // Verify initial state
    assert_eq!(manager.plugin_count().await, 0);
}

#[tokio::test]
async fn test_config_loading() {
    // Note: This test requires a valid mcp_config.json file
    // In CI/CD, you would create a temporary config file

    let mut manager = McpPluginManager::new();

    // Try loading config (will fail if file doesn't exist, which is expected)
    let result = manager.load_config(Path::new("mcp_config.json")).await;

    // If config exists, it should load successfully
    // If not, we just verify the error handling works
    match result {
        Ok(_) => {
            println!("✓ Config loaded successfully");
            assert!(manager.plugin_count().await > 0);
        }
        Err(e) => {
            println!("⚠️  Config not found (expected): {}", e);
            assert!(e.to_string().contains("No such file") || e.to_string().contains("not found"));
        }
    }
}

#[tokio::test]
async fn test_plugin_start_requires_valid_config() {
    let mut manager = McpPluginManager::new();

    // Trying to start a plugin without config should fail
    let result = manager.start_plugin("filesystem").await;
    assert!(result.is_err());
}

// Note: The following test requires an actual MCP server to be available
// It's commented out by default to prevent CI failures
/*
#[tokio::test]
async fn test_full_plugin_lifecycle() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary config file
    let mut temp_file = NamedTempFile::new().unwrap();
    let config_json = r#"{
        "mcp_plugins": {
            "local_servers": [
                {
                    "id": "filesystem",
                    "name": "Filesystem Access",
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
                    "enabled": true,
                    "auto_restart": false,
                    "timeout": 60
                }
            ],
            "cloud_services": []
        }
    }"#;
    temp_file.write_all(config_json.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Create manager and load config
    let mut manager = McpPluginManager::new();
    manager.load_config(temp_file.path()).await.unwrap();

    // Start plugin
    manager.start_plugin("filesystem").await.unwrap();

    // Verify plugin is running
    let plugin = manager.get_plugin("filesystem").await.unwrap();
    assert_eq!(plugin.state, rustbot::mcp::PluginState::Running);
    assert!(plugin.tools.len() > 0);

    // Execute a tool
    let result = manager.execute_tool(
        "filesystem",
        "read_file",
        Some(serde_json::json!({"path": "/tmp/test.txt"}))
    ).await;

    // Result depends on whether /tmp/test.txt exists
    // Just verify we got a response (error or success)
    match result {
        Ok(text) => println!("Tool succeeded: {}", text),
        Err(e) => println!("Tool error (expected if file doesn't exist): {}", e),
    }

    // Stop plugin
    manager.stop_plugin("filesystem").await.unwrap();

    // Verify plugin is stopped
    let plugin = manager.get_plugin("filesystem").await.unwrap();
    assert_eq!(plugin.state, rustbot::mcp::PluginState::Stopped);
    assert_eq!(plugin.tools.len(), 0);
}
*/

#[test]
fn test_integration_tests_compile() {
    // This test just ensures the integration test file compiles
    // Real integration tests require MCP servers to be available
    assert!(true);
}
