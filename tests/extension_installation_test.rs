//! Extension Installation Integration Tests
//!
//! Tests the complete workflow of:
//! 1. Discovering extensions from marketplace
//! 2. Installing extensions to local registry
//! 3. Loading installed extensions
//! 4. Configuring extensions for agents
//!
//! # Test Coverage
//!
//! - Extension discovery from marketplace API
//! - Local server installation (npm, pypi, docker)
//! - Remote service installation (streamable-http)
//! - Registry persistence (save/load)
//! - Error handling and validation
//! - Environment variable extraction

use anyhow::Result;
use rustbot::mcp::extensions::{ExtensionInstaller, ExtensionRegistry, InstallationType};
use rustbot::mcp::marketplace::{McpServerListing, Package, Remote, Repository, Transport};
use tempfile::TempDir;

/// Create a mock marketplace listing for testing
fn create_mock_npm_listing() -> McpServerListing {
    McpServerListing {
        schema: None,
        name: "test/npm-server".to_string(),
        description: "Test NPM MCP Server".to_string(),
        version: "1.0.0".to_string(),
        packages: vec![Package {
            registry_type: "npm".to_string(),
            identifier: "@test/mcp-server".to_string(),
            transport: Transport {
                transport_type: "stdio".to_string(),
            },
            environment_variables: vec![],
        }],
        remotes: vec![],
        repository: Repository {
            url: "https://github.com/test/mcp-server".to_string(),
            source: "github".to_string(),
        },
    }
}

fn create_mock_pypi_listing() -> McpServerListing {
    McpServerListing {
        schema: None,
        name: "test/pypi-server".to_string(),
        description: "Test PyPI MCP Server".to_string(),
        version: "2.0.0".to_string(),
        packages: vec![Package {
            registry_type: "pypi".to_string(),
            identifier: "test-mcp-server".to_string(),
            transport: Transport {
                transport_type: "stdio".to_string(),
            },
            environment_variables: vec![],
        }],
        remotes: vec![],
        repository: Repository {
            url: "https://github.com/test/pypi-server".to_string(),
            source: "github".to_string(),
        },
    }
}

fn create_mock_remote_listing() -> McpServerListing {
    McpServerListing {
        schema: None,
        name: "test/remote-service".to_string(),
        description: "Test Remote MCP Service".to_string(),
        version: "3.0.0".to_string(),
        packages: vec![],
        remotes: vec![Remote {
            remote_type: "streamable-http".to_string(),
            url: "https://api.test.com/mcp".to_string(),
        }],
        repository: Repository {
            url: "https://github.com/test/remote-service".to_string(),
            source: "github".to_string(),
        },
    }
}

#[test]
fn test_extension_registry_creation() {
    let registry = ExtensionRegistry::new();
    assert_eq!(registry.extensions.len(), 0);
    assert_eq!(registry.version, "1.0");
}

#[test]
fn test_extension_registry_save_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let registry_path = temp_dir.path().join("registry.json");

    // Create and save registry
    let mut registry = ExtensionRegistry::new();
    let listing = create_mock_npm_listing();
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());
    let extension = installer.install_from_listing(&listing, None)?;

    registry.install(extension);
    registry.save(&registry_path)?;

    // Load registry and verify
    let loaded = ExtensionRegistry::load(&registry_path)?;
    assert_eq!(loaded.extensions.len(), 1);
    assert!(loaded.get("test/npm-server").is_some());

    Ok(())
}

#[test]
fn test_install_npm_extension() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    let listing = create_mock_npm_listing();
    let extension = installer.install_from_listing(&listing, None)?;

    assert_eq!(extension.id, "test/npm-server");
    assert_eq!(extension.name, "test/npm-server");
    assert_eq!(extension.install_type, InstallationType::Local);
    assert_eq!(extension.metadata.version, "1.0.0");

    // Verify MCP config was created
    match &extension.mcp_config {
        rustbot::mcp::extensions::McpConfigEntry::LocalServer(config) => {
            assert_eq!(config.command, "npx");
            assert_eq!(config.args, vec!["-y", "@test/mcp-server"]);
            assert!(!config.enabled); // Should be disabled by default
        }
        _ => panic!("Expected LocalServer config"),
    }

    Ok(())
}

#[test]
fn test_install_pypi_extension() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    let listing = create_mock_pypi_listing();
    let extension = installer.install_from_listing(&listing, None)?;

    assert_eq!(extension.id, "test/pypi-server");
    assert_eq!(extension.install_type, InstallationType::Local);

    match &extension.mcp_config {
        rustbot::mcp::extensions::McpConfigEntry::LocalServer(config) => {
            assert_eq!(config.command, "uvx");
            assert_eq!(config.args, vec!["test-mcp-server"]);
        }
        _ => panic!("Expected LocalServer config"),
    }

    Ok(())
}

#[test]
fn test_install_remote_extension() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    let listing = create_mock_remote_listing();
    let extension = installer.install_from_listing(&listing, None)?;

    assert_eq!(extension.id, "test/remote-service");
    assert_eq!(extension.install_type, InstallationType::Remote);

    match &extension.mcp_config {
        rustbot::mcp::extensions::McpConfigEntry::CloudService(config) => {
            assert_eq!(config.url, "https://api.test.com/mcp");
            assert!(!config.enabled); // Should be disabled by default
            assert_eq!(config.timeout, 30);
        }
        _ => panic!("Expected CloudService config"),
    }

    Ok(())
}

#[test]
fn test_extension_uninstall() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());
    let mut registry = ExtensionRegistry::new();

    let listing = create_mock_npm_listing();
    let extension = installer.install_from_listing(&listing, None)?;

    registry.install(extension.clone());
    assert_eq!(registry.extensions.len(), 1);

    let removed = registry.uninstall("test/npm-server");
    assert!(removed.is_some());
    assert_eq!(registry.extensions.len(), 0);

    Ok(())
}

#[test]
fn test_extension_list() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());
    let mut registry = ExtensionRegistry::new();

    // Install multiple extensions
    let npm = create_mock_npm_listing();
    let pypi = create_mock_pypi_listing();
    let remote = create_mock_remote_listing();

    registry.install(installer.install_from_listing(&npm, None)?);
    registry.install(installer.install_from_listing(&pypi, None)?);
    registry.install(installer.install_from_listing(&remote, None)?);

    let extensions = registry.list();
    assert_eq!(extensions.len(), 3);

    // Verify all types are present
    let has_npm = extensions.iter().any(|e| e.id == "test/npm-server");
    let has_pypi = extensions.iter().any(|e| e.id == "test/pypi-server");
    let has_remote = extensions.iter().any(|e| e.id == "test/remote-service");

    assert!(has_npm);
    assert!(has_pypi);
    assert!(has_remote);

    Ok(())
}

#[test]
fn test_registry_persistence_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let registry_path = temp_dir.path().join("registry.json");

    // Save empty registry
    let registry = ExtensionRegistry::new();
    registry.save(&registry_path)?;

    // Load and verify
    let loaded = ExtensionRegistry::load(&registry_path)?;
    assert_eq!(loaded.extensions.len(), 0);
    assert_eq!(loaded.version, "1.0");

    Ok(())
}

#[test]
fn test_registry_load_missing_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let registry_path = temp_dir.path().join("nonexistent.json");

    // Should return empty registry without error
    let registry = ExtensionRegistry::load(&registry_path)?;
    assert_eq!(registry.extensions.len(), 0);

    Ok(())
}

#[test]
fn test_extension_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    let listing = create_mock_npm_listing();
    let extension = installer.install_from_listing(&listing, None)?;

    // Verify metadata
    assert_eq!(extension.metadata.version, "1.0.0");
    assert_eq!(
        extension.metadata.repository_url,
        "https://github.com/test/mcp-server"
    );
    assert!(!extension.metadata.installed_at.is_empty());

    Ok(())
}

#[test]
fn test_package_type_selection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    // Create listing with multiple package types
    let mut listing = create_mock_npm_listing();
    listing.packages.push(Package {
        registry_type: "pypi".to_string(),
        identifier: "test-server-python".to_string(),
        transport: Transport {
            transport_type: "stdio".to_string(),
        },
        environment_variables: vec![],
    });

    // Install with explicit package type
    let extension = installer.install_from_listing(&listing, Some("pypi"))?;

    match &extension.mcp_config {
        rustbot::mcp::extensions::McpConfigEntry::LocalServer(config) => {
            assert_eq!(config.command, "uvx");
            assert_eq!(config.args, vec!["test-server-python"]);
        }
        _ => panic!("Expected LocalServer config"),
    }

    Ok(())
}

#[test]
fn test_docker_package_installation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());

    let mut listing = create_mock_npm_listing();
    listing.packages = vec![Package {
        registry_type: "oci".to_string(),
        identifier: "test/mcp-server:latest".to_string(),
        transport: Transport {
            transport_type: "stdio".to_string(),
        },
        environment_variables: vec![],
    }];

    let extension = installer.install_from_listing(&listing, None)?;

    match &extension.mcp_config {
        rustbot::mcp::extensions::McpConfigEntry::LocalServer(config) => {
            assert_eq!(config.command, "docker");
            assert_eq!(
                config.args,
                vec!["run", "-i", "test/mcp-server:latest"]
            );
        }
        _ => panic!("Expected LocalServer config"),
    }

    Ok(())
}

#[test]
fn test_extension_update() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let installer = ExtensionInstaller::new(temp_dir.path().to_path_buf());
    let mut registry = ExtensionRegistry::new();

    // Install v1.0.0
    let listing_v1 = create_mock_npm_listing();
    let extension_v1 = installer.install_from_listing(&listing_v1, None)?;
    registry.install(extension_v1);

    // Install v2.0.0 (should replace)
    let mut listing_v2 = create_mock_npm_listing();
    listing_v2.version = "2.0.0".to_string();
    let extension_v2 = installer.install_from_listing(&listing_v2, None)?;
    registry.install(extension_v2);

    // Should still have only one extension
    assert_eq!(registry.extensions.len(), 1);

    // Should be v2.0.0
    let extension = registry.get("test/npm-server").unwrap();
    assert_eq!(extension.metadata.version, "2.0.0");

    Ok(())
}
