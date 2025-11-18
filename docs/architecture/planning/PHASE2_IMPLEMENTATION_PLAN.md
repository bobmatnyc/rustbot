---
title: Phase 2 Implementation Plan - AppBuilder and Service Integration
category: Architecture
audience: Developer, PM
reading_time: 45 minutes
last_updated: 2025-01-17
status: Ready for Implementation
---

# Phase 2 Implementation Plan: AppBuilder and Service Integration

**Project**: Rustbot Service Layer Refactoring
**Phase**: 2 of 4 - Application Builder and Main Integration
**Status**: 🟢 READY TO BEGIN
**Estimated Duration**: 2-3 weeks
**Risk Level**: Medium (UI integration required)

---

## Executive Summary

Phase 2 builds on the trait interfaces created in Phase 1 to integrate the service layer with the main application. The primary deliverable is an **AppBuilder** that constructs the entire dependency graph using the builder pattern, replacing the hardcoded initialization in `main.rs`.

**Key Objectives**:
1. Fix Phase 1 blockers (runtime nesting, `.expect()` calls, test coverage)
2. Create mock implementations for all traits using `mockall`
3. Implement `AppBuilder` for dependency construction
4. Integrate services with `main.rs` and `RustbotApp`
5. Achieve 100% test coverage for service layer

**Success Metrics**:
- All Phase 1 tests passing (22/22)
- Test coverage >80% for service layer
- Main app using AppBuilder pattern
- Zero `.expect()` calls in production code
- No performance regressions

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Phase 1 Blockers Resolution](#2-phase-1-blockers-resolution)
3. [Mock Implementations](#3-mock-implementations)
4. [AppBuilder Design](#4-appbuilder-design)
5. [Main.rs Integration](#5-mainrs-integration)
6. [Task Breakdown](#6-task-breakdown)
7. [Testing Strategy](#7-testing-strategy)
8. [Risk Assessment](#8-risk-assessment)
9. [Success Criteria](#9-success-criteria)
10. [Timeline and Estimates](#10-timeline-and-estimates)

---

## 1. Current State Analysis

### 1.1 Phase 1 Completion Status

**Completed** ✅:
- Trait interfaces defined (FileSystem, StorageService, ConfigService, AgentService)
- Real implementations working (RealFileSystem, FileStorageService, FileConfigService, DefaultAgentService)
- 16/22 tests passing (72.7% success rate)
- Build successful with acceptable warnings
- Excellent documentation

**Blockers** ⚠️:
- 6 agent service tests failing (runtime nesting issue)
- Production code contains `.expect()` calls (4 instances)
- Code formatting inconsistencies
- Test coverage gaps (agent service 0%, overall ~65%)

### 1.2 Dependencies in main.rs

**Current Initialization Pattern** (lines 175-284):
```rust
fn new(api_key: String) -> Self {
    // 1. Create event bus
    let event_bus = Arc::new(EventBus::new());

    // 2. Create runtime
    let runtime = Arc::new(Runtime::new().expect(...));  // ❌ HARDCODED

    // 3. Create LLM adapter
    let llm_adapter = Arc::from(create_adapter(...));    // ❌ HARDCODED

    // 4. Load agents from files
    let agent_loader = AgentLoader::new();               // ❌ HARDCODED FILESYSTEM
    let agent_configs = agent_loader.load_all()?;

    // 5. Build API
    let api = RustbotApiBuilder::new()
        .event_bus(event_bus.clone())
        .runtime(runtime.clone())
        .llm_adapter(llm_adapter.clone())
        .build()?;                                       // ❌ HARDCODED DEPENDENCIES

    // 6. Initialize MCP manager
    let mcp_manager = McpPluginManager::with_event_bus(...); // ❌ HARDCODED

    // 7. Load MCP config from filesystem
    if mcp_config_path.exists() {
        runtime.block_on(manager.load_config(...))?;     // ❌ DIRECT FILESYSTEM
    }

    // 8. Create views
    let plugins_view = PluginsView::new(...);
    let marketplace_view = MarketplaceView::new(...);
    let mermaid_renderer = MermaidRenderer::new();
}
```

**Problems**:
- ❌ All dependencies hardcoded in constructor
- ❌ Direct filesystem access (not using FileSystem trait)
- ❌ No dependency injection
- ❌ Impossible to unit test without real filesystem
- ❌ Tight coupling to concrete implementations
- ❌ Runtime created with `.expect()` (can panic)

**Target Pattern** (after Phase 2):
```rust
fn main() -> Result<()> {
    // Build entire dependency graph
    let app = AppBuilder::new()
        .with_production_deps()?
        .with_api_key(env::var("OPENROUTER_API_KEY")?)
        .build()?;

    // Run UI
    eframe::run_native("Rustbot", options, Box::new(|_cc| Box::new(app)))?;
    Ok(())
}

// Test version
#[test]
fn test_app_initialization() {
    let app = AppBuilder::new()
        .with_test_deps()
        .build()
        .unwrap();
    // All dependencies injected, no filesystem I/O
}
```

### 1.3 Dependency Graph

```
┌─────────────────────────────────────────────────────┐
│                   RustbotApp                        │
│                  (UI Layer)                         │
└────┬─────────┬─────────┬──────────┬────────────────┘
     │         │         │          │
     ▼         ▼         ▼          ▼
┌─────────┐ ┌────────┐ ┌──────────┐ ┌──────────────┐
│   API   │ │ Events │ │   MCP    │ │   Mermaid    │
│         │ │  Bus   │ │ Manager  │ │   Renderer   │
└────┬────┘ └───┬────┘ └────┬─────┘ └──────────────┘
     │          │           │
     ▼          ▼           ▼
┌──────────────────────────────────────┐
│        Service Layer (Phase 1)       │
│  ┌──────────┐  ┌──────────────┐     │
│  │  Agent   │  │  Storage     │     │
│  │  Service │  │  Service     │     │
│  └────┬─────┘  └──────┬───────┘     │
│       │               │             │
│  ┌────▼────┐    ┌─────▼──────┐     │
│  │ Config  │    │ Filesystem │     │
│  │ Service │    │  Service   │     │
│  └─────────┘    └────────────┘     │
└──────────────────────────────────────┘
```

**AppBuilder Responsibilities**:
1. Create all service instances in correct order
2. Wire dependencies between services
3. Create runtime and event bus
4. Create LLM adapters
5. Initialize MCP manager
6. Load configurations
7. Validate entire dependency graph
8. Return fully-initialized RustbotApp

---

## 2. Phase 1 Blockers Resolution

### 2.1 Critical Issue C-1: Runtime Nesting

**Problem**:
```rust
// src/services/agents.rs:55
pub struct DefaultAgentService {
    runtime: Arc<Runtime>,  // ❌ Creates nested runtime in tests
}

// Test fails:
#[tokio::test]
async fn test_agent_service() {
    // tokio::test creates runtime
    let service = DefaultAgentService::new(...).await?;  // Creates ANOTHER runtime
    // ❌ Panic: "Cannot drop a runtime in a context where blocking is not allowed"
}
```

**Root Cause**: Tokio runtime dropping within async context violates tokio constraints.

**Solution 1**: Use `Handle` instead of `Runtime` (RECOMMENDED)
```rust
// Change from:
pub struct DefaultAgentService {
    runtime: Arc<Runtime>,
}

// To:
pub struct DefaultAgentService {
    runtime: tokio::runtime::Handle,
}

impl DefaultAgentService {
    pub async fn new(
        config_service: Arc<dyn ConfigService>,
        event_bus: Arc<EventBus>,
        runtime: tokio::runtime::Handle,  // ✅ Handle, not Runtime
        system_instructions: String,
    ) -> Result<Self> {
        // Use runtime.spawn() for async tasks
        // No ownership issues, no drop issues
    }
}

// Usage:
let handle = tokio::runtime::Handle::current();
let service = DefaultAgentService::new(..., handle, ...).await?;
```

**Solution 2**: External Runtime Management
```rust
// Don't store runtime at all, pass it when needed
impl DefaultAgentService {
    pub async fn execute_with_runtime<F>(&self, runtime: &Runtime, f: F)
    where F: FnOnce() -> Result<()>
    {
        runtime.block_on(async { f() })
    }
}
```

**Recommendation**: **Solution 1** (Handle) - cleaner API, follows tokio best practices.

**Impact**:
- Lines changed: ~20 (agents.rs)
- Tests fixed: 6/6
- Breaking change: Constructor signature (acceptable for Phase 2)

**Effort**: 2-3 hours

---

### 2.2 Critical Issue C-2: Production `.expect()` Calls

**Problem**: 4 instances of `.expect()` in production code can panic at runtime.

**Locations**:
```rust
// src/services/agents.rs:114
.expect("At least one agent should exist")

// src/services/agents.rs:160
.expect("Active agent should always exist")

// src/services/filesystem.rs:82,87,119 (tests only - acceptable)
```

**Solution**: Replace with proper error handling

**Before**:
```rust
// agents.rs:114
let active_agent_id = agent_configs.first()
    .expect("At least one agent should exist")  // ❌ PANIC
    .id.clone();
```

**After**:
```rust
// agents.rs:114
let active_agent_id = agent_configs.first()
    .ok_or_else(|| RustbotError::ConfigError(
        "No agents found in configuration. At least one agent is required.".to_string()
    ))?
    .id.clone();
```

**Before**:
```rust
// agents.rs:160
self.agents.get(&self.active_agent_id)
    .expect("Active agent should always exist")  // ❌ PANIC
    .clone()
```

**After**:
```rust
// agents.rs:160
self.agents.get(&self.active_agent_id)
    .cloned()
    .ok_or_else(|| RustbotError::AgentNotFound(
        format!("Active agent '{}' not found in registry", self.active_agent_id)
    ))
```

**Impact**:
- Lines changed: 4-6
- Return type changes: Add `Result<>` to methods
- Callers must handle errors (good practice)

**Effort**: 1-2 hours

---

### 2.3 High Priority Issue H-1: Code Formatting

**Problem**: `cargo fmt --check` fails with formatting inconsistencies.

**Solution**:
```bash
cargo fmt
```

**Validation**:
```bash
cargo fmt --check  # Should pass
```

**Impact**: Cosmetic only

**Effort**: 5 minutes

---

### 2.4 Test Coverage Gaps

**Current Coverage**:
```
Storage Service:   85% ✅
Config Service:    70% ✅
Filesystem Service: 80% ✅
Agent Service:      0% ❌ (all tests failing)
Overall:           65% ⚠️
```

**Target Coverage**: >80%

**Action Plan**:
1. Fix agent service tests (C-1)
2. Add missing edge case tests
3. Add error path tests
4. Add concurrent access tests

**Effort**: 4-6 hours

---

## 3. Mock Implementations

### 3.1 Overview

Create mock implementations for all service traits using `mockall` crate.

**Why mockall?**
- ✅ Auto-generates mock implementations from traits
- ✅ Fluent expectation API
- ✅ Type-safe
- ✅ Works with async traits (via async-trait)
- ✅ Industry standard in Rust

### 3.2 FileSystem Mock

**Location**: `src/services/filesystem.rs`

**Implementation**:
```rust
#[cfg(test)]
use mockall::predicate::*;
#[cfg(test)]
use mockall::mock;

#[cfg(test)]
mock! {
    pub Filesystem {}

    #[async_trait]
    impl FileSystem for Filesystem {
        async fn read_to_string(&self, path: &Path) -> Result<String>;
        async fn write(&self, path: &Path, content: &str) -> Result<()>;
        async fn exists(&self, path: &Path) -> bool;
        async fn create_dir_all(&self, path: &Path) -> Result<()>;
        async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    }
}
```

**Usage Example**:
```rust
#[tokio::test]
async fn test_with_mock_filesystem() {
    let mut mock_fs = MockFilesystem::new();

    // Set expectations
    mock_fs.expect_read_to_string()
        .with(eq(Path::new("test.json")))
        .times(1)
        .returning(|_| Ok(r#"{"key": "value"}"#.to_string()));

    // Use mock
    let content = mock_fs.read_to_string(Path::new("test.json")).await?;
    assert_eq!(content, r#"{"key": "value"}"#);

    // Mock verifies expectations were met
}
```

### 3.3 StorageService Mock

**Location**: `src/services/storage.rs`

**Implementation**:
```rust
#[cfg(test)]
mock! {
    pub Storage {}

    #[async_trait]
    impl StorageService for Storage {
        async fn load_token_stats(&self) -> Result<TokenStats>;
        async fn save_token_stats(&self, stats: &TokenStats) -> Result<()>;
        async fn load_system_prompts(&self) -> Result<SystemPrompts>;
        async fn save_system_prompts(&self, prompts: &SystemPrompts) -> Result<()>;
    }
}
```

**Usage**:
```rust
#[tokio::test]
async fn test_storage_error_handling() {
    let mut mock_storage = MockStorage::new();

    // Simulate disk full error
    mock_storage.expect_save_token_stats()
        .returning(|_| Err(RustbotError::IoError(
            std::io::Error::new(std::io::ErrorKind::Other, "Disk full")
        )));

    let result = mock_storage.save_token_stats(&TokenStats::default()).await;
    assert!(result.is_err());
}
```

### 3.4 ConfigService Mock

**Location**: `src/services/config.rs`

**Implementation**:
```rust
#[cfg(test)]
mock! {
    pub Config {}

    #[async_trait]
    impl ConfigService for Config {
        async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>>;
        async fn save_agent_config(&self, config: &AgentConfig) -> Result<()>;
        async fn get_active_agent_id(&self) -> Result<String>;
        async fn set_active_agent_id(&self, id: &str) -> Result<()>;
        fn get_agents_dir(&self) -> PathBuf;
        fn get_api_key(&self) -> Result<String>;
        fn get_model(&self) -> String;
    }
}
```

### 3.5 AgentService Mock

**Location**: `src/services/agents.rs`

**Implementation**:
```rust
#[cfg(test)]
mock! {
    pub Agent {}

    #[async_trait]
    impl AgentService for Agent {
        async fn get_agent(&self, id: &str) -> Result<Arc<Agent>>;
        fn list_agents(&self) -> Vec<String>;
        async fn switch_agent(&mut self, id: &str) -> Result<()>;
        fn current_agent(&self) -> Arc<Agent>;
    }
}
```

### 3.6 Manual Mock (Simple Alternative)

For simple cases, manual mocks are acceptable:

```rust
#[cfg(test)]
pub struct MockConfigService {
    pub api_key: String,
    pub model: String,
    pub agents_dir: PathBuf,
    pub agent_configs: Vec<AgentConfig>,
}

#[cfg(test)]
impl MockConfigService {
    pub fn new() -> Self {
        Self {
            api_key: "test-key-12345".to_string(),
            model: "test/model".to_string(),
            agents_dir: PathBuf::from("test-agents"),
            agent_configs: vec![AgentConfig::default_assistant()],
        }
    }
}

#[cfg(test)]
#[async_trait]
impl ConfigService for MockConfigService {
    async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>> {
        Ok(self.agent_configs.clone())
    }
    // ... other methods
}
```

**Trade-offs**:
- ✅ Simple, no external dependencies
- ✅ Full control over behavior
- ❌ More boilerplate
- ❌ No automatic verification

**Recommendation**: Use `mockall` for complex traits, manual mocks for simple ones.

---

## 4. AppBuilder Design

### 4.1 Builder Pattern Architecture

**Goal**: Construct the entire dependency graph using the builder pattern.

**Responsibilities**:
1. Create all service instances
2. Wire dependencies in correct order
3. Validate configuration
4. Support both production and test modes
5. Provide clear error messages

**Design**:
```rust
// src/app_builder.rs

use crate::services::*;
use crate::events::EventBus;
use crate::llm::{LlmAdapter, create_adapter, AdapterType};
use crate::mcp::manager::McpPluginManager;
use crate::mermaid::MermaidRenderer;
use crate::api::{RustbotApi, RustbotApiBuilder};
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;

/// Builder for constructing RustbotApp with dependency injection
///
/// This builder constructs the entire dependency graph for the application,
/// wiring together all services, adapters, and infrastructure components.
///
/// Usage (Production):
///     let app = AppBuilder::new()
///         .with_production_deps()?
///         .with_api_key(env::var("OPENROUTER_API_KEY")?)
///         .build()?;
///
/// Usage (Testing):
///     let app = AppBuilder::new()
///         .with_test_deps()
///         .build()?;
///
pub struct AppBuilder {
    // Core infrastructure
    runtime: Option<Arc<Runtime>>,
    event_bus: Option<Arc<EventBus>>,

    // Services
    filesystem: Option<Arc<dyn FileSystem>>,
    storage: Option<Arc<dyn StorageService>>,
    config: Option<Arc<dyn ConfigService>>,
    agent_service: Option<Arc<dyn AgentService>>,

    // Adapters
    llm_adapter: Option<Arc<dyn LlmAdapter>>,

    // MCP
    mcp_manager: Option<Arc<Mutex<McpPluginManager>>>,

    // Rendering
    mermaid_renderer: Option<Arc<Mutex<MermaidRenderer>>>,

    // Configuration
    api_key: Option<String>,
    system_instructions: Option<String>,

    // Build mode
    mode: BuildMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BuildMode {
    Production,
    Test,
}

impl AppBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            runtime: None,
            event_bus: None,
            filesystem: None,
            storage: None,
            config: None,
            agent_service: None,
            llm_adapter: None,
            mcp_manager: None,
            mermaid_renderer: None,
            api_key: None,
            system_instructions: None,
            mode: BuildMode::Production,
        }
    }

    /// Configure builder with production dependencies
    ///
    /// Creates real implementations of all services:
    /// - RealFileSystem for filesystem operations
    /// - FileStorageService for persistence
    /// - FileConfigService for configuration
    /// - DefaultAgentService for agent management
    /// - Real tokio runtime
    ///
    /// # Errors
    /// - Configuration loading errors
    /// - Filesystem access errors
    /// - Missing required environment variables
    pub fn with_production_deps(mut self) -> Result<Self> {
        self.mode = BuildMode::Production;

        // Create runtime
        let runtime = Arc::new(
            Runtime::new()
                .context("Failed to create tokio runtime")?
        );

        // Create event bus
        let event_bus = Arc::new(EventBus::new());

        // Create filesystem
        let filesystem = Arc::new(RealFileSystem) as Arc<dyn FileSystem>;

        // Create config service (loads from .env)
        let config = Arc::new(
            FileConfigService::load()
                .context("Failed to load configuration from .env")?
        ) as Arc<dyn ConfigService>;

        // Create storage service
        let storage_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".rustbot")
            .join("data");

        let storage = Arc::new(
            FileStorageService::new(filesystem.clone(), storage_dir)
        ) as Arc<dyn StorageService>;

        self.runtime = Some(runtime);
        self.event_bus = Some(event_bus);
        self.filesystem = Some(filesystem);
        self.config = Some(config);
        self.storage = Some(storage);

        Ok(self)
    }

    /// Configure builder with test dependencies
    ///
    /// Creates mock implementations for testing:
    /// - MockFileSystem (in-memory)
    /// - Mock storage
    /// - Mock configuration
    /// - Test runtime
    #[cfg(test)]
    pub fn with_test_deps(mut self) -> Self {
        self.mode = BuildMode::Test;

        // Create runtime
        let runtime = Arc::new(
            Runtime::new().expect("Failed to create test runtime")
        );

        // Create event bus
        let event_bus = Arc::new(EventBus::new());

        // Create mock filesystem
        let filesystem = Arc::new(MockFileSystem::new()) as Arc<dyn FileSystem>;

        // Create mock config
        let config = Arc::new(MockConfigService::new()) as Arc<dyn ConfigService>;

        // Create mock storage
        let storage = Arc::new(MockStorageService::new()) as Arc<dyn StorageService>;

        self.runtime = Some(runtime);
        self.event_bus = Some(event_bus);
        self.filesystem = Some(filesystem);
        self.config = Some(config);
        self.storage = Some(storage);
        self.api_key = Some("test-api-key".to_string());

        self
    }

    /// Set API key for LLM adapter
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set system instructions for all agents
    pub fn with_system_instructions(mut self, instructions: String) -> Self {
        self.system_instructions = Some(instructions);
        self
    }

    /// Override runtime (for advanced use cases)
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Override event bus (for advanced use cases)
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Build the application with all dependencies
    ///
    /// Constructs the dependency graph and validates configuration.
    ///
    /// # Errors
    /// - Missing required dependencies
    /// - Service initialization failures
    /// - Configuration validation failures
    pub fn build(self) -> Result<RustbotApp> {
        // Validate required dependencies
        let runtime = self.runtime
            .context("Runtime not configured. Call with_production_deps() or with_test_deps()")?;

        let event_bus = self.event_bus
            .context("Event bus not configured")?;

        let config = self.config
            .context("Config service not configured")?;

        let storage = self.storage
            .context("Storage service not configured")?;

        let api_key = self.api_key
            .or_else(|| config.get_api_key().ok())
            .context("API key not provided and not found in configuration")?;

        // Create LLM adapter
        let llm_adapter = self.llm_adapter.unwrap_or_else(|| {
            Arc::from(create_adapter(AdapterType::OpenRouter, api_key.clone()))
        });

        // Load system instructions from storage
        let system_prompts = runtime.block_on(async {
            storage.load_system_prompts().await
        }).unwrap_or_default();

        let system_instructions = self.system_instructions
            .unwrap_or_else(|| system_prompts.system_instructions.clone());

        // Create agent service
        let agent_service = runtime.block_on(async {
            DefaultAgentService::new(
                config.clone(),
                event_bus.clone(),
                runtime.handle().clone(),  // ✅ Use Handle, not Arc<Runtime>
                system_instructions.clone(),
            ).await
        })?;

        // Create API
        let mut api_builder = RustbotApiBuilder::new()
            .event_bus(event_bus.clone())
            .runtime(runtime.clone())
            .llm_adapter(llm_adapter.clone())
            .max_history_size(20)
            .system_instructions(system_instructions.clone());

        // Load agents and add to API builder
        let agent_configs = runtime.block_on(async {
            config.load_agent_configs().await
        })?;

        for agent_config in &agent_configs {
            api_builder = api_builder.add_agent(agent_config.clone());
        }

        let api = api_builder.build()
            .context("Failed to build RustbotApi")?;

        // Create MCP manager
        let mcp_manager = self.mcp_manager.unwrap_or_else(|| {
            Arc::new(Mutex::new(McpPluginManager::with_event_bus(
                Some(event_bus.clone())
            )))
        });

        // Load MCP configuration if available
        let mcp_config_path = PathBuf::from("mcp_config.json");
        if runtime.block_on(async {
            self.filesystem.as_ref().unwrap().exists(&mcp_config_path).await
        }) {
            runtime.block_on(async {
                if let Ok(mut manager) = mcp_manager.try_lock() {
                    match manager.load_config(&mcp_config_path).await {
                        Ok(_) => {
                            tracing::info!("✓ Loaded MCP configuration");
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load MCP config: {}", e);
                        }
                    }
                }
            });
        }

        // Create mermaid renderer
        let mermaid_renderer = self.mermaid_renderer
            .unwrap_or_else(|| Arc::new(Mutex::new(MermaidRenderer::new())));

        // Load token stats
        let token_stats = runtime.block_on(async {
            storage.load_token_stats().await
        }).unwrap_or_default();

        // Create plugins view
        let plugins_view = Some(PluginsView::new(
            mcp_manager.clone(),
            runtime.handle().clone()
        ));

        // Create marketplace view
        let extensions_marketplace_view = Some(MarketplaceView::new(
            runtime.handle().clone()
        ));

        // Build RustbotApp
        Ok(RustbotApp {
            api: Arc::new(Mutex::new(api)),
            runtime,
            event_bus,
            mcp_manager,
            mermaid_renderer,
            agent_configs,
            token_stats,
            system_prompts,
            plugins_view,
            extensions_marketplace_view,
            // ... other UI state fields
            message_input: String::new(),
            messages: Vec::new(),
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            spinner_rotation: 0.0,
            context_tracker: ContextTracker::default(),
            sidebar_open: true,
            current_view: AppView::Chat,
            settings_view: SettingsView::Agents,
            current_activity: None,
            event_rx: event_bus.subscribe(),
            selected_agent_index: None,
            event_history: VecDeque::with_capacity(50),
            show_event_visualizer: true,
            pending_agent_result: None,
            extensions_view: ExtensionsView::default(),
            markdown_cache: CommonMarkCache::default(),
        })
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4.2 Error Handling

**Validation Strategy**:
```rust
impl AppBuilder {
    fn validate(&self) -> Result<()> {
        // Check required dependencies
        if self.runtime.is_none() {
            return Err(anyhow::anyhow!("Runtime not configured"));
        }

        if self.config.is_none() {
            return Err(anyhow::anyhow!("Config service not configured"));
        }

        // Check API key (production only)
        if self.mode == BuildMode::Production && self.api_key.is_none() {
            return Err(anyhow::anyhow!(
                "API key required for production mode. Set OPENROUTER_API_KEY environment variable."
            ));
        }

        Ok(())
    }
}
```

**Error Context**:
```rust
// Good error messages
let runtime = self.runtime
    .context("Runtime not configured. Call with_production_deps() or with_test_deps()")?;

let api_key = self.api_key
    .or_else(|| config.get_api_key().ok())
    .context("API key not provided and not found in .env file")?;
```

### 4.3 Async Initialization

**Problem**: Some initialization requires async operations (loading configs, etc.)

**Solution**: Use `Runtime::block_on()` in builder
```rust
pub fn build(self) -> Result<RustbotApp> {
    let runtime = self.runtime.context("...")?;

    // Load configs asynchronously
    let agent_configs = runtime.block_on(async {
        config.load_agent_configs().await
    })?;

    // Create async services
    let agent_service = runtime.block_on(async {
        DefaultAgentService::new(...).await
    })?;

    Ok(app)
}
```

**Alternative**: Async builder
```rust
pub async fn build(self) -> Result<RustbotApp> {
    let agent_configs = config.load_agent_configs().await?;
    // ...
}

// Usage in main:
#[tokio::main]
async fn main() -> Result<()> {
    let app = AppBuilder::new()
        .with_production_deps()?
        .build().await?;  // ✅ Async
}
```

**Recommendation**: Sync builder with internal `block_on()` - simpler for egui integration.

---

## 5. Main.rs Integration

### 5.1 Current Implementation

**Before** (lines 175-284):
```rust
impl RustbotApp {
    fn new(api_key: String) -> Self {
        // 150+ lines of hardcoded initialization
        let runtime = Arc::new(Runtime::new().expect(...));
        let event_bus = Arc::new(EventBus::new());
        let llm_adapter = Arc::from(create_adapter(...));
        let agent_loader = AgentLoader::new();
        let agent_configs = agent_loader.load_all()?;
        // ... many more dependencies
    }
}
```

### 5.2 Target Implementation

**After**:
```rust
impl RustbotApp {
    /// Create app using AppBuilder (constructor injection)
    ///
    /// This constructor receives all dependencies via the builder pattern.
    /// For testing, use AppBuilder::with_test_deps().
    ///
    /// # Arguments
    /// All dependencies injected from AppBuilder
    fn from_builder(
        api: Arc<Mutex<RustbotApi>>,
        runtime: Arc<Runtime>,
        event_bus: Arc<EventBus>,
        mcp_manager: Arc<Mutex<McpPluginManager>>,
        mermaid_renderer: Arc<Mutex<MermaidRenderer>>,
        agent_configs: Vec<AgentConfig>,
        token_stats: TokenStats,
        system_prompts: SystemPrompts,
        plugins_view: Option<PluginsView>,
        extensions_marketplace_view: Option<MarketplaceView>,
    ) -> Self {
        Self {
            api,
            runtime,
            event_bus: event_bus.subscribe(),
            mcp_manager,
            mermaid_renderer,
            agent_configs,
            token_stats,
            system_prompts,
            plugins_view,
            extensions_marketplace_view,
            // UI state initialized with defaults
            message_input: String::new(),
            messages: Vec::new(),
            response_rx: None,
            current_response: String::new(),
            is_waiting: false,
            spinner_rotation: 0.0,
            context_tracker: ContextTracker::default(),
            sidebar_open: true,
            current_view: AppView::Chat,
            settings_view: SettingsView::Agents,
            current_activity: None,
            selected_agent_index: None,
            event_history: VecDeque::with_capacity(50),
            show_event_visualizer: true,
            pending_agent_result: None,
            extensions_view: ExtensionsView::default(),
            markdown_cache: CommonMarkCache::default(),
        }
    }

    /// Legacy constructor for backward compatibility
    ///
    /// DEPRECATED: Use AppBuilder instead
    /// This will be removed in v0.3.0
    #[deprecated(
        since = "0.2.4",
        note = "Use AppBuilder::new().with_production_deps().build() instead"
    )]
    fn new(api_key: String) -> Self {
        // Delegate to AppBuilder
        AppBuilder::new()
            .with_production_deps()
            .expect("Failed to initialize production dependencies")
            .with_api_key(api_key)
            .build()
            .expect("Failed to build application")
    }
}
```

### 5.3 Main Function

**Before**:
```rust
fn main() -> Result<(), eframe::Error> {
    // Load env vars
    dotenv::dotenv().ok();

    // Get API key
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY not set");

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Run app
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rustbot",
        options,
        Box::new(|_cc| Box::new(RustbotApp::new(api_key))),  // ❌ HARDCODED
    )
}
```

**After**:
```rust
fn main() -> Result<(), eframe::Error> {
    // Initialize logging first
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Build application with dependency injection
    let app = AppBuilder::new()
        .with_production_deps()
        .map_err(|e| {
            tracing::error!("Failed to initialize dependencies: {:#}", e);
            eframe::Error::Any(Box::new(e))
        })?
        .with_api_key(
            std::env::var("OPENROUTER_API_KEY")
                .map_err(|_| {
                    eframe::Error::Any(Box::new(anyhow::anyhow!(
                        "OPENROUTER_API_KEY environment variable not set"
                    )))
                })?
        )
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build application: {:#}", e);
            eframe::Error::Any(Box::new(e))
        })?;

    // Configure UI options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    // Run application
    eframe::run_native(
        "Rustbot",
        options,
        Box::new(|_cc| Box::new(app)),  // ✅ INJECTED DEPENDENCIES
    )
}
```

**Benefits**:
- ✅ All dependencies injected
- ✅ Clear error messages
- ✅ Testable (can use `AppBuilder::with_test_deps()`)
- ✅ No hardcoded filesystem access
- ✅ Runtime managed by builder

### 5.4 Testing Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization_with_test_deps() {
        let app = AppBuilder::new()
            .with_test_deps()
            .build()
            .expect("Failed to build test app");

        // App should be initialized with mock dependencies
        assert_eq!(app.messages.len(), 0);
        assert_eq!(app.current_view, AppView::Chat);
    }

    #[tokio::test]
    async fn test_app_with_mock_config() {
        // Create mock config
        let mut mock_config = MockConfigService::new();
        mock_config.agent_configs = vec![
            AgentConfig {
                id: "test-agent".to_string(),
                name: "Test Agent".to_string(),
                // ...
            }
        ];

        // Build app with custom mock
        let app = AppBuilder::new()
            .with_test_deps()
            .with_config(Arc::new(mock_config))
            .build()
            .expect("Failed to build app");

        // Verify agent loaded
        assert_eq!(app.agent_configs.len(), 1);
        assert_eq!(app.agent_configs[0].id, "test-agent");
    }
}
```

---

## 6. Task Breakdown

### 6.1 Step 1: Fix Phase 1 Blockers

**Duration**: 1 day (8 hours)

**Tasks**:

#### Task 1.1: Fix Runtime Nesting Issue (C-1)
**Effort**: 2-3 hours
**Files**: `src/services/agents.rs`

- [ ] Change `Arc<Runtime>` to `tokio::runtime::Handle` in `DefaultAgentService`
- [ ] Update constructor signature
- [ ] Update all usages of `runtime.block_on()` to `runtime.spawn()`
- [ ] Update tests to pass `Handle::current()`
- [ ] Verify all 6 agent service tests pass

#### Task 1.2: Remove Production `.expect()` Calls (C-2)
**Effort**: 1-2 hours
**Files**: `src/services/agents.rs`

- [ ] Replace `.expect()` at line 114 with `ok_or_else()`
- [ ] Replace `.expect()` at line 160 with `ok_or_else()`
- [ ] Update method return types to `Result<>`
- [ ] Update callers to handle errors
- [ ] Add tests for error paths

#### Task 1.3: Run Code Formatting (H-1)
**Effort**: 5 minutes

- [ ] Run `cargo fmt`
- [ ] Verify `cargo fmt --check` passes
- [ ] Commit formatting changes

#### Task 1.4: Increase Test Coverage
**Effort**: 4-6 hours

- [ ] Add edge case tests for agent service
- [ ] Add concurrent access tests
- [ ] Add error path tests for all services
- [ ] Target: >80% coverage

**Acceptance Criteria**:
- ✅ All 22/22 tests passing
- ✅ Zero `.expect()` in production code
- ✅ Code formatting consistent
- ✅ Test coverage >80%

---

### 6.2 Step 2: Create Mock Implementations

**Duration**: 2-3 days (16-24 hours)

**Tasks**:

#### Task 2.1: Add mockall to Cargo.toml
**Effort**: 10 minutes

```toml
[dev-dependencies]
mockall = "0.13.1"
```

#### Task 2.2: Create FileSystem Mock
**Effort**: 2-3 hours
**Files**: `src/services/filesystem.rs`

- [ ] Add `#[cfg(test)]` mock module
- [ ] Implement `MockFilesystem` using `mockall::mock!`
- [ ] Add example usage in doc comments
- [ ] Write 5+ tests using mock
- [ ] Test error conditions (permission denied, not found, etc.)

#### Task 2.3: Create StorageService Mock
**Effort**: 2-3 hours
**Files**: `src/services/storage.rs`

- [ ] Implement `MockStorage` using mockall
- [ ] Test load/save workflows
- [ ] Test error handling (disk full, corrupt data)
- [ ] Test concurrent access

#### Task 2.4: Create ConfigService Mock
**Effort**: 2-3 hours
**Files**: `src/services/config.rs`

- [ ] Implement `MockConfig` using mockall (or manual)
- [ ] Test config loading
- [ ] Test missing environment variables
- [ ] Test custom configurations

#### Task 2.5: Create AgentService Mock
**Effort**: 3-4 hours
**Files**: `src/services/agents.rs`

- [ ] Implement `MockAgent` using mockall
- [ ] Test agent switching
- [ ] Test agent lookup (success and failure)
- [ ] Test agent registry operations

#### Task 2.6: Comprehensive Mock Testing
**Effort**: 4-6 hours

- [ ] Integration tests using mocks
- [ ] Error path coverage
- [ ] Concurrent access patterns
- [ ] Performance validation (mocks should be fast)

**Acceptance Criteria**:
- ✅ Mock implementations for all 4 service traits
- ✅ 20+ new tests using mocks
- ✅ All tests passing
- ✅ Test execution <1 second (mocks are fast)

---

### 6.3 Step 3: Implement AppBuilder

**Duration**: 3-4 days (24-32 hours)

**Tasks**:

#### Task 3.1: Create AppBuilder Skeleton
**Effort**: 2-3 hours
**Files**: `src/app_builder.rs` (new file)

- [ ] Create `AppBuilder` struct
- [ ] Add all dependency fields
- [ ] Implement `new()` constructor
- [ ] Add doc comments

#### Task 3.2: Implement `with_production_deps()`
**Effort**: 4-6 hours

- [ ] Create runtime
- [ ] Create event bus
- [ ] Initialize RealFileSystem
- [ ] Load FileConfigService from .env
- [ ] Create FileStorageService
- [ ] Add error handling with context
- [ ] Write integration test

#### Task 3.3: Implement `with_test_deps()`
**Effort**: 2-3 hours

- [ ] Create test runtime
- [ ] Create mock filesystem
- [ ] Create mock config
- [ ] Create mock storage
- [ ] Set default test API key
- [ ] Write unit test

#### Task 3.4: Implement `build()` Method
**Effort**: 8-12 hours

- [ ] Validate all dependencies present
- [ ] Create LLM adapter
- [ ] Load system instructions
- [ ] Create agent service (async)
- [ ] Build RustbotApi
- [ ] Load agent configs
- [ ] Create MCP manager
- [ ] Load MCP configuration
- [ ] Create rendering components
- [ ] Load token stats
- [ ] Create UI views
- [ ] Build RustbotApp struct
- [ ] Add error handling
- [ ] Write comprehensive tests

#### Task 3.5: Add Builder Methods
**Effort**: 2-3 hours

- [ ] `with_api_key()`
- [ ] `with_system_instructions()`
- [ ] `with_runtime()` (override)
- [ ] `with_event_bus()` (override)
- [ ] `with_config()` (override)
- [ ] `with_storage()` (override)

#### Task 3.6: Validation and Error Handling
**Effort**: 2-3 hours

- [ ] Implement `validate()` method
- [ ] Add context to all errors
- [ ] Test missing dependencies
- [ ] Test invalid configurations
- [ ] Add helpful error messages

**Acceptance Criteria**:
- ✅ AppBuilder compiles
- ✅ `with_production_deps()` works
- ✅ `with_test_deps()` works
- ✅ `build()` creates valid RustbotApp
- ✅ 10+ tests for builder
- ✅ Clear error messages

---

### 6.4 Step 4: Integrate with main.rs

**Duration**: 4-5 days (32-40 hours)

**Tasks**:

#### Task 4.1: Refactor RustbotApp Constructor
**Effort**: 4-6 hours
**Files**: `src/main.rs`

- [ ] Create `from_builder()` method
- [ ] Accept all dependencies as parameters
- [ ] Initialize UI state with defaults
- [ ] Deprecate old `new()` method
- [ ] Keep old `new()` for backward compatibility

#### Task 4.2: Update main() Function
**Effort**: 2-3 hours

- [ ] Remove hardcoded initialization
- [ ] Use AppBuilder pattern
- [ ] Add error handling
- [ ] Test application startup
- [ ] Verify UI loads correctly

#### Task 4.3: Update RustbotApp to Use Services
**Effort**: 8-12 hours

Currently, `RustbotApp` has methods like:
```rust
fn load_token_stats() -> Result<TokenStats> {
    let path = dirs::home_dir()?.join(".rustbot/token_stats.json");
    let json = std::fs::read_to_string(path)?;  // ❌ DIRECT FS
    serde_json::from_str(&json)
}
```

Refactor to:
```rust
async fn load_token_stats(storage: &dyn StorageService) -> Result<TokenStats> {
    storage.load_token_stats().await  // ✅ USE SERVICE
}
```

- [ ] Identify all filesystem access in RustbotApp
- [ ] Replace with service calls
- [ ] Update config access to use ConfigService
- [ ] Update agent access to use AgentService
- [ ] Update storage access to use StorageService
- [ ] Test all UI flows still work

#### Task 4.4: Remove Old Code Paths
**Effort**: 2-3 hours

- [ ] Remove direct `std::fs` calls
- [ ] Remove direct `AgentLoader` usage
- [ ] Remove hardcoded paths
- [ ] Keep deprecated methods for one version

#### Task 4.5: Integration Testing
**Effort**: 8-12 hours

- [ ] Manual testing of all UI features
- [ ] Agent loading
- [ ] Chat functionality
- [ ] MCP plugins
- [ ] Settings
- [ ] Token tracking
- [ ] Agent switching
- [ ] System prompts

#### Task 4.6: Documentation
**Effort**: 4-6 hours

- [ ] Update README.md
- [ ] Update DEVELOPMENT.md
- [ ] Add AppBuilder examples
- [ ] Document migration path
- [ ] Update architecture diagrams

**Acceptance Criteria**:
- ✅ Main app uses AppBuilder
- ✅ No direct filesystem access
- ✅ All UI features work
- ✅ Backward compatibility maintained
- ✅ Documentation updated

---

### 6.5 Step 5: Testing & Validation

**Duration**: 2-3 days (16-24 hours)

**Tasks**:

#### Task 5.1: Unit Tests
**Effort**: 4-6 hours

- [ ] Test AppBuilder with all combinations
- [ ] Test error handling
- [ ] Test validation
- [ ] Test mock dependencies
- [ ] Target: 100% builder coverage

#### Task 5.2: Integration Tests
**Effort**: 6-8 hours

- [ ] Test full app initialization (production)
- [ ] Test full app initialization (test mode)
- [ ] Test service interactions
- [ ] Test async operations
- [ ] Test error propagation

#### Task 5.3: UI Testing
**Effort**: 4-6 hours

- [ ] Manual testing checklist
- [ ] Test all views
- [ ] Test all agent operations
- [ ] Test MCP plugins
- [ ] Test settings persistence

#### Task 5.4: Performance Testing
**Effort**: 2-3 hours

- [ ] Measure startup time (before/after)
- [ ] Measure memory usage
- [ ] Measure test execution time
- [ ] Ensure no regressions

#### Task 5.5: Regression Testing
**Effort**: 2-3 hours

- [ ] Run full test suite
- [ ] Check all examples compile
- [ ] Verify no breaking changes
- [ ] Test backward compatibility

**Acceptance Criteria**:
- ✅ All tests passing (unit + integration)
- ✅ Test coverage >80%
- ✅ No performance regressions
- ✅ All UI features working
- ✅ Manual testing completed

---

## 7. Testing Strategy

### 7.1 Test Pyramid

```
        ┌─────────────┐
        │   Manual    │  (UI testing, exploratory)
        │  Testing    │  ~5% of effort
        └─────────────┘
       ┌───────────────┐
       │  Integration  │  (Full app initialization)
       │     Tests     │  ~25% of effort
       └───────────────┘
    ┌─────────────────────┐
    │    Unit Tests       │  (Service layer, AppBuilder)
    │   (with mocks)      │  ~70% of effort
    └─────────────────────┘
```

### 7.2 Unit Test Strategy

**Focus**: Service layer and AppBuilder

**Tools**:
- `mockall` for mock implementations
- `tokio::test` for async tests
- `tempfile` for filesystem integration tests (where needed)

**Coverage Goals**:
- AppBuilder: 100%
- Services: >90%
- Mocks: >80%

**Example Tests**:
```rust
#[tokio::test]
async fn test_app_builder_validates_missing_dependencies() {
    let result = AppBuilder::new()
        .build();  // Missing deps

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Runtime not configured"));
}

#[tokio::test]
async fn test_app_builder_production_deps() {
    let app = AppBuilder::new()
        .with_production_deps()
        .expect("Failed to load production deps")
        .with_api_key("test-key".to_string())
        .build()
        .expect("Failed to build app");

    // App should be fully initialized
    assert!(app.agent_configs.len() > 0);
}

#[test]
fn test_mock_filesystem_read_write() {
    let mut mock_fs = MockFilesystem::new();

    mock_fs.expect_write()
        .times(1)
        .returning(|_, _| Ok(()));

    mock_fs.expect_read_to_string()
        .times(1)
        .returning(|_| Ok("test content".to_string()));

    // Use mock
}
```

### 7.3 Integration Test Strategy

**Focus**: Full dependency graph

**Tests**:
- App initialization with real services
- Service interactions
- Async workflows
- Error propagation

**Example**:
```rust
#[tokio::test]
async fn test_full_app_initialization() {
    // Create temp directory for test
    let temp_dir = TempDir::new().unwrap();

    // Set up test environment
    env::set_var("OPENROUTER_API_KEY", "test-key");

    // Build app
    let app = AppBuilder::new()
        .with_production_deps()
        .unwrap()
        .build()
        .unwrap();

    // Verify initialization
    assert!(app.agent_configs.len() > 0);
    assert!(app.runtime.handle().is_some());
}
```

### 7.4 Manual Testing Checklist

- [ ] Application starts without errors
- [ ] Agent loading works (presets and custom)
- [ ] Chat functionality works
- [ ] Agent switching works
- [ ] Token tracking updates correctly
- [ ] System prompts persist
- [ ] MCP plugins load
- [ ] MCP tools callable
- [ ] Settings UI functional
- [ ] Marketplace loads
- [ ] All views accessible
- [ ] No console errors

---

## 8. Risk Assessment

### 8.1 Technical Risks

#### Risk T-1: Breaking UI Functionality
**Probability**: Medium
**Impact**: High
**Mitigation**:
- Comprehensive manual testing
- Keep old code paths temporarily
- Gradual migration
- Feature flags if needed

#### Risk T-2: Runtime/Async Complexity
**Probability**: Low
**Impact**: Medium
**Mitigation**:
- Use `Handle` instead of `Arc<Runtime>`
- Follow tokio best practices
- Comprehensive async tests

#### Risk T-3: Performance Regressions
**Probability**: Low
**Impact**: Medium
**Mitigation**:
- Performance benchmarks
- Before/after comparisons
- Profile if issues detected

#### Risk T-4: Test Coverage Gaps
**Probability**: Medium
**Impact**: Low
**Mitigation**:
- Use `cargo tarpaulin` for coverage
- Set minimum thresholds (80%)
- Review coverage reports

### 8.2 Schedule Risks

#### Risk S-1: Underestimated Complexity
**Probability**: Medium
**Impact**: Medium
**Mitigation**:
- Add 20% buffer to estimates
- Break tasks into smaller chunks
- Regular progress reviews

#### Risk S-2: Dependency on Phase 1 Fixes
**Probability**: Low
**Impact**: High
**Mitigation**:
- Fix blockers FIRST (Step 1)
- Validate fixes before proceeding
- Don't skip validation

### 8.3 Risk Mitigation Timeline

**Week 1**: Fix Phase 1 blockers, create mocks
- Risk: Can't proceed if blockers not fixed
- Mitigation: Focus on blockers first, validate thoroughly

**Week 2**: Implement AppBuilder
- Risk: Complexity higher than expected
- Mitigation: Start with simple version, iterate

**Week 3**: Integrate with main.rs
- Risk: UI breaks
- Mitigation: Gradual migration, keep old code paths

---

## 9. Success Criteria

### 9.1 Phase 2 Complete When:

**Code Quality**:
- [ ] All Phase 1 tests passing (22/22)
- [ ] All Phase 2 tests passing (50+ new tests)
- [ ] Test coverage >80% for service layer
- [ ] Zero `.expect()` in production code
- [ ] Zero clippy warnings
- [ ] Code formatted (`cargo fmt`)

**Architecture**:
- [ ] AppBuilder pattern implemented
- [ ] Main.rs using AppBuilder
- [ ] All dependencies injected (no hardcoding)
- [ ] Services decoupled from infrastructure
- [ ] Mock implementations working

**Functionality**:
- [ ] Application starts without errors
- [ ] All UI features working
- [ ] Agent loading functional
- [ ] Chat working
- [ ] MCP plugins functional
- [ ] Settings persist

**Documentation**:
- [ ] AppBuilder documented
- [ ] Migration guide written
- [ ] README updated
- [ ] DEVELOPMENT.md updated
- [ ] Architecture diagrams updated

### 9.2 Metrics

**Test Metrics**:
- Test count: >70 (22 Phase 1 + 50+ Phase 2)
- Pass rate: 100%
- Coverage: >80%
- Execution time: <5 seconds

**Code Metrics**:
- Lines added: ~2,000
- Lines removed: ~150 (hardcoded init)
- Net change: +1,850
- Complexity: Reduced (better separation)

**Quality Metrics**:
- Clippy warnings: 0
- `.expect()` calls in production: 0
- Direct filesystem access: 0
- Hardcoded dependencies: 0

---

## 10. Timeline and Estimates

### 10.1 Detailed Schedule

**Week 1: Foundation** (5 days, 40 hours)
- Day 1: Fix Phase 1 blockers (8 hours)
  - Runtime issue (3 hours)
  - `.expect()` removal (2 hours)
  - Formatting (1 hour)
  - Test coverage (2 hours)
- Day 2-3: Create mock implementations (16 hours)
  - FileSystem mock (3 hours)
  - StorageService mock (3 hours)
  - ConfigService mock (3 hours)
  - AgentService mock (4 hours)
  - Mock testing (3 hours)
- Day 4-5: Begin AppBuilder (16 hours)
  - Skeleton structure (3 hours)
  - `with_production_deps()` (6 hours)
  - `with_test_deps()` (3 hours)
  - Tests (4 hours)

**Week 2: AppBuilder** (5 days, 40 hours)
- Day 1-3: Complete AppBuilder (24 hours)
  - `build()` method (12 hours)
  - Builder methods (3 hours)
  - Validation (3 hours)
  - Testing (6 hours)
- Day 4-5: Begin main.rs integration (16 hours)
  - Refactor RustbotApp (6 hours)
  - Update main() (3 hours)
  - Initial testing (7 hours)

**Week 3: Integration** (5 days, 40 hours)
- Day 1-2: Complete main.rs integration (16 hours)
  - Replace filesystem access (8 hours)
  - Remove old code paths (3 hours)
  - Testing (5 hours)
- Day 3: Documentation (8 hours)
  - Update docs (4 hours)
  - Migration guide (4 hours)
- Day 4-5: Final testing & validation (16 hours)
  - Unit tests (6 hours)
  - Integration tests (6 hours)
  - Manual testing (4 hours)

**Total**: 15 days, 120 hours (~3 weeks)

### 10.2 Critical Path

```
Week 1:
[Fix Blockers] → [Create Mocks] → [AppBuilder Skeleton]
    ↓                ↓                    ↓
   Day 1          Day 2-3              Day 4-5

Week 2:
[Complete AppBuilder] → [Begin Integration]
         ↓                      ↓
     Day 1-3                Day 4-5

Week 3:
[Complete Integration] → [Documentation] → [Testing]
         ↓                     ↓               ↓
     Day 1-2                 Day 3          Day 4-5
```

**Bottlenecks**:
1. Phase 1 blockers MUST be fixed first
2. AppBuilder must work before integration
3. Integration testing critical before completion

### 10.3 Milestones

**Milestone 1**: Phase 1 Blockers Fixed (End of Week 1, Day 1)
- All 22 tests passing
- Zero production `.expect()` calls
- Code formatted

**Milestone 2**: Mocks Complete (End of Week 1, Day 3)
- All 4 service traits have mocks
- 20+ new tests using mocks
- All tests passing

**Milestone 3**: AppBuilder Working (End of Week 2, Day 3)
- `with_production_deps()` functional
- `with_test_deps()` functional
- `build()` creates valid app
- 10+ builder tests passing

**Milestone 4**: Main Integration Complete (End of Week 3, Day 2)
- Main.rs using AppBuilder
- All UI features working
- No direct filesystem access

**Milestone 5**: Phase 2 Complete (End of Week 3, Day 5)
- All tests passing (>70 tests)
- Documentation updated
- Ready for Phase 3

---

## 11. Rollback Plan

### 11.1 Rollback Triggers

**When to Rollback**:
1. Critical bug in production that can't be fixed quickly
2. Performance regression >20%
3. Major UI functionality broken
4. Tests failing after 2 attempts to fix

### 11.2 Rollback Strategy

**Git Strategy**:
```bash
# Create feature branch for Phase 2
git checkout -b phase2-app-builder

# Merge to main after each milestone
git checkout main
git merge phase2-app-builder

# If rollback needed
git revert <merge-commit>
```

**Backward Compatibility**:
- Keep old `RustbotApp::new()` method (deprecated)
- Old code paths functional until v0.3.0
- Feature flag for new architecture (optional)

### 11.3 Rollback Testing

- [ ] Test that deprecated `new()` still works
- [ ] Verify old code paths functional
- [ ] Document rollback procedure

---

## 12. Next Steps After Phase 2

### Phase 3: Migration and Cleanup (Week 6)
- Remove deprecated code paths
- Update all examples to use AppBuilder
- Add property-based tests
- Performance optimization

### Phase 4: Advanced Features
- Database backend option
- Cloud storage option
- Advanced mocking features
- Performance telemetry

---

## Appendix A: Code Examples

### A.1 Complete AppBuilder Example

See section 4.1 for full implementation.

### A.2 Complete main.rs Example

See section 5.3 for full implementation.

### A.3 Mock Implementation Example

See section 3 for full mock implementations.

---

## Appendix B: References

- [Rust Architecture Best Practices](../architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)
- [Testing Methods](../../qa/TESTING_METHODS.md)
- [Phase 1 Implementation Summary](./PHASE1_IMPLEMENTATION_SUMMARY.md)
- [Refactoring Plan](./RUSTBOT_REFACTORING_PLAN.md)
- [Refactoring Checklist](./REFACTORING_CHECKLIST.md)
- [QA Validation Report](../../qa/QA_VALIDATION_REPORT.md)

---

**Document Version**: 1.0
**Last Updated**: January 17, 2025
**Author**: AI Assistant (Claude Sonnet 4.5)
**Status**: Ready for Implementation
**Estimated Completion**: 3 weeks from start

---

## Document Changelog

- **2025-01-17**: Initial version created based on Phase 1 analysis
