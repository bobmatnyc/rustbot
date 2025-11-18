//! MCP Extensions System
//!
//! Extensions are downloadable MCP services from the registry that can be:
//! 1. Installed locally or configured as remote services
//! 2. Associated with specific agents (not global features)
//! 3. Loaded as part of an agent's toolbox
//!
//! # Architecture
//!
//! ```text
//! Registry (marketplace.rs) → Extensions (installed) → Agent Config → Tools
//!     ↓                           ↓                        ↓            ↓
//! Browse/Search              Download/Install         Enable/Disable  Load
//! ```
//!
//! # Design Decisions
//!
//! - Extensions are per-agent, not global (agents have different toolsets)
//! - Installation creates MCP config entries (local or remote)
//! - Agent configs reference which extensions to enable
//! - API layer loads tools based on agent's enabled extensions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

use super::config::{LocalServerConfig, CloudServiceConfig};
use super::marketplace::{McpServerListing, Package};

/// Extension installation state
///
/// Tracks what extensions are installed and their configuration.
/// Stored in: ~/.rustbot/extensions/registry.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRegistry {
    /// Installed extensions indexed by ID (e.g., "ai.exa/exa")
    pub extensions: HashMap<String, InstalledExtension>,

    /// Registry format version for future compatibility
    pub version: String,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            version: "1.0".to_string(),
        }
    }

    /// Load registry from file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)
            .context("Failed to read extension registry")?;

        let registry: Self = serde_json::from_str(&content)
            .context("Failed to parse extension registry")?;

        Ok(registry)
    }

    /// Save registry to file
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create extensions directory")?;
        }

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize extension registry")?;

        std::fs::write(path, json)
            .context("Failed to write extension registry")?;

        Ok(())
    }

    /// Add or update an extension
    pub fn install(&mut self, extension: InstalledExtension) {
        self.extensions.insert(extension.id.clone(), extension);
    }

    /// Remove an extension
    pub fn uninstall(&mut self, extension_id: &str) -> Option<InstalledExtension> {
        self.extensions.remove(extension_id)
    }

    /// Get an extension by ID
    pub fn get(&self, extension_id: &str) -> Option<&InstalledExtension> {
        self.extensions.get(extension_id)
    }

    /// List all installed extensions
    pub fn list(&self) -> Vec<&InstalledExtension> {
        self.extensions.values().collect()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// An installed extension
///
/// Represents an MCP service that has been downloaded/configured
/// and is available for agents to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    /// Extension ID from registry (e.g., "ai.exa/exa")
    pub id: String,

    /// Display name
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: String,

    /// Installation type (local server or remote service)
    pub install_type: InstallationType,

    /// MCP configuration (either local or remote)
    pub mcp_config: McpConfigEntry,

    /// Installation metadata
    pub metadata: InstallationMetadata,
}

/// Installation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallationType {
    /// Local server (stdio transport)
    Local,

    /// Remote service (HTTP transport)
    Remote,
}

/// MCP configuration entry
///
/// Either a local server or cloud service config.
/// This gets merged into mcp_config.json when agent enables the extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConfigEntry {
    LocalServer(LocalServerConfig),
    CloudService(CloudServiceConfig),
}

/// Installation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationMetadata {
    /// Version installed
    pub version: String,

    /// When installed
    pub installed_at: String,

    /// Repository URL
    #[serde(default)]
    pub repository_url: String,

    /// Required environment variables (for user to configure)
    #[serde(default)]
    pub required_env_vars: Vec<String>,
}

/// Extension installer
///
/// Handles converting marketplace listings into installed extensions.
pub struct ExtensionInstaller {
    /// Path to extension installations (e.g., ~/.rustbot/extensions/bin/)
    install_dir: PathBuf,
}

impl ExtensionInstaller {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    /// Install an extension from a marketplace listing
    ///
    /// # Arguments
    /// * `listing` - The marketplace server listing to install
    /// * `package_type` - Which package to install (if multiple available)
    ///
    /// # Returns
    /// `InstalledExtension` ready to add to registry
    ///
    /// # Implementation
    /// For Phase 1, this creates configuration only (no actual download).
    /// Future phases will handle npm/pypi/docker installation.
    pub fn install_from_listing(
        &self,
        listing: &McpServerListing,
        package_type: Option<&str>,
    ) -> Result<InstalledExtension> {
        // Determine installation type
        let (install_type, mcp_config) = if !listing.remotes.is_empty() {
            // Remote service (streamable-http)
            self.create_remote_config(listing)?
        } else {
            // Local server (find appropriate package)
            self.create_local_config(listing, package_type)?
        };

        // Extract required env vars
        let required_env_vars = self.extract_required_env_vars(listing);

        let extension = InstalledExtension {
            id: listing.name.clone(),
            name: listing.name.clone(),
            description: listing.description.clone(),
            install_type,
            mcp_config,
            metadata: InstallationMetadata {
                version: listing.version.clone(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                repository_url: listing.repository.url.clone(),
                required_env_vars,
            },
        };

        Ok(extension)
    }

    /// Create remote service configuration
    fn create_remote_config(
        &self,
        listing: &McpServerListing,
    ) -> Result<(InstallationType, McpConfigEntry)> {
        let remote = listing.remotes.first()
            .context("No remote endpoints available")?;

        let config = CloudServiceConfig {
            id: listing.name.clone(),
            name: listing.name.clone(),
            description: Some(listing.description.clone()),
            url: remote.url.clone(),
            auth: None, // User must configure authentication
            enabled: false, // Disabled by default until user configures
            max_retries: Some(3), // Default 3 retries
            health_check_interval: None,
            timeout: 30, // Default 30s timeout
        };

        Ok((InstallationType::Remote, McpConfigEntry::CloudService(config)))
    }

    /// Create local server configuration
    fn create_local_config(
        &self,
        listing: &McpServerListing,
        package_type: Option<&str>,
    ) -> Result<(InstallationType, McpConfigEntry)> {
        // Find appropriate package
        let package = if let Some(pkg_type) = package_type {
            listing.packages.iter()
                .find(|p| p.registry_type == pkg_type)
                .with_context(|| format!("Package type {} not found", pkg_type))?
        } else {
            // Default preference: npm > pypi > oci
            listing.packages.iter()
                .find(|p| p.registry_type == "npm")
                .or_else(|| listing.packages.iter().find(|p| p.registry_type == "pypi"))
                .or_else(|| listing.packages.iter().find(|p| p.registry_type == "oci"))
                .context("No supported package type found")?
        };

        let (command, args) = self.package_to_command(package)?;

        let config = LocalServerConfig {
            id: listing.name.clone(),
            name: listing.name.clone(),
            description: Some(listing.description.clone()),
            command,
            args,
            env: HashMap::new(), // User must configure env vars
            enabled: false, // Disabled by default
            auto_restart: true,
            max_retries: Some(3),
            health_check_interval: None,
            timeout: 30,
            working_dir: None,
        };

        Ok((InstallationType::Local, McpConfigEntry::LocalServer(config)))
    }

    /// Convert package to command and args
    fn package_to_command(&self, package: &Package) -> Result<(String, Vec<String>)> {
        match package.registry_type.as_str() {
            "npm" => {
                // npm packages use: npx -y <package-name>
                let package_name = package.identifier.clone();
                Ok(("npx".to_string(), vec!["-y".to_string(), package_name]))
            }
            "pypi" => {
                // pypi packages use: uvx <package-name>
                let package_name = package.identifier.clone();
                Ok(("uvx".to_string(), vec![package_name]))
            }
            "oci" => {
                // OCI containers use: docker run <image>
                let image = package.identifier.clone();
                Ok(("docker".to_string(), vec!["run".to_string(), "-i".to_string(), image]))
            }
            other => anyhow::bail!("Unsupported package type: {}", other),
        }
    }

    /// Extract required environment variables from listing
    fn extract_required_env_vars(&self, listing: &McpServerListing) -> Vec<String> {
        listing.packages.iter()
            .flat_map(|pkg| {
                pkg.environment_variables.iter()
                    .map(|env_var| env_var.name.clone())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_registry_new() {
        let registry = ExtensionRegistry::new();
        assert_eq!(registry.extensions.len(), 0);
        assert_eq!(registry.version, "1.0");
    }

    #[test]
    fn test_extension_registry_install() {
        let mut registry = ExtensionRegistry::new();

        let extension = InstalledExtension {
            id: "test/extension".to_string(),
            name: "Test Extension".to_string(),
            description: "A test extension".to_string(),
            install_type: InstallationType::Local,
            mcp_config: McpConfigEntry::LocalServer(LocalServerConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: None,
                command: "npx".to_string(),
                args: vec![],
                env: HashMap::new(),
                enabled: false,
                auto_restart: true,
                max_retries: Some(3),
                health_check_interval: None,
                timeout: 30,
                working_dir: None,
            }),
            metadata: InstallationMetadata {
                version: "1.0.0".to_string(),
                installed_at: "2025-01-01T00:00:00Z".to_string(),
                repository_url: "https://github.com/test/repo".to_string(),
                required_env_vars: vec![],
            },
        };

        registry.install(extension.clone());
        assert_eq!(registry.extensions.len(), 1);
        assert!(registry.get("test/extension").is_some());
    }
}
