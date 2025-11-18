# AppBuilder Pattern Guide

## Overview

AppBuilder provides a flexible, testable way to construct Rustbot application dependencies using the builder pattern with dependency injection. It enables clean separation between production and test configurations while maintaining type safety and validation.

## Architecture

### Builder Pattern Benefits

1. **Testability**: Easy to inject mocks for unit testing
2. **Flexibility**: Override specific dependencies without affecting others
3. **Type Safety**: Compile-time validation of dependency types
4. **Clarity**: Explicit dependency construction and validation
5. **Maintainability**: Single place to wire dependencies

### Components

```
AppBuilder
├── Configuration (API key, base path, system instructions)
├── Infrastructure (Runtime, EventBus, LLM Adapter)
└── Services (FileSystem, Storage, Config, AgentService)
    ↓
AppDependencies (Container for all wired dependencies)
```

## Basic Usage

### Production Configuration

```rust
use rustbot::{AppBuilder, Result};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let deps = AppBuilder::new()
        .with_api_key(std::env::var("OPENROUTER_API_KEY")?)
        .with_base_path(PathBuf::from("."))
        .with_system_instructions("Your system instructions".to_string())
        .with_production_deps()
        .await?
        .build()?;

    // Use dependencies
    let current_agent = deps.agent_service.current_agent();
    let agents = deps.agent_service.list_agents();

    println!("Current agent: {}", current_agent.id());
    println!("Available agents: {:?}", agents);

    Ok(())
}
```

### Test Configuration

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustbot::services::mocks::test_helpers::*;

    #[tokio::test]
    async fn test_with_mocks() {
        // Use mock implementations for all dependencies
        let deps = AppBuilder::new()
            .with_test_deps()
            .with_api_key("test_key".to_string())
            .with_agent_service(create_mock_agent_service().await)
            .build()
            .unwrap();

        // Test with mocked dependencies (no real I/O)
        let agents = deps.agent_service.list_agents();
        assert!(!agents.is_empty());
    }
}
```

### Custom Overrides

You can override specific dependencies while keeping others from production or test modes:

```rust
use std::sync::Arc;

// Create custom storage implementation
let custom_storage = Arc::new(MyCustomStorage::new());

let deps = AppBuilder::new()
    .with_production_deps()
    .await?
    .with_storage(custom_storage)  // Override just storage
    .build()?;
```

## Configuration Options

### Required Configuration

- **API Key**: Required for production mode
  ```rust
  .with_api_key("sk-or-v1-...".to_string())
  ```

### Optional Configuration

- **Base Path**: Set the base directory for file operations (default: ".")
  ```rust
  .with_base_path(PathBuf::from("/path/to/data"))
  ```

- **System Instructions**: Set global system instructions for agents
  ```rust
  .with_system_instructions("Custom instructions".to_string())
  ```

### Dependency Modes

#### Production Mode

Creates real implementations:
- RealFileSystem (actual file I/O)
- FileStorageService (persistent storage)
- FileConfigService (loads from JSON files)
- DefaultAgentService (loads agents from config)
- OpenRouter LLM adapter

```rust
let builder = AppBuilder::new()
    .with_api_key(api_key)
    .with_production_deps()
    .await?;
```

#### Test Mode

Creates mock implementations:
- MockFileSystem (in-memory, no I/O)
- MockStorageService (in-memory data)
- MockConfigService (predefined values)
- MockAgentService (manually injected)

```rust
let builder = AppBuilder::new()
    .with_test_deps()
    .with_agent_service(create_mock_agent_service().await);
```

## Dependency Override Methods

Override individual dependencies for testing or customization:

```rust
// Override filesystem
.with_filesystem(Arc::new(CustomFileSystem::new()))

// Override storage
.with_storage(Arc::new(CustomStorage::new()))

// Override config
.with_config(Arc::new(CustomConfig::new()))

// Override agent service
.with_agent_service(Arc::new(CustomAgentService::new()))

// Override event bus
.with_event_bus(Arc::new(EventBus::with_capacity(2000)))

// Override runtime
.with_runtime(Arc::new(tokio::runtime::Runtime::new()?))
```

## Error Handling

### Build-Time Validation

The `build()` method validates that all required dependencies are configured:

```rust
let result = AppBuilder::new().build();

match result {
    Ok(deps) => {
        // All dependencies present
    }
    Err(RustbotError::ConfigError(msg)) => {
        // Missing dependency
        eprintln!("Configuration error: {}", msg);
    }
    Err(e) => {
        // Other error
        eprintln!("Build error: {}", e);
    }
}
```

### Common Errors

1. **Missing API Key** (production mode)
   ```
   Error: Configuration error: API key required
   ```

2. **Missing Dependency**
   ```
   Error: Configuration error: Agent service not configured
   ```

3. **Agent Loading Failure**
   ```
   Error: Configuration error: No agents loaded from configuration
   ```

4. **Invalid Agent Directory**
   ```
   Error: IO error: No such file or directory
   ```

## AppDependencies Usage

The `AppDependencies` struct provides access to all wired services:

```rust
let deps = builder.build()?;

// Access services
deps.filesystem       // Arc<dyn FileSystem>
deps.storage          // Arc<dyn StorageService>
deps.config           // Arc<dyn ConfigService>
deps.agent_service    // Arc<dyn AgentService>
deps.runtime          // Arc<tokio::runtime::Runtime>
deps.event_bus        // Arc<EventBus>
deps.llm_adapter      // Option<Arc<dyn LlmAdapter>>

// Example: Use agent service
let current = deps.agent_service.current_agent();
println!("Current agent: {}", current.id());

// Example: Use event bus
let mut rx = deps.event_bus.subscribe();
```

## Testing Patterns

### Pattern 1: Full Mock Setup

```rust
#[tokio::test]
async fn test_full_mock_setup() {
    use rustbot::services::mocks::test_helpers::*;

    let deps = AppBuilder::new()
        .with_test_deps()
        .with_api_key("test".to_string())
        .with_agent_service(create_mock_agent_service().await)
        .build()
        .unwrap();

    // All dependencies are mocked
    let agents = deps.agent_service.list_agents();
    assert_eq!(agents.len(), 2);
}
```

### Pattern 2: Selective Override

```rust
#[tokio::test]
async fn test_selective_override() {
    use rustbot::services::mocks::test_helpers::*;

    // Create custom mock for specific test
    let mut mock_config = MockConfigService::new();
    mock_config
        .expect_load_agent_configs()
        .returning(|| Ok(vec![/* custom configs */]));

    let deps = AppBuilder::new()
        .with_test_deps()
        .with_config(Arc::new(mock_config))  // Override just config
        .with_api_key("test".to_string())
        .with_agent_service(create_mock_agent_service().await)
        .build()
        .unwrap();

    // Test with custom config mock
}
```

### Pattern 3: Integration Testing with Real Filesystem

```rust
#[tokio::test]
async fn test_with_real_filesystem() {
    let temp_dir = tempfile::tempdir().unwrap();

    let deps = AppBuilder::new()
        .with_api_key("test-key".to_string())
        .with_base_path(temp_dir.path().to_path_buf())
        .with_production_deps()
        .await
        .unwrap()
        .build()
        .unwrap();

    // Test with real filesystem in temporary directory
}
```

## Design Decisions

### Why Builder Pattern?

**Rationale**: Builder pattern provides method chaining for readable configuration while maintaining type safety and validation.

**Alternatives Considered**:
1. **Direct Construction**: Rejected due to lack of flexibility and testability
2. **Configuration Struct**: Rejected due to verbose initialization syntax
3. **Factory Functions**: Rejected due to proliferation of factory variants

**Trade-offs**:
- **Pro**: Clear, readable configuration with compile-time validation
- **Pro**: Easy to add new configuration options without breaking existing code
- **Con**: More verbose than direct construction
- **Con**: Requires two-step process (configure → build)

### Why Separate Production/Test Modes?

**Rationale**: Test mode should completely eliminate I/O and external dependencies for fast, reliable unit tests.

**Benefits**:
- Tests run in milliseconds (no filesystem/network)
- Tests are deterministic (no external state)
- Tests can run in parallel without conflicts
- Easy to set up test scenarios with mocks

### Why Arc for All Dependencies?

**Rationale**: Services need to be shared across multiple components (UI, API, agents) and potentially across threads.

**Benefits**:
- Thread-safe shared ownership
- No lifetime annotation complexity
- Easy to clone for passing to async tasks
- Prevents accidental mutations (interior mutability when needed)

## Integration with Main Application

### Example main.rs Integration

```rust
use rustbot::{AppBuilder, Result};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let api_key = env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY must be set");

    // Build dependencies
    let deps = AppBuilder::new()
        .with_api_key(api_key)
        .with_base_path(PathBuf::from("."))
        .with_system_instructions(load_system_instructions())
        .with_production_deps()
        .await?
        .build()?;

    // Initialize UI with dependencies
    let app = RustbotApp::new(deps);

    // Run application
    app.run()?;

    Ok(())
}

fn load_system_instructions() -> String {
    std::fs::read_to_string("system_instructions.txt")
        .unwrap_or_default()
}
```

## Troubleshooting

### Issue: "Filesystem not configured"

**Cause**: `build()` called without setting up dependencies.

**Solution**: Call `.with_production_deps()` or `.with_test_deps()` before `build()`.

### Issue: "API key required"

**Cause**: Production mode requires API key for LLM adapter.

**Solution**: Provide API key before calling `.with_production_deps()`:
```rust
.with_api_key(api_key)
.with_production_deps()
```

### Issue: "No agents loaded from configuration"

**Cause**: Agent config directory is empty or invalid.

**Solution**: Ensure `agents/presets/` contains valid agent JSON files.

### Issue: Test hangs or deadlocks

**Cause**: Async runtime not properly configured in tests.

**Solution**: Use `#[tokio::test]` attribute and ensure runtime is created:
```rust
#[tokio::test]
async fn my_test() {
    // Runtime is automatically created by tokio::test
}
```

## Performance Considerations

### Startup Time

- **Production Mode**: ~100-500ms to load agents and initialize services
- **Test Mode**: <10ms to create mock implementations

### Memory Usage

- Each dependency uses Arc, so cloning is cheap (pointer copy)
- Mock implementations use minimal memory (no real data)
- Production mode memory depends on number of agents and config size

### Thread Safety

- All services implement `Send + Sync` for concurrent access
- EventBus uses broadcast channels (lock-free for publishers)
- Agent service uses Arc for zero-cost cloning

## Future Enhancements

1. **Configuration Validation**: Add compile-time validation of configuration completeness
2. **Lazy Initialization**: Support lazy loading of agents on first use
3. **Hot Reload**: Support runtime reconfiguration without restart
4. **Metrics**: Add instrumentation for dependency creation and usage
5. **Multiple LLM Adapters**: Support multiple LLM providers simultaneously

## Related Documentation

- [Service Layer Architecture](../SERVICE_LAYER.md)
- [Testing Guide](../../testing/TESTING_GUIDE.md)
- [Dependency Injection Patterns](../patterns/DEPENDENCY_INJECTION.md)

---

*Last Updated: 2025-11-17*
*Version: 1.0.0*
