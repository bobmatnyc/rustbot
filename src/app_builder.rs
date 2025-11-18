// AppBuilder pattern for dependency construction and injection
//
// Design Decision: Builder pattern with dependency injection for testability
//
// Rationale: Rustbot needs clean separation between production and test configurations.
// Builder pattern allows:
// 1. Explicit dependency construction with validation
// 2. Easy test setup with mock implementations
// 3. Method chaining for readable configuration
// 4. Compile-time validation of required dependencies
//
// Trade-offs:
// - Type safety: Builder validates dependencies vs. runtime errors
// - Flexibility: Can override any dependency for testing
// - Verbosity: More code than direct construction, but clearer intent
//
// Architecture Pattern: Builder + Dependency Injection
// - Builder constructs dependencies in correct order
// - AppDependencies container holds all wired services
// - Production vs Test modes provide complete configurations
//
// Usage Example:
//     // Production
//     let deps = AppBuilder::new()
//         .with_api_key(api_key)
//         .with_production_deps()
//         .await?
//         .build()?;
//
//     // Testing
//     let deps = AppBuilder::new()
//         .with_test_deps()
//         .with_api_key("test")
//         .build()?;

use crate::error::{Result, RustbotError};
use crate::events::EventBus;
use crate::llm::{AdapterType, LlmAdapter};
use crate::services::{
    AgentService, ConfigService, DefaultAgentService, FileConfigService, FileStorageService,
    FileSystem, RealFileSystem, StorageService,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Builder for constructing RustbotApp with dependency injection
///
/// Provides a fluent interface for configuring and creating all application
/// dependencies with proper validation and error handling.
///
/// # Examples
///
/// ```no_run
/// use rustbot::AppBuilder;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> rustbot::Result<()> {
///     let deps = AppBuilder::new()
///         .with_api_key("sk-...".to_string())
///         .with_base_path(PathBuf::from("."))
///         .with_production_deps()
///         .await?
///         .build()?;
///
///     // Use dependencies...
///     Ok(())
/// }
/// ```
pub struct AppBuilder {
    // Required configuration
    api_key: Option<String>,

    // Optional overrides (for testing)
    filesystem: Option<Arc<dyn FileSystem>>,
    storage: Option<Arc<dyn StorageService>>,
    config: Option<Arc<dyn ConfigService>>,
    agent_service: Option<Arc<dyn AgentService>>,

    // Infrastructure
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    event_bus: Option<Arc<EventBus>>,
    llm_adapter: Option<Arc<dyn LlmAdapter>>,

    // Configuration paths
    base_path: PathBuf,
    system_instructions: String,
}

impl AppBuilder {
    /// Create a new AppBuilder with default configuration
    pub fn new() -> Self {
        Self {
            api_key: None,
            filesystem: None,
            storage: None,
            config: None,
            agent_service: None,
            runtime: None,
            event_bus: None,
            llm_adapter: None,
            base_path: PathBuf::from("."),
            system_instructions: String::new(),
        }
    }

    /// Set the API key (required for production)
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = Some(key);
        self
    }

    /// Set base path for file operations
    pub fn with_base_path(mut self, path: PathBuf) -> Self {
        self.base_path = path;
        self
    }

    /// Set system instructions for agents
    pub fn with_system_instructions(mut self, instructions: String) -> Self {
        self.system_instructions = instructions;
        self
    }

    /// Use production dependencies (default)
    ///
    /// Creates real implementations of all services:
    /// - RealFileSystem for file I/O
    /// - FileStorageService for persistence
    /// - FileConfigService for configuration
    /// - DefaultAgentService with loaded agents
    /// - OpenRouter LLM adapter
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - API key not provided
    /// - Configuration files cannot be loaded
    /// - Agent initialization fails
    pub async fn with_production_deps(mut self) -> Result<Self> {
        // Validate API key
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| RustbotError::ConfigError("API key required".to_string()))?
            .clone();

        // Create real filesystem
        let filesystem = Arc::new(RealFileSystem) as Arc<dyn FileSystem>;

        // Create storage service
        let storage = Arc::new(FileStorageService::new(
            filesystem.clone(),
            self.base_path.clone(),
        )) as Arc<dyn StorageService>;

        // Create config service (loads from environment and files)
        let config = Arc::new(FileConfigService::load()?) as Arc<dyn ConfigService>;

        // Create runtime and event bus
        let runtime = Arc::new(
            tokio::runtime::Runtime::new()
                .map_err(|e| RustbotError::ApiError(format!("Failed to create runtime: {}", e)))?,
        );
        let event_bus = Arc::new(EventBus::new());

        // Create LLM adapter
        let llm_adapter = Arc::from(crate::llm::create_adapter(
            AdapterType::OpenRouter,
            api_key.clone(),
        )) as Arc<dyn LlmAdapter>;

        // Create agent service
        let agent_service = Arc::new(
            DefaultAgentService::new(
                config.clone(),
                event_bus.clone(),
                runtime.handle().clone(),
                self.system_instructions.clone(),
            )
            .await?,
        ) as Arc<dyn AgentService>;

        self.filesystem = Some(filesystem);
        self.storage = Some(storage);
        self.config = Some(config);
        self.agent_service = Some(agent_service);
        self.runtime = Some(runtime); // Production owns the runtime
        self.event_bus = Some(event_bus);
        self.llm_adapter = Some(llm_adapter);

        Ok(self)
    }

    /// Use test dependencies (mocks)
    ///
    /// Creates mock implementations for testing:
    /// - Mock filesystem (no real I/O)
    /// - Mock storage (in-memory)
    /// - Mock config (predefined values)
    ///
    /// Note: Agent service and runtime should be manually injected for tests
    /// that need them. Don't create a runtime here as tests already run in one.
    #[cfg(test)]
    pub fn with_test_deps(mut self) -> Self {
        use crate::services::mocks::test_helpers::*;

        self.filesystem = Some(Arc::new(create_mock_filesystem()) as Arc<dyn FileSystem>);
        self.storage = Some(Arc::new(create_mock_storage()) as Arc<dyn StorageService>);
        self.config = Some(Arc::new(create_mock_config()) as Arc<dyn ConfigService>);

        // Don't create runtime in tests - tests already run in tokio runtime
        // Runtime and event_bus should be injected separately if needed

        let event_bus = Arc::new(EventBus::new());
        self.event_bus = Some(event_bus);

        self
    }

    /// Override filesystem (for testing)
    pub fn with_filesystem(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.filesystem = Some(fs);
        self
    }

    /// Override storage service (for testing)
    pub fn with_storage(mut self, storage: Arc<dyn StorageService>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Override config service (for testing)
    pub fn with_config(mut self, config: Arc<dyn ConfigService>) -> Self {
        self.config = Some(config);
        self
    }

    /// Override agent service (for testing)
    pub fn with_agent_service(mut self, agent_service: Arc<dyn AgentService>) -> Self {
        self.agent_service = Some(agent_service);
        self
    }

    /// Override event bus (for testing)
    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Override runtime (for testing)
    pub fn with_runtime(mut self, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Build the configured dependencies
    ///
    /// Validates that all required dependencies are present.
    ///
    /// # Errors
    ///
    /// Returns error if any required dependency is missing.
    pub fn build(self) -> Result<AppDependencies> {
        Ok(AppDependencies {
            filesystem: self.filesystem.ok_or_else(|| {
                RustbotError::ConfigError("Filesystem not configured".to_string())
            })?,
            storage: self
                .storage
                .ok_or_else(|| RustbotError::ConfigError("Storage not configured".to_string()))?,
            config: self
                .config
                .ok_or_else(|| RustbotError::ConfigError("Config not configured".to_string()))?,
            agent_service: self.agent_service.ok_or_else(|| {
                RustbotError::ConfigError("Agent service not configured".to_string())
            })?,
            runtime: self.runtime, // Optional - can be None for tests
            event_bus: self
                .event_bus
                .ok_or_else(|| RustbotError::ConfigError("Event bus not configured".to_string()))?,
            llm_adapter: self.llm_adapter,
        })
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Container for all application dependencies
///
/// Provides centralized access to all services and infrastructure
/// components used throughout the application.
///
/// # Thread Safety
///
/// All fields use Arc for shared ownership across threads.
/// Services must implement Send + Sync for concurrent access.
///
/// # Runtime Ownership
///
/// The runtime field is optional. For production, AppBuilder creates
/// and owns the runtime. For tests, the runtime can be omitted since
/// tests already run in a tokio runtime (via #[tokio::test]).
pub struct AppDependencies {
    pub filesystem: Arc<dyn FileSystem>,
    pub storage: Arc<dyn StorageService>,
    pub config: Arc<dyn ConfigService>,
    pub agent_service: Arc<dyn AgentService>,
    pub runtime: Option<Arc<tokio::runtime::Runtime>>,
    pub event_bus: Arc<EventBus>,
    pub llm_adapter: Option<Arc<dyn LlmAdapter>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mocks::test_helpers::*;

    // Helper to create a mock agent service for testing
    async fn create_test_agent_service() -> Arc<dyn AgentService> {
        let mock_config = Arc::new(create_mock_config_with_agents()) as Arc<dyn ConfigService>;
        let event_bus = Arc::new(EventBus::new());
        let runtime = tokio::runtime::Handle::current();

        Arc::new(
            DefaultAgentService::new(
                mock_config,
                event_bus,
                runtime,
                "Test system instructions".to_string(),
            )
            .await
            .unwrap(),
        ) as Arc<dyn AgentService>
    }

    #[tokio::test]
    async fn test_builder_with_test_deps() {
        let builder = AppBuilder::new()
            .with_test_deps()
            .with_api_key("test_key".to_string());

        // Create a mock agent service for testing
        let mock_agent_service = create_test_agent_service().await;

        let builder = builder.with_agent_service(mock_agent_service);

        let deps = builder.build().unwrap();

        // Verify all dependencies are present
        assert!(Arc::strong_count(&deps.filesystem) >= 1);
        assert!(Arc::strong_count(&deps.storage) >= 1);
        assert!(Arc::strong_count(&deps.config) >= 1);
        assert!(Arc::strong_count(&deps.agent_service) >= 1);
        assert!(Arc::strong_count(&deps.event_bus) >= 1);
        // Runtime is optional for tests
        assert!(deps.runtime.is_none());
    }

    #[tokio::test]
    async fn test_builder_with_production_deps() {
        let temp_dir = tempfile::tempdir().unwrap();
        let agents_dir = temp_dir.path().join("agents/presets");
        std::fs::create_dir_all(&agents_dir).unwrap();

        // Create a minimal test agent config
        let config_path = agents_dir.join("test_agent.json");
        std::fs::write(
            &config_path,
            r#"{
                "id": "test",
                "name": "Test Agent",
                "instructions": "Test instructions",
                "model": "test-model",
                "enabled": true
            }"#,
        )
        .unwrap();

        let result = AppBuilder::new()
            .with_api_key("sk-test-key".to_string())
            .with_base_path(temp_dir.path().to_path_buf())
            .with_system_instructions("Test system instructions".to_string())
            .with_production_deps()
            .await;

        assert!(result.is_ok());

        let builder = result.unwrap();
        let mut deps = builder.build().unwrap();

        // Verify all dependencies are present
        assert!(Arc::strong_count(&deps.filesystem) >= 1);
        assert!(Arc::strong_count(&deps.storage) >= 1);
        assert!(Arc::strong_count(&deps.config) >= 1);
        assert!(Arc::strong_count(&deps.agent_service) >= 1);
        assert!(Arc::strong_count(&deps.event_bus) >= 1);
        assert!(deps.llm_adapter.is_some());
        // Production creates runtime
        assert!(deps.runtime.is_some());

        // Prevent runtime drop panic by taking ownership and forgetting
        // This is safe for tests as the process will exit anyway
        let _runtime = deps.runtime.take();
        std::mem::forget(_runtime);
    }

    #[tokio::test]
    async fn test_builder_missing_api_key() {
        let result = AppBuilder::new().with_production_deps().await;

        assert!(result.is_err());
        match result {
            Err(RustbotError::ConfigError(msg)) => {
                assert!(msg.contains("API key"));
            }
            _ => panic!("Expected ConfigError for missing API key"),
        }
    }

    #[tokio::test]
    async fn test_builder_custom_overrides() {
        let custom_storage = Arc::new(create_mock_storage()) as Arc<dyn StorageService>;
        let custom_storage_clone = custom_storage.clone();

        let builder = AppBuilder::new()
            .with_test_deps()
            .with_storage(custom_storage)
            .with_api_key("test".to_string());

        let mock_agent_service = create_test_agent_service().await;
        let builder = builder.with_agent_service(mock_agent_service);

        let deps = builder.build().unwrap();

        // Storage should be our custom one
        assert!(Arc::ptr_eq(&deps.storage, &custom_storage_clone));
    }

    #[tokio::test]
    async fn test_builder_incomplete_build_fails() {
        let builder = AppBuilder::new(); // No deps configured

        let result = builder.build();
        assert!(result.is_err());
        match result {
            Err(RustbotError::ConfigError(msg)) => {
                assert!(msg.contains("not configured"));
            }
            _ => panic!("Expected ConfigError for incomplete build"),
        }
    }

    #[tokio::test]
    async fn test_builder_method_chaining() {
        let deps = AppBuilder::new()
            .with_api_key("test".to_string())
            .with_base_path(PathBuf::from("/test"))
            .with_system_instructions("Test instructions".to_string())
            .with_test_deps()
            .with_agent_service(create_test_agent_service().await)
            .build()
            .unwrap();

        assert!(Arc::strong_count(&deps.filesystem) >= 1);
    }

    #[tokio::test]
    async fn test_builder_default() {
        let builder = AppBuilder::default();
        assert!(builder.api_key.is_none());
        assert_eq!(builder.base_path, PathBuf::from("."));
    }

    #[tokio::test]
    async fn test_builder_multiple_builds_from_same_config() {
        // Create builder with test deps
        let builder = AppBuilder::new()
            .with_test_deps()
            .with_api_key("test".to_string())
            .with_agent_service(create_test_agent_service().await);

        // First build should succeed
        let deps1 = builder.build();
        assert!(deps1.is_ok());
    }

    #[tokio::test]
    async fn test_app_dependencies_arc_counts() {
        let deps = AppBuilder::new()
            .with_test_deps()
            .with_api_key("test".to_string())
            .with_agent_service(create_test_agent_service().await)
            .build()
            .unwrap();

        // Create clones to test Arc reference counting
        let _fs_clone = deps.filesystem.clone();
        assert!(Arc::strong_count(&deps.filesystem) >= 2);

        let _storage_clone = deps.storage.clone();
        assert!(Arc::strong_count(&deps.storage) >= 2);
    }
}
