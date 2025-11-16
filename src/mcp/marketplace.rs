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
    /// List of MCP server listings (wrapped in server + metadata)
    pub servers: Vec<McpServerWrapper>,

    /// Pagination metadata (uses cursor-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RegistryMetadata>,
}

/// Wrapper for server listing with metadata
///
/// Each entry in the registry contains the server definition and
/// associated metadata (publication info, official status, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerWrapper {
    /// The actual server definition
    pub server: McpServerListing,

    /// Registry metadata (publication date, official status)
    #[serde(rename = "_meta")]
    pub meta: ServerMeta,
}

/// Server listing metadata
///
/// Complete metadata for an MCP server as provided by the registry.
/// Includes all information needed to install and configure the server.
///
/// # Field Descriptions
///
/// - `name`: Unique identifier and display name (e.g., "ai.exa/exa")
/// - `description`: Human-readable description of functionality
/// - `repository`: Source code repository information
/// - `version`: Latest version number
/// - `packages`: Installation packages (npm, pypi, docker, etc.)
/// - `remotes`: Remote server endpoints (for streamable-http servers)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerListing {
    /// Schema URL (optional, not used)
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Server name (unique identifier, e.g., "ai.exa/exa")
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Source code repository
    #[serde(default)]
    pub repository: Repository,

    /// Version string
    #[serde(default)]
    pub version: String,

    /// Installation packages (OCI, npm, etc.)
    #[serde(default)]
    pub packages: Vec<Package>,

    /// Remote endpoints (for HTTP-based servers)
    #[serde(default)]
    pub remotes: Vec<Remote>,
}

/// Repository information
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Repository {
    /// Repository URL (e.g., "https://github.com/exa-labs/exa-mcp-server")
    #[serde(default)]
    pub url: String,

    /// Source type (e.g., "github")
    #[serde(default)]
    pub source: String,
}

/// Package installation information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    /// Registry type: "oci", "npm", "pypi", etc.
    #[serde(rename = "registryType")]
    pub registry_type: String,

    /// Package identifier (e.g., "docker.io/aliengiraffe/spotdb:0.1.0")
    pub identifier: String,

    /// Transport configuration
    #[serde(default)]
    pub transport: Transport,

    /// Required environment variables
    #[serde(rename = "environmentVariables", default)]
    pub environment_variables: Vec<EnvironmentVariable>,
}

/// Transport configuration for package
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Transport {
    /// Transport type (e.g., "stdio")
    #[serde(rename = "type", default)]
    pub transport_type: String,
}

/// Environment variable definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvironmentVariable {
    /// Variable name
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Value format (e.g., "string")
    #[serde(default)]
    pub format: String,

    /// Whether this is a secret (should be hidden in UI)
    #[serde(rename = "isSecret", default)]
    pub is_secret: bool,
}

/// Remote server endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Remote {
    /// Remote type (e.g., "streamable-http")
    #[serde(rename = "type")]
    pub remote_type: String,

    /// Remote URL
    pub url: String,
}

/// Server metadata from registry
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerMeta {
    /// Official registry metadata
    #[serde(rename = "io.modelcontextprotocol.registry/official")]
    pub official: OfficialMetadata,
}

/// Official registry metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OfficialMetadata {
    /// Publication status (e.g., "active")
    pub status: String,

    /// When the server was first published
    #[serde(rename = "publishedAt")]
    pub published_at: String,

    /// When the server was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: String,

    /// Whether this is the latest version
    #[serde(rename = "isLatest")]
    pub is_latest: bool,
}

/// Registry metadata (pagination)
///
/// Uses cursor-based pagination for efficient result navigation.
///
/// # Example
/// ```ignore
/// if let Some(metadata) = registry.metadata {
///     if let Some(next_cursor) = metadata.next_cursor {
///         // Load next page using cursor
///     }
/// }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryMetadata {
    /// Cursor for next page of results
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// Number of servers in this response
    pub count: usize,
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
