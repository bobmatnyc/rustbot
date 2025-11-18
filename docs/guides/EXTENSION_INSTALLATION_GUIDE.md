# MCP Extension Installation Guide

**Version**: 1.0
**Last Updated**: 2025-11-17
**Status**: Complete

---

## Overview

This guide covers the MCP Extension Installation system that allows users to discover, install, and configure MCP servers from the official Anthropic registry.

## Table of Contents

1. [Architecture](#architecture)
2. [User Workflow](#user-workflow)
3. [Technical Implementation](#technical-implementation)
4. [Testing](#testing)
5. [Troubleshooting](#troubleshooting)

---

## Architecture

### Components

The extension installation system consists of three main components:

```
┌─────────────────────┐
│  Marketplace UI     │  ← Browse/Search servers
│  (marketplace.rs)   │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│ Extension Installer │  ← Convert listing → config
│  (extensions.rs)    │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│ Extension Registry  │  ← Persistent storage
│  (extensions.rs)    │
└─────────────────────┘
```

### Data Flow

1. **Discovery**: User browses marketplace → selects server
2. **Installation**: Click "Install" → create MCP config entry
3. **Registry**: Save to `~/.rustbot/extensions/registry.json`
4. **Configuration**: User configures env vars/settings
5. **Usage**: Agent loads extension's tools

---

## User Workflow

### Step 1: Browse Marketplace

1. Launch Rustbot
2. Navigate to **Extensions → Marketplace**
3. Browse or search for MCP servers
4. Click on a server to view details

### Step 2: Install Extension

1. In the server details panel, click **"Install Extension"**
2. Installation creates:
   - Extension entry in registry
   - MCP configuration (local or remote)
   - Metadata (version, env vars, etc.)

3. Success message appears:
   ```
   ✓ Successfully installed 'server-name'. Configure in Extensions → Installed.
   ```

### Step 3: Configure Extension (Manual)

**Current State**: Extensions are installed but **disabled by default**.

**Required Configuration**:
- Set environment variables (API keys, etc.)
- Enable the extension in `mcp_config.json`
- Restart Rustbot to load the extension

**Example**: Installing the Exa search server

1. Install via marketplace → creates entry:
   ```json
   {
     "id": "ai.exa/exa",
     "name": "ai.exa/exa",
     "install_type": "local",
     "mcp_config": {
       "local_server": {
         "command": "npx",
         "args": ["-y", "@exa-labs/exa-mcp-server"],
         "env": {},
         "enabled": false  ← Disabled until configured
       }
     },
     "metadata": {
       "required_env_vars": ["EXA_API_KEY"]
     }
   }
   ```

2. User manually adds to `mcp_config.json`:
   ```json
   {
     "local_servers": {
       "exa": {
         "command": "npx",
         "args": ["-y", "@exa-labs/exa-mcp-server"],
         "env": {
           "EXA_API_KEY": "your-key-here"
         },
         "enabled": true  ← User enables
       }
     }
   }
   ```

3. Restart Rustbot → Extension tools available

---

## Technical Implementation

### Extension Registry Schema

**File**: `~/.rustbot/extensions/registry.json`

```json
{
  "version": "1.0",
  "extensions": {
    "server-id": {
      "id": "server-id",
      "name": "Display Name",
      "description": "Server description",
      "install_type": "local" | "remote",
      "mcp_config": {
        "local_server": {
          "id": "server-id",
          "command": "npx",
          "args": ["-y", "package-name"],
          "env": {},
          "enabled": false,
          "timeout": 30
        }
      },
      "metadata": {
        "version": "1.0.0",
        "installed_at": "2025-11-17T12:00:00Z",
        "repository_url": "https://github.com/...",
        "required_env_vars": ["API_KEY"]
      }
    }
  }
}
```

### Installation Types

#### Local Server (stdio transport)

**Package Types Supported**:
- **npm**: `npx -y <package>`
- **pypi**: `uvx <package>`
- **oci** (Docker): `docker run -i <image>`

**Example**: npm package installation
```rust
let extension = installer.install_from_listing(&listing, None)?;
// Creates LocalServerConfig with command: "npx", args: ["-y", "package"]
```

#### Remote Service (streamable-http)

**Example**: Remote API service
```rust
let extension = installer.install_from_listing(&listing, None)?;
// Creates CloudServiceConfig with url: "https://api.example.com/mcp"
```

### Code Example

```rust
use rustbot::mcp::extensions::{ExtensionInstaller, ExtensionRegistry};
use rustbot::mcp::marketplace::McpServerListing;

// Load registry
let registry_path = PathBuf::from("~/.rustbot/extensions/registry.json");
let mut registry = ExtensionRegistry::load(&registry_path)?;

// Create installer
let install_dir = PathBuf::from("~/.rustbot/extensions/bin");
let installer = ExtensionInstaller::new(install_dir);

// Install extension
let listing: McpServerListing = /* from marketplace API */;
let extension = installer.install_from_listing(&listing, None)?;

// Add to registry
registry.install(extension);
registry.save(&registry_path)?;

println!("Installed: {}", extension.name);
```

---

## Testing

### Running Tests

```bash
# Run all extension installation tests
cargo test --test extension_installation_test

# Run specific test
cargo test --test extension_installation_test test_install_npm_extension

# Run with output
cargo test --test extension_installation_test -- --nocapture
```

### Test Coverage

**13 Integration Tests** (all passing):

1. **Registry Operations**:
   - `test_extension_registry_creation` - Create new registry
   - `test_extension_registry_save_load` - Persistence
   - `test_registry_persistence_empty` - Empty registry
   - `test_registry_load_missing_file` - Missing file handling

2. **Installation Types**:
   - `test_install_npm_extension` - npm package
   - `test_install_pypi_extension` - PyPI package
   - `test_docker_package_installation` - Docker (OCI)
   - `test_install_remote_extension` - Remote service

3. **Registry Management**:
   - `test_extension_uninstall` - Remove extension
   - `test_extension_list` - List all extensions
   - `test_extension_update` - Update to new version

4. **Advanced Features**:
   - `test_extension_metadata` - Verify metadata
   - `test_package_type_selection` - Multiple package types

### Test Results

```
running 13 tests
test test_extension_registry_creation ... ok
test test_docker_package_installation ... ok
test test_install_remote_extension ... ok
test test_extension_metadata ... ok
test test_extension_list ... ok
test test_install_pypi_extension ... ok
test test_extension_update ... ok
test test_extension_uninstall ... ok
test test_registry_load_missing_file ... ok
test test_package_type_selection ... ok
test test_install_npm_extension ... ok
test test_registry_persistence_empty ... ok
test test_extension_registry_save_load ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured
```

### Manual Testing Checklist

- [ ] Open marketplace view
- [ ] Search for a server (e.g., "filesystem")
- [ ] Select a server
- [ ] Click "Install Extension"
- [ ] Verify success message appears
- [ ] Check `~/.rustbot/extensions/registry.json` created
- [ ] Verify extension entry has correct configuration
- [ ] Verify "Installed" badge shows on button
- [ ] Click install again → no action (already installed)
- [ ] Manually configure in `mcp_config.json`
- [ ] Restart Rustbot
- [ ] Verify extension tools load

---

## Troubleshooting

### Installation Fails

**Symptom**: Error message "✗ Installation failed: ..."

**Possible Causes**:
1. **No supported package type**:
   - Extension doesn't provide npm/pypi/docker package
   - Solution: Use "Copy Configuration" and configure manually

2. **Missing package information**:
   - Marketplace listing incomplete
   - Solution: Report to registry maintainers

3. **Permission issues**:
   - Can't write to `~/.rustbot/extensions/`
   - Solution: Check directory permissions

### Registry Not Saving

**Symptom**: Extension installs but doesn't persist

**Solution**:
```bash
# Check if directory exists
ls -la ~/.rustbot/extensions/

# Create if missing
mkdir -p ~/.rustbot/extensions/

# Check permissions
chmod 755 ~/.rustbot/extensions/
```

### Extension Not Loading

**Symptom**: Installed but tools don't appear

**Causes**:
1. Extension not enabled in `mcp_config.json`
2. Missing environment variables
3. Rustbot not restarted

**Solution**:
```bash
# 1. Check mcp_config.json
cat ~/.rustbot/mcp_config.json | grep -A5 "extension-name"

# 2. Verify environment variables set
# 3. Restart Rustbot
```

### Package Type Selection

**Symptom**: Wrong package type installed

**Solution**:
```rust
// Explicitly specify package type
let extension = installer.install_from_listing(&listing, Some("pypi"))?;
```

**Default Priority**:
1. npm (if available)
2. pypi
3. oci (Docker)
4. Remote service

---

## Future Enhancements

### Phase 2: Automated Configuration

**Planned Features**:
- UI for configuring environment variables
- Enable/disable extensions from UI
- Test extension connection before enabling
- Auto-restart Rustbot after configuration

### Phase 3: Package Management

**Planned Features**:
- Actually download and install packages (npm/pypi/docker)
- Version management and updates
- Dependency resolution
- Uninstall cleanup (remove installed packages)

### Phase 4: Advanced Features

**Planned Features**:
- Extension categories and tags
- User ratings and reviews
- Installation history
- Backup and restore configurations

---

## API Reference

### ExtensionRegistry

```rust
impl ExtensionRegistry {
    pub fn new() -> Self;
    pub fn load(path: &Path) -> Result<Self>;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn install(&mut self, extension: InstalledExtension);
    pub fn uninstall(&mut self, extension_id: &str) -> Option<InstalledExtension>;
    pub fn get(&self, extension_id: &str) -> Option<&InstalledExtension>;
    pub fn list(&self) -> Vec<&InstalledExtension>;
}
```

### ExtensionInstaller

```rust
impl ExtensionInstaller {
    pub fn new(install_dir: PathBuf) -> Self;

    pub fn install_from_listing(
        &self,
        listing: &McpServerListing,
        package_type: Option<&str>,
    ) -> Result<InstalledExtension>;
}
```

---

## Related Documentation

- [MCP Marketplace Guide](../progress/2025-11-16-marketplace-phase1.md)
- [Extension System Architecture](../architecture/mcp_extensions.md)
- [Testing Guide](../qa/TESTING_METHODS.md)

---

**Document Status**: ✅ Complete
**Test Coverage**: ✅ 13/13 passing
**Production Ready**: ⚠️ Partial (UI complete, automated config pending)
