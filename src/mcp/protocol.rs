//! MCP Protocol Types
//!
//! Design Decision: Type-safe MCP protocol message structures
//!
//! Rationale: MCP defines specific message formats for initialization, tool discovery,
//! and tool execution. Creating typed structures ensures compile-time correctness and
//! provides better IDE support. These types match the MCP specification exactly.
//!
//! Trade-offs:
//! - Type Safety vs Flexibility: Specific types prevent errors but require updates for protocol changes
//! - Completeness vs Simplicity: Implement all MCP features vs. just what we need now
//! - Versioning: Support one protocol version (2024-11-05) vs. multiple versions
//!
//! Alternatives Considered:
//! 1. Use serde_json::Value everywhere: Rejected - loses type safety
//! 2. Generate from MCP spec: Rejected - spec not machine-readable yet
//! 3. Support all protocol versions: Deferred - start with latest stable
//!
//! MCP Protocol Reference:
//! - Specification: https://spec.modelcontextprotocol.io/specification/2024-11-05/
//! - Version: 2024-11-05 (latest stable as of implementation)
//!
//! Extension Points:
//! - Add Resources support (resources/list, resources/read)
//! - Add Prompts support (prompts/list, prompts/get)
//! - Add Logging capabilities
//! - Add Sampling support for LLM calls from MCP servers

use serde::{Deserialize, Serialize};

/// MCP Initialize Request Parameters
///
/// Sent by client to server as first message in MCP handshake.
/// Server responds with InitializeResult containing server capabilities.
///
/// Example:
/// ```json
/// {
///   "protocolVersion": "2024-11-05",
///   "capabilities": {},
///   "clientInfo": {
///     "name": "Rustbot",
///     "version": "0.2.1"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// MCP protocol version client implements
    ///
    /// Current supported version: "2024-11-05"
    /// Servers must support this version or return error
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities (what features client supports)
    ///
    /// Phase 2: Basic capabilities only
    /// Phase 5: Add sampling for LLM calls from server
    pub capabilities: ClientCapabilities,

    /// Information about the client application
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Client capabilities declaration
///
/// Tells server what optional MCP features the client supports.
/// Servers can use this to enable/disable features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Support for server-initiated LLM sampling
    ///
    /// Phase 5: Implement sampling capability
    /// Allows servers to request LLM completions from client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<serde_json::Value>,

    /// Experimental features (use for testing new capabilities)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Client application information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client application name
    pub name: String,

    /// Client application version (semantic versioning recommended)
    pub version: String,
}

/// MCP Initialize Response Result
///
/// Server's response to initialize request. Contains server capabilities
/// and information about the server.
///
/// Example:
/// ```json
/// {
///   "protocolVersion": "2024-11-05",
///   "capabilities": {
///     "tools": {}
///   },
///   "serverInfo": {
///     "name": "filesystem-server",
///     "version": "1.0.0"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// MCP protocol version server implements
    ///
    /// Must match or be compatible with client's requested version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Server capabilities (what features server provides)
    pub capabilities: ServerCapabilities,

    /// Information about the server
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Server capabilities declaration
///
/// Indicates which MCP features the server implements.
/// Client should check capabilities before calling optional features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Tool support (tools/list, tools/call)
    ///
    /// If present, server provides tools that can be called
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapability>,

    /// Resource support (resources/list, resources/read)
    ///
    /// If present, server provides resources (files, data, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapability>,

    /// Prompt template support (prompts/list, prompts/get)
    ///
    /// If present, server provides reusable prompt templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapability>,

    /// Logging support
    ///
    /// If present, server can send log messages to client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
}

/// Tool capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    /// Whether server sends notifications when tool list changes
    ///
    /// If true, client should listen for tools/list_changed notifications
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resource capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapability {
    /// Whether clients can subscribe to resource updates
    ///
    /// If true, resources can push updates to client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether server sends notifications when resource list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompt capability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapability {
    /// Whether server sends notifications when prompt list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Server application information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server application name
    pub name: String,

    /// Server application version
    pub version: String,
}

/// MCP Tool Definition
///
/// Describes a tool available from an MCP server. Returned by tools/list.
///
/// Example:
/// ```json
/// {
///   "name": "read_file",
///   "description": "Read contents of a file",
///   "inputSchema": {
///     "type": "object",
///     "properties": {
///       "path": { "type": "string", "description": "File path to read" }
///     },
///     "required": ["path"]
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Tool identifier (unique within server)
    ///
    /// Used in tools/call to specify which tool to invoke
    pub name: String,

    /// Human-readable tool description
    ///
    /// Shown to users and included in LLM tool definitions
    pub description: Option<String>,

    /// JSON Schema describing tool input parameters
    ///
    /// Used to:
    /// 1. Validate arguments before calling tool
    /// 2. Generate UI forms for manual invocation
    /// 3. Provide to LLMs for function calling
    ///
    /// Must be a valid JSON Schema (typically object with properties)
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP Tool List Response
///
/// Response to tools/list request. Contains all tools server provides.
///
/// Example:
/// ```json
/// {
///   "tools": [
///     { "name": "read_file", "description": "...", "inputSchema": {...} },
///     { "name": "write_file", "description": "...", "inputSchema": {...} }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResult {
    /// List of available tools
    pub tools: Vec<McpToolDefinition>,
}

/// MCP Tool Call Request Parameters
///
/// Parameters for tools/call request. Invokes a tool with arguments.
///
/// Example:
/// ```json
/// {
///   "name": "read_file",
///   "arguments": {
///     "path": "/etc/hosts"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Tool name to invoke (from tools/list)
    pub name: String,

    /// Tool arguments (must match tool's inputSchema)
    ///
    /// Can be:
    /// - Object for tools with named parameters
    /// - Array for tools with positional parameters
    /// - null for tools with no parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// MCP Tool Call Response
///
/// Result of tools/call request. Contains tool output and error status.
///
/// Success Example:
/// ```json
/// {
///   "content": [
///     {
///       "type": "text",
///       "text": "File contents here..."
///     }
///   ]
/// }
/// ```
///
/// Error Example:
/// ```json
/// {
///   "content": [
///     {
///       "type": "text",
///       "text": "Error: File not found: /missing.txt"
///     }
///   ],
///   "isError": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Tool output content
    ///
    /// Can contain multiple content blocks (text, images, etc.)
    /// Most tools return a single text block
    pub content: Vec<ToolContent>,

    /// Whether the tool call resulted in an error
    ///
    /// If true, content contains error message
    /// If false or absent, content contains successful result
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool output content block
///
/// MCP supports multiple content types. Phase 2 focuses on text.
/// Future phases can add image, binary data, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    /// Content type (e.g., "text", "image", "resource")
    ///
    /// Phase 2: Only "text" is supported
    /// Phase 4+: Add support for images, files, structured data
    #[serde(rename = "type")]
    pub content_type: String,

    /// Text content (for type="text")
    ///
    /// Contains the actual tool output or error message
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {
                sampling: None,
                experimental: None,
            },
            client_info: ClientInfo {
                name: "Rustbot".to_string(),
                version: "0.2.1".to_string(),
            },
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["protocolVersion"], "2024-11-05");
        assert_eq!(json["clientInfo"]["name"], "Rustbot");
        assert_eq!(json["clientInfo"]["version"], "0.2.1");
    }

    #[test]
    fn test_initialize_result_deserialization() {
        let json = r#"{
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            }
        }"#;

        let result: InitializeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert!(result.capabilities.tools.is_some());
        assert_eq!(result.server_info.name, "test-server");
    }

    #[test]
    fn test_tool_definition_deserialization() {
        let json = r#"{
            "name": "read_file",
            "description": "Read a file",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }
        }"#;

        let tool: McpToolDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(tool.description, Some("Read a file".to_string()));
        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn test_tool_call_params_serialization() {
        let params = ToolCallParams {
            name: "read_file".to_string(),
            arguments: Some(serde_json::json!({
                "path": "/etc/hosts"
            })),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["name"], "read_file");
        assert_eq!(json["arguments"]["path"], "/etc/hosts");
    }

    #[test]
    fn test_tool_call_result_deserialization() {
        let json = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "File contents"
                }
            ]
        }"#;

        let result: ToolCallResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, "text");
        assert_eq!(result.content[0].text, "File contents");
        assert_eq!(result.is_error, None);
    }

    #[test]
    fn test_tool_call_error_result() {
        let json = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "Error: File not found"
                }
            ],
            "isError": true
        }"#;

        let result: ToolCallResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].text.contains("Error"));
    }
}
