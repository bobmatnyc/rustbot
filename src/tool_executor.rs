// Tool execution abstraction for agent delegation
// Allows agents to execute specialist tools without direct coupling

use anyhow::Result;
use async_trait::async_trait;

/// Trait for executing tool calls by delegating to specialist agents
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool call and return the result as a string
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool/agent to call (e.g., "web_search")
    /// * `arguments` - JSON arguments for the tool call
    ///
    /// # Returns
    /// * `Result<String>` - The tool execution result or error
    async fn execute_tool(&self, tool_name: &str, arguments: &str) -> Result<String>;
}
