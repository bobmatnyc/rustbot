//! MCP Transport Layer
//!
//! Design Decision: Abstract transport trait for multiple communication methods
//!
//! Rationale: MCP supports both stdio (local processes) and HTTP (cloud services).
//! An abstract McpTransport trait allows implementing both transports while keeping
//! the client code transport-agnostic. This follows dependency inversion principle.
//!
//! Trade-offs:
//! - Abstraction vs Simplicity: Trait adds indirection but enables multiple transports
//! - Async Trait: Uses async_trait for async support in trait methods
//! - Error Handling: Transport errors are wrapped in McpError for consistency
//!
//! Alternatives Considered:
//! 1. Enum-based transport: Rejected - harder to extend, loses compile-time checks
//! 2. Separate clients per transport: Rejected - duplicates protocol logic
//! 3. Sync trait: Rejected - all I/O must be async to avoid blocking UI
//!
//! Performance Characteristics:
//! - stdio transport: ~1-5ms latency for local process communication
//! - HTTP transport: ~50-200ms latency for network requests
//! - JSON-RPC overhead: <1ms for serialization/deserialization
//!
//! Extension Points:
//! - Add WebSocket transport for bidirectional streaming
//! - Add IPC transport for same-machine services
//! - Add metrics collection in transport implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::mcp::error::Result;

/// JSON-RPC 2.0 Request
///
/// Core message type for MCP communication. All MCP methods (initialize, tools/list,
/// tools/call, etc.) are sent as JSON-RPC 2.0 requests.
///
/// Example:
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "method": "tools/list",
///   "params": null
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request identifier for matching responses
    ///
    /// Can be a number (for sequential requests) or string (for correlation IDs)
    pub id: RequestId,

    /// Method name (e.g., "initialize", "tools/list", "tools/call")
    pub method: String,

    /// Optional method parameters
    ///
    /// Type varies by method - use serde_json::Value for flexibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
///
/// All MCP servers respond with JSON-RPC 2.0 responses. Either `result` or
/// `error` is present, never both.
///
/// Success Example:
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "result": { "tools": [...] }
/// }
/// ```
///
/// Error Example:
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 1,
///   "error": {
///     "code": -32601,
///     "message": "Method not found"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request identifier (matches request)
    pub id: RequestId,

    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error result (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Request/Response Identifier
///
/// JSON-RPC allows request IDs to be either numbers or strings.
/// Numbers are typical for sequential requests, strings for correlation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric request ID (common for sequential requests)
    Number(u64),

    /// String request ID (useful for correlation/tracing)
    String(String),
}

/// JSON-RPC 2.0 Error Object
///
/// Standard error codes (from JSON-RPC 2.0 spec):
/// - `-32700`: Parse error (invalid JSON)
/// - `-32600`: Invalid request (malformed JSON-RPC)
/// - `-32601`: Method not found
/// - `-32602`: Invalid params
/// - `-32603`: Internal error
/// - `-32000 to -32099`: Server-defined errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (see JSON-RPC 2.0 spec for standard codes)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Core transport trait for MCP communication
///
/// Implementers provide the low-level communication mechanism (stdio, HTTP, etc.)
/// while the MCP client handles the protocol-level operations.
///
/// Design Pattern: Strategy pattern - transport is injected into MCP client
///
/// Thread Safety: Implementations must be Send + Sync for async operation
///
/// Error Handling:
/// - Transport failures: Return McpError::Transport
/// - Protocol violations: Return McpError::Protocol
/// - Timeout: Return McpError::Transport with timeout message
///
/// Example Implementation:
/// ```rust,ignore
/// struct MyTransport { ... }
///
/// #[async_trait]
/// impl McpTransport for MyTransport {
///     async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
///         // 1. Serialize request to JSON
///         // 2. Send over transport (stdio, HTTP, etc.)
///         // 3. Wait for response
///         // 4. Deserialize and return
///     }
///
///     fn is_connected(&self) -> bool {
///         // Check if transport is ready
///     }
///
///     async fn close(&mut self) -> Result<()> {
///         // Clean up resources
///     }
/// }
/// ```
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a JSON-RPC request and wait for response
    ///
    /// This is the core transport operation. Implementations must:
    /// 1. Serialize the request to JSON
    /// 2. Send it over the transport mechanism
    /// 3. Wait for and receive the response
    /// 4. Deserialize and return the response
    ///
    /// Error Conditions:
    /// - Not connected: Return Transport error
    /// - Timeout: Return Transport error with timeout info
    /// - Invalid response: Return Protocol error
    /// - Connection lost: Return Transport error
    ///
    /// Performance Note: This is a blocking operation at the protocol level
    /// (request/response), but async to avoid blocking the UI thread.
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Check if transport is connected and ready
    ///
    /// Returns true if transport can send requests, false otherwise.
    /// Use this before attempting to send to provide better error messages.
    fn is_connected(&self) -> bool;

    /// Close the transport and clean up resources
    ///
    /// This should:
    /// - Close connections (HTTP) or terminate processes (stdio)
    /// - Release file handles and network sockets
    /// - Mark transport as disconnected
    ///
    /// Should be idempotent - safe to call multiple times.
    ///
    /// Error Conditions:
    /// - Already closed: Return Ok (idempotent)
    /// - Cleanup failure: Log warning but still return Ok
    async fn close(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_serialization() {
        let id = RequestId::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");

        let id = RequestId::String("req-abc-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""req-abc-123""#);
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "tools/list".to_string(),
            params: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "tools/list");
        assert!(json.get("params").is_none());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"status": "ok"}
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, RequestId::Number(1));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
    }

    #[test]
    fn test_request_id_equality() {
        assert_eq!(RequestId::Number(1), RequestId::Number(1));
        assert_ne!(RequestId::Number(1), RequestId::Number(2));

        assert_eq!(
            RequestId::String("abc".to_string()),
            RequestId::String("abc".to_string())
        );
        assert_ne!(
            RequestId::String("abc".to_string()),
            RequestId::String("xyz".to_string())
        );
    }
}
