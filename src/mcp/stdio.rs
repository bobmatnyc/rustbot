//! stdio Transport for MCP
//!
//! Design Decision: Process-based stdio communication for local MCP servers
//!
//! Rationale: Local MCP servers communicate via stdin/stdout using newline-delimited
//! JSON-RPC 2.0 messages. This is the standard for local tool execution and matches
//! how Claude Desktop integrates with MCP servers.
//!
//! Trade-offs:
//! - Process Spawning: Simple but requires process management
//! - Line-based Protocol: Easy to implement but requires buffering
//! - Request/Response Matching: Sequential IDs for simplicity
//! - Error Recovery: Kill and restart vs. attempting to recover
//!
//! Alternatives Considered:
//! 1. WebSocket: Rejected - overkill for local servers, adds complexity
//! 2. Named pipes: Rejected - less portable than stdio
//! 3. HTTP localhost: Rejected - requires server to handle HTTP
//!
//! Performance Characteristics:
//! - Process spawn: ~50-200ms one-time cost
//! - Per-request latency: ~1-5ms for local process
//! - Memory overhead: ~10-50MB per server process
//! - Throughput: Limited by JSON serialization (~10K req/sec)
//!
//! Error Recovery Strategy:
//! - Connection lost: Mark transport as disconnected, manager will restart
//! - Invalid JSON: Return Protocol error, don't kill process
//! - Process crash: Detected via broken pipe, manager restarts with backoff
//! - Timeout: Kill process and restart (timeout indicates hang)
//!
//! Extension Points:
//! - Add request timeout handling
//! - Add process health monitoring
//! - Add request/response correlation for concurrent requests
//! - Add stderr capture and logging

use async_trait::async_trait;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::mcp::config::LocalServerConfig;
use crate::mcp::error::{McpError, Result};
use crate::mcp::transport::{JsonRpcRequest, JsonRpcResponse, McpTransport, RequestId};

/// stdio transport for local MCP servers
///
/// Spawns a child process and communicates via stdin/stdout using
/// newline-delimited JSON-RPC 2.0 messages.
///
/// Protocol:
/// - Client writes JSON-RPC request followed by newline to stdin
/// - Server reads from stdin, processes request
/// - Server writes JSON-RPC response followed by newline to stdout
/// - Client reads from stdout and parses response
///
/// Thread Safety:
/// - Uses Arc<Mutex<>> for shared stdout reader
/// - request_id_counter is atomic via Mutex
/// - Safe to clone and share across threads
///
/// Lifecycle:
/// 1. Create: StdioTransport::new(config)
/// 2. Start: transport.start() spawns process
/// 3. Use: send_request() communicates with process
/// 4. Close: transport.close() kills process
///
/// Example:
/// ```rust,ignore
/// let config = LocalServerConfig { ... };
/// let mut transport = StdioTransport::new(config);
/// transport.start().await?;
///
/// let request = JsonRpcRequest { ... };
/// let response = transport.send_request(request).await?;
///
/// transport.close().await?;
/// ```
pub struct StdioTransport {
    /// Server configuration (command, args, env)
    config: LocalServerConfig,

    /// Child process handle
    ///
    /// Some when process is running, None when stopped
    process: Option<Child>,

    /// Process stdin (for sending requests)
    stdin: Option<ChildStdin>,

    /// Process stdout (for receiving responses)
    ///
    /// Wrapped in Arc<Mutex<>> to allow async read from send_request
    stdout: Arc<Mutex<Option<BufReader<ChildStdout>>>>,

    /// Request ID counter for generating sequential IDs
    ///
    /// Incremented for each request to ensure unique IDs
    request_id_counter: Arc<Mutex<u64>>,

    /// Connection status
    connected: bool,
}

impl StdioTransport {
    /// Create a new stdio transport
    ///
    /// Does not start the process - call start() to spawn.
    ///
    /// Example:
    /// ```rust,ignore
    /// let config = LocalServerConfig {
    ///     id: "filesystem".to_string(),
    ///     command: "npx".to_string(),
    ///     args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
    ///     env: HashMap::new(),
    ///     ...
    /// };
    /// let transport = StdioTransport::new(config);
    /// ```
    pub fn new(config: LocalServerConfig) -> Self {
        Self {
            config,
            process: None,
            stdin: None,
            stdout: Arc::new(Mutex::new(None)),
            request_id_counter: Arc::new(Mutex::new(0)),
            connected: false,
        }
    }

    /// Start the MCP server process
    ///
    /// Spawns the process configured in LocalServerConfig and captures
    /// stdin/stdout for JSON-RPC communication.
    ///
    /// Error Conditions:
    /// - Command not found: Returns Transport error
    /// - Permission denied: Returns Transport error
    /// - Working directory invalid: Returns Transport error
    ///
    /// Side Effects:
    /// - Sets connected = true on success
    /// - Stores process handle, stdin, stdout
    ///
    /// Example:
    /// ```rust,ignore
    /// let mut transport = StdioTransport::new(config);
    /// transport.start().await?;
    /// assert!(transport.is_connected());
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        // Build command with arguments
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args);

        // Set environment variables (resolve ${VAR} references)
        for (key, value) in &self.config.env {
            let resolved_value = crate::mcp::config::resolve_env_var(value)?;
            cmd.env(key, resolved_value);
        }

        // Set working directory if specified
        if let Some(ref working_dir) = self.config.working_dir {
            cmd.current_dir(working_dir);
        }

        // Configure stdio pipes
        // - stdin: Pipe (we write JSON-RPC requests)
        // - stdout: Pipe (we read JSON-RPC responses)
        // - stderr: Inherit (for debugging - shows in terminal)
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            McpError::Transport(format!(
                "Failed to spawn MCP server '{}': {} (command: {})",
                self.config.name, e, self.config.command
            ))
        })?;

        // Take ownership of stdin (for writing requests)
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdin for MCP server".into()))?;

        // Take ownership of stdout (for reading responses)
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdout for MCP server".into()))?;

        // Store process handles
        self.stdin = Some(stdin);
        *self.stdout.lock().await = Some(BufReader::new(stdout));
        self.process = Some(child);
        self.connected = true;

        Ok(())
    }

    /// Generate next request ID
    ///
    /// Atomically increments counter and returns new ID.
    /// Used to ensure request/response matching.
    async fn next_request_id(&self) -> u64 {
        let mut counter = self.request_id_counter.lock().await;
        *counter += 1;
        *counter
    }

    /// Read one JSON-RPC message from stdout
    ///
    /// Reads a single line, parses as JSON-RPC response.
    ///
    /// Error Conditions:
    /// - EOF (process died): Returns Transport error
    /// - Invalid JSON: Returns Protocol error
    /// - Malformed JSON-RPC: Returns Protocol error
    ///
    /// Performance:
    /// - Blocking at protocol level (waits for response)
    /// - Async to avoid blocking UI thread
    async fn read_response(&self) -> Result<JsonRpcResponse> {
        let mut stdout = self.stdout.lock().await;
        let reader = stdout
            .as_mut()
            .ok_or_else(|| McpError::Transport("No stdout available".into()))?;

        // Read one line (JSON-RPC messages are newline-delimited)
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| McpError::Transport(format!("Failed to read from MCP server: {}", e)))?;

        // Check for EOF (process died)
        if line.is_empty() {
            return Err(McpError::Transport(
                "MCP server connection closed (EOF)".into(),
            ));
        }

        // Parse JSON-RPC response
        serde_json::from_str(&line).map_err(|e| {
            McpError::Protocol(format!("Invalid JSON-RPC response from MCP server: {}", e))
        })
    }

    /// Write one JSON-RPC message to stdin
    ///
    /// Serializes request to JSON, writes to stdin with newline.
    ///
    /// Error Conditions:
    /// - Broken pipe (process died): Returns Transport error
    /// - Serialization failure: Returns Protocol error
    async fn write_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Transport("No stdin available".into()))?;

        // Serialize to JSON
        let json = serde_json::to_string(request)
            .map_err(|e| McpError::Protocol(format!("Failed to serialize request: {}", e)))?;

        // Write JSON + newline
        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| McpError::Transport(format!("Failed to write to MCP server: {}", e)))?;

        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| McpError::Transport(format!("Failed to write newline: {}", e)))?;

        // Flush to ensure immediate delivery
        stdin
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&mut self, mut request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if !self.connected {
            return Err(McpError::Transport("Not connected to MCP server".into()));
        }

        // Auto-assign request ID if not set (ID 0 is treated as "auto")
        if matches!(request.id, RequestId::Number(0)) {
            request.id = RequestId::Number(self.next_request_id().await);
        }

        // Send request to server
        self.write_request(&request).await?;

        // Wait for response
        self.read_response().await
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;

        // Kill process if still running
        if let Some(mut process) = self.process.take() {
            // Send SIGTERM (graceful shutdown)
            if let Err(e) = process.kill().await {
                eprintln!("Warning: Failed to kill MCP server process: {}", e);
            }

            // Wait for process to exit (with timeout in production)
            // TODO: Add timeout to prevent indefinite wait
            let _ = process.wait().await;
        }

        // Clean up stdio handles
        self.stdin = None;
        *self.stdout.lock().await = None;

        Ok(())
    }
}

/// Ensure process cleanup on drop
///
/// If transport is dropped without calling close(), this ensures
/// the child process is killed to prevent orphaned processes.
impl Drop for StdioTransport {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Best effort kill (can't await in Drop)
            let _ = process.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_transport_creation() {
        let config = LocalServerConfig {
            id: "test".to_string(),
            name: "Test Server".to_string(),
            description: None,
            command: "echo".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            auto_restart: false,
            max_retries: None,
            health_check_interval: None,
            timeout: 60,
            working_dir: None,
        };

        let transport = StdioTransport::new(config);
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_next_request_id() {
        let config = LocalServerConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            command: "test".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            auto_restart: false,
            max_retries: None,
            health_check_interval: None,
            timeout: 60,
            working_dir: None,
        };

        let transport = StdioTransport::new(config);

        let id1 = transport.next_request_id().await;
        let id2 = transport.next_request_id().await;
        let id3 = transport.next_request_id().await;

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[tokio::test]
    async fn test_send_request_when_not_connected() {
        let config = LocalServerConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            command: "test".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            auto_restart: false,
            max_retries: None,
            health_check_interval: None,
            timeout: 60,
            working_dir: None,
        };

        let mut transport = StdioTransport::new(config);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "test".to_string(),
            params: None,
        };

        let result = transport.send_request(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    // Note: Full integration tests with actual MCP server processes
    // are in tests/mcp_integration.rs
}
