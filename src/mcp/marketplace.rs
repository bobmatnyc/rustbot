//! MCP Marketplace integration
//!
//! Provides programmatic access to the official MCP Registry at
//! https://registry.modelcontextprotocol.io for discovering and browsing
//! available MCP servers.
//!
//! # Design Rationale
//!
//! This module wraps the official Anthropic MCP Registry API to enable:
//! - Discovery of official and community MCP servers
//! - Search and filtering by package type, keywords, official status
//! - Metadata retrieval for installation and configuration
//!
//! # API Structure
//!
//! The registry provides a RESTful JSON API with endpoints:
//! - GET /v0.1/servers - List all servers with pagination
//! - GET /v0.1/servers?search=query - Search servers by keywords
//! - GET /v0.1/servers?limit=N&offset=M - Paginated results
//!
//! # Performance Characteristics
//!
//! - Network latency: ~100-500ms per API call (depends on internet connection)
//! - Response size: ~5-20KB for 20 servers (typical pagination)
//! - Client caching: Not implemented in Phase 1 (future optimization)
//!
//! # Error Handling
//!
//! Network errors and JSON parsing errors are wrapped in `MarketplaceError`.
//! The UI layer is responsible for user-facing error messages and retry logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base URL for the official MCP Registry
const REGISTRY_BASE_URL: &str = "https://registry.modelcontextprotocol.io";

/// API version - currently v0.1
const API_VERSION: &str = "v0.1";

/// MCP Registry client
///
/// Provides async methods to interact with the MCP Registry API.
///
/// # Thread Safety
///
/// The client uses `reqwest::Client` internally, which is designed for
/// concurrent use and connection pooling. Safe to clone and share across threads.
pub struct MarketplaceClient {
    http_client: reqwest::Client,
    base_url: String,
}

impl MarketplaceClient {
    /// Create a new marketplace client
    ///
    /// Initializes an HTTP client with default settings (connection pooling enabled).
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: format!("{}/{}", REGISTRY_BASE_URL, API_VERSION),
        }
    }

    /// List all servers with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of servers to return (typically 10-50)
    /// * `offset` - Number of servers to skip for pagination
    ///
    /// # Returns
    /// `Result<McpRegistry, MarketplaceError>` - Registry response with servers and pagination metadata
    ///
    /// # Performance
    /// - Network round-trip: ~100-500ms depending on connection
    /// - Response parsing: <10ms for typical 20-server response
    ///
    /// # Example
    /// ```ignore
    /// let client = MarketplaceClient::new();
    /// let registry = client.list_servers(20, 0).await?;
    /// println!("Found {} servers", registry.servers.len());
    /// ```
    pub async fn list_servers(&self, limit: usize, offset: usize) -> Result<McpRegistry, MarketplaceError> {
        let url = format!("{}/servers?limit={}&offset={}", self.base_url, limit, offset);
        let response = self.http_client.get(&url).send().await?;

        // Check for HTTP errors before parsing
        let response = response.error_for_status()?;

        let registry: McpRegistry = response.json().await?;
        Ok(registry)
    }

    /// Search servers by query string
    ///
    /// # Arguments
    /// * `query` - Search keywords (matches against name and description)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// `Result<McpRegistry, MarketplaceError>` - Filtered registry response
    ///
    /// # Search Behavior
    /// - Case-insensitive keyword matching
    /// - Searches both server name and description fields
    /// - Exact behavior depends on registry API implementation
    ///
    /// # Example
    /// ```ignore
    /// let client = MarketplaceClient::new();
    /// let results = client.search_servers("filesystem", 20).await?;
    /// ```
    pub async fn search_servers(&self, query: &str, limit: usize) -> Result<McpRegistry, MarketplaceError> {
        let url = format!("{}/servers?search={}&limit={}", self.base_url, query, limit);
        let response = self.http_client.get(&url).send().await?;

        // Check for HTTP errors before parsing
        let response = response.error_for_status()?;

        let registry: McpRegistry = response.json().await?;
        Ok(registry)
    }
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry response structure
///
/// Returned by all marketplace API endpoints. Contains list of servers
/// and optional pagination metadata for navigating large result sets.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpRegistry {
    /// List of MCP server listings
    pub servers: Vec<McpServerListing>,

    /// Pagination metadata (present when results are paginated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}

/// Server listing metadata
///
/// Complete metadata for an MCP server as provided by the registry.
/// Includes all information needed to install and configure the server.
///
/// # Field Descriptions
///
/// - `name`: Unique identifier and display name
/// - `description`: Human-readable description of functionality
/// - `package_type`: Installation method (npm, pypi, docker, remote)
/// - `package`: Package identifier (e.g., npm package name)
/// - `command`: Executable command to run the server
/// - `args`: Command-line arguments for the server
/// - `env`: Required environment variables (keys only, values set by user)
/// - `official`: True if maintained by Anthropic
/// - `version`: Latest version number (if versioned)
/// - `homepage`: Documentation/repository URL
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerListing {
    /// Server name (unique identifier)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Package type: "npm", "pypi", "docker", or "remote"
    #[serde(rename = "packageType")]
    pub package_type: String,

    /// Package identifier (e.g., "@modelcontextprotocol/server-filesystem")
    pub package: String,

    /// Command to execute (e.g., "npx", "uvx", "docker")
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Environment variable keys (user must provide values)
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// True if official Anthropic server
    #[serde(default)]
    pub official: bool,

    /// Version string (if versioned)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Homepage/documentation URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// Pagination metadata
///
/// Provides information for navigating through paginated results.
///
/// # Example
/// ```ignore
/// if let Some(pagination) = registry.pagination {
///     let has_more = (pagination.offset + pagination.limit) < pagination.total;
///     if has_more {
///         // Load next page
///     }
/// }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pagination {
    /// Total number of servers available
    pub total: usize,

    /// Number of servers per page
    pub limit: usize,

    /// Number of servers skipped
    pub offset: usize,
}

/// Marketplace error types
///
/// Wraps network and parsing errors from the registry API.
///
/// # Error Handling Strategy
///
/// - `NetworkError`: Retry with exponential backoff (UI responsibility)
/// - `ParseError`: Log error and show generic failure message
#[derive(Debug)]
pub enum MarketplaceError {
    /// HTTP request failed (network error, DNS failure, timeout, etc.)
    NetworkError(reqwest::Error),

    /// JSON parsing failed (malformed response)
    ParseError(serde_json::Error),
}

impl From<reqwest::Error> for MarketplaceError {
    fn from(err: reqwest::Error) -> Self {
        MarketplaceError::NetworkError(err)
    }
}

impl From<serde_json::Error> for MarketplaceError {
    fn from(err: serde_json::Error) -> Self {
        MarketplaceError::ParseError(err)
    }
}

impl std::fmt::Display for MarketplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketplaceError::NetworkError(e) => write!(f, "Network error: {}", e),
            MarketplaceError::ParseError(e) => write!(f, "Failed to parse response: {}", e),
        }
    }
}

impl std::error::Error for MarketplaceError {}
