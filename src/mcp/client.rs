//! MCP Client
//!
//! Design Decision: Transport-agnostic high-level MCP client
//!
//! Rationale: The MCP client implements the protocol-level operations (initialize,
//! tools/list, tools/call) independent of the transport mechanism (stdio, HTTP).
//! This separation allows the same client code to work with any transport
//! implementation via the McpTransport trait.
//!
//! Trade-offs:
//! - Abstraction vs Performance: Trait indirection adds minimal overhead (~1-2ns)
//! - Stateful vs Stateless: Track initialization state for safety
//! - Error Handling: Fail fast on protocol violations
//!
//! Alternatives Considered:
//! 1. Separate clients per transport: Rejected - duplicates protocol logic
//! 2. Stateless client: Rejected - need to enforce initialization handshake
//! 3. Callback-based API: Rejected - async/await is more ergonomic
//!
//! Performance Characteristics:
//! - initialize: One-time cost (~5-50ms depending on server)
//! - list_tools: Cached by server, fast (~1-10ms)
//! - call_tool: Depends on tool implementation (1ms to seconds)
//!
//! Protocol Flow:
//! 1. initialize: Client → Server (capabilities exchange)
//! 2. initialized notification: Client → Server (confirm ready)
//! 3. tools/list: Client → Server (discover available tools)
//! 4. tools/call: Client → Server (invoke specific tool)
//!
//! Extension Points:
//! - Add resources/list and resources/read
//! - Add prompts/list and prompts/get
//! - Add sampling support for server-initiated LLM calls
//! - Add notification handling (tools/list_changed, etc.)

use crate::mcp::error::{McpError, Result};
use crate::mcp::protocol::*;
use crate::mcp::transport::{JsonRpcRequest, McpTransport, RequestId};

/// High-level MCP client
///
/// Implements MCP protocol operations on top of any transport.
/// Handles the initialization handshake and enforces protocol rules.
///
/// Type Parameter:
/// - T: Transport implementation (StdioTransport, HttpTransport, etc.)
///
/// State Machine:
/// - Created → initialize() → Initialized
/// - Initialized → list_tools(), call_tool()
///
/// Thread Safety:
/// - Not thread-safe - intended for single-owner use
/// - Clone transport if needed for concurrent access
///
/// Example:
/// ```rust,ignore
/// // Create transport
/// let mut transport = StdioTransport::new(config);
/// transport.start().await?;
///
/// // Create client
/// let mut client = McpClient::new(transport);
///
/// // Initialize connection
/// let init_result = client.initialize().await?;
/// println!("Connected to: {}", init_result.server_info.name);
///
/// // Discover tools
/// let tools = client.list_tools().await?;
/// for tool in tools {
///     println!("- {}: {}", tool.name, tool.description.unwrap_or_default());
/// }
///
/// // Call a tool
/// let result = client.call_tool(
///     "read_file".to_string(),
///     Some(serde_json::json!({"path": "/etc/hosts"}))
/// ).await?;
/// println!("Result: {}", result.content[0].text);
/// ```
pub struct McpClient<T: McpTransport> {
    /// Underlying transport (stdio, HTTP, etc.)
    transport: T,

    /// Initialization state
    ///
    /// false: Must call initialize() before other operations
    /// true: Ready for tools/list, tools/call, etc.
    initialized: bool,

    /// Server capabilities from initialization
    ///
    /// Used to check if server supports optional features
    /// (e.g., tools, resources, prompts)
    server_capabilities: Option<ServerCapabilities>,

    /// Request ID counter for generating sequential IDs
    next_id: u64,
}

impl<T: McpTransport> McpClient<T> {
    /// Create a new MCP client with the given transport
    ///
    /// Transport should already be started/connected.
    ///
    /// Example:
    /// ```rust,ignore
    /// let transport = StdioTransport::new(config);
    /// let client = McpClient::new(transport);
    /// ```
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            initialized: false,
            server_capabilities: None,
            next_id: 0,
        }
    }

    /// Initialize the MCP connection
    ///
    /// This is the required first operation in MCP protocol.
    /// Exchanges capabilities between client and server.
    ///
    /// Protocol Flow:
    /// 1. Client sends initialize request with client info and capabilities
    /// 2. Server responds with server info and capabilities
    /// 3. Client sends initialized notification (confirms ready)
    ///
    /// Error Conditions:
    /// - Already initialized: Returns Ok (idempotent)
    /// - Transport not connected: Returns Transport error
    /// - Protocol version mismatch: Returns Protocol error
    /// - Server error response: Returns Protocol error
    ///
    /// Side Effects:
    /// - Sets initialized = true
    /// - Stores server_capabilities
    /// - Sends initialized notification
    ///
    /// Example:
    /// ```rust,ignore
    /// let result = client.initialize().await?;
    /// println!("Server: {} v{}", result.server_info.name, result.server_info.version);
    /// println!("Supports tools: {}", result.capabilities.tools.is_some());
    /// ```
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        // Allow re-initialization (idempotent)
        if self.initialized {
            if let Some(caps) = &self.server_capabilities {
                // Return cached result
                return Ok(InitializeResult {
                    protocol_version: crate::mcp::MCP_PROTOCOL_VERSION.to_string(),
                    capabilities: caps.clone(),
                    server_info: ServerInfo {
                        name: "cached".to_string(),
                        version: "0.0.0".to_string(),
                    },
                });
            }
        }

        // Build initialize request
        let params = InitializeParams {
            protocol_version: crate::mcp::MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities {
                sampling: None,
                experimental: None,
            },
            client_info: ClientInfo {
                name: crate::mcp::MCP_CLIENT_NAME.to_string(),
                version: crate::mcp::mcp_client_version(),
            },
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(self.get_next_id()),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        // Send initialize request
        let response = self.transport.send_request(request).await?;

        // Check for error
        if let Some(error) = response.error {
            return Err(McpError::Protocol(format!(
                "Initialize failed: {} (code: {})",
                error.message, error.code
            )));
        }

        // Parse result
        let result: InitializeResult = serde_json::from_value(
            response
                .result
                .ok_or_else(|| McpError::Protocol("No result in initialize response".into()))?,
        )?;

        // Validate protocol version
        if result.protocol_version != crate::mcp::MCP_PROTOCOL_VERSION {
            eprintln!(
                "Warning: Protocol version mismatch. Client: {}, Server: {}",
                crate::mcp::MCP_PROTOCOL_VERSION,
                result.protocol_version
            );
        }

        // Store server capabilities
        self.server_capabilities = Some(result.capabilities.clone());
        self.initialized = true;

        // Send initialized notification
        self.send_initialized().await?;

        Ok(result)
    }

    /// Send initialized notification to server
    ///
    /// This notification tells the server that the client has processed
    /// the initialize response and is ready for operation.
    ///
    /// Note: This is a notification (no response expected), but we still
    /// wait for acknowledgment to ensure the message was delivered.
    async fn send_initialized(&mut self) -> Result<()> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(self.get_next_id()),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        // Send notification (response is typically empty acknowledgment)
        let _ = self.transport.send_request(request).await?;
        Ok(())
    }

    /// List available tools from server
    ///
    /// Queries the server for all tools it provides. Tools can then be
    /// invoked using call_tool().
    ///
    /// Preconditions:
    /// - Must call initialize() first
    /// - Server must support tools (check capabilities.tools)
    ///
    /// Error Conditions:
    /// - Not initialized: Returns Protocol error
    /// - Server error: Returns Protocol error with server message
    ///
    /// Performance:
    /// - Typically fast (~1-10ms) as servers cache tool lists
    /// - Can be slow on first call if server does discovery
    ///
    /// Example:
    /// ```rust,ignore
    /// let tools = client.list_tools().await?;
    /// for tool in tools {
    ///     println!("{}: {}", tool.name, tool.description.unwrap_or_default());
    /// }
    /// ```
    pub async fn list_tools(&mut self) -> Result<Vec<McpToolDefinition>> {
        if !self.initialized {
            return Err(McpError::Protocol(
                "Client not initialized - call initialize() first".into(),
            ));
        }

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(self.get_next_id()),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::Protocol(format!(
                "tools/list failed: {} (code: {})",
                error.message, error.code
            )));
        }

        let result: ToolListResult = serde_json::from_value(
            response
                .result
                .ok_or_else(|| McpError::Protocol("No result in tools/list response".into()))?,
        )?;

        Ok(result.tools)
    }

    /// Call a tool with arguments
    ///
    /// Invokes a tool on the server and returns its result.
    ///
    /// Preconditions:
    /// - Must call initialize() first
    /// - Tool must exist (from list_tools())
    /// - Arguments must match tool's inputSchema
    ///
    /// Error Conditions:
    /// - Not initialized: Returns Protocol error
    /// - Tool not found: Server returns error (code -32601)
    /// - Invalid arguments: Server returns error (code -32602)
    /// - Tool execution error: Returns result with is_error=true
    ///
    /// Tool Error Handling:
    /// - Protocol errors: Returned as Err(McpError::Protocol(...))
    /// - Tool execution errors: Returned as Ok(ToolCallResult) with is_error=true
    ///
    /// Example:
    /// ```rust,ignore
    /// let result = client.call_tool(
    ///     "read_file".to_string(),
    ///     Some(serde_json::json!({"path": "/etc/hosts"}))
    /// ).await?;
    ///
    /// if result.is_error == Some(true) {
    ///     eprintln!("Tool error: {}", result.content[0].text);
    /// } else {
    ///     println!("Result: {}", result.content[0].text);
    /// }
    /// ```
    pub async fn call_tool(
        &mut self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult> {
        if !self.initialized {
            return Err(McpError::Protocol(
                "Client not initialized - call initialize() first".into(),
            ));
        }

        let params = ToolCallParams { name, arguments };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(self.get_next_id()),
            method: "tools/call".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::Protocol(format!(
                "tools/call failed: {} (code: {})",
                error.message, error.code
            )));
        }

        let result: ToolCallResult = serde_json::from_value(
            response
                .result
                .ok_or_else(|| McpError::Protocol("No result in tools/call response".into()))?,
        )?;

        Ok(result)
    }

    /// Check if client is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get server capabilities (available after initialize())
    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    /// Get mutable reference to transport
    ///
    /// Useful for accessing transport-specific features
    /// (e.g., checking connection status, closing)
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Generate next request ID
    fn get_next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::transport::{JsonRpcResponse, RequestId};
    use async_trait::async_trait;

    // Mock transport for testing
    struct MockTransport {
        responses: Vec<JsonRpcResponse>,
        current: usize,
    }

    impl MockTransport {
        fn new(responses: Vec<JsonRpcResponse>) -> Self {
            Self {
                responses,
                current: 0,
            }
        }
    }

    #[async_trait]
    impl McpTransport for MockTransport {
        async fn send_request(&mut self, _request: JsonRpcRequest) -> Result<JsonRpcResponse> {
            if self.current >= self.responses.len() {
                return Err(McpError::Transport("No more responses".into()));
            }
            let response = self.responses[self.current].clone();
            self.current += 1;
            Ok(response)
        }

        fn is_connected(&self) -> bool {
            true
        }

        async fn close(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_client_requires_initialization() {
        let transport = MockTransport::new(vec![]);
        let mut client = McpClient::new(transport);

        assert!(!client.is_initialized());

        let result = client.list_tools().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
    }

    #[tokio::test]
    async fn test_successful_initialization() {
        let init_response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            })),
            error: None,
        };

        let initialized_response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(2),
            result: Some(serde_json::json!({})),
            error: None,
        };

        let transport = MockTransport::new(vec![init_response, initialized_response]);
        let mut client = McpClient::new(transport);

        let result = client.initialize().await.unwrap();
        assert_eq!(result.server_info.name, "test-server");
        assert!(client.is_initialized());
        assert!(client.server_capabilities().is_some());
    }
}
