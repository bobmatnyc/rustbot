# Phase 2 Quick Reference - AppBuilder & Dependency Injection

**Quick lookup guide for developers using Phase 2 architecture**

**Last Updated**: 2025-01-17
**Status**: Phase 2 Complete ✅

---

## Table of Contents

1. [AppBuilder Usage](#appbuilder-usage)
2. [Service Layer API](#service-layer-api)
3. [Testing Patterns](#testing-patterns)
4. [Common Tasks](#common-tasks)
5. [Troubleshooting](#troubleshooting)

---

## AppBuilder Usage

### Production Configuration

```rust
use rustbot::app_builder::AppBuilder;

fn main() -> Result<()> {
    let deps = AppBuilder::new()
        .with_production_deps()
        .with_api_key("your-api-key")
        .with_agents_dir(PathBuf::from("./agents"))
        .build()?;

    let app = RustbotApp::new(deps)?;
    // Run app...
}
```

### Test Configuration

```rust
#[tokio::test]
async fn test_something() {
    let deps = AppBuilder::new()
        .with_test_doubles()
        .build()
        .unwrap();

    // All dependencies are mocks
    // Test your code...
}
```

### Custom Overrides

```rust
let custom_fs = Arc::new(MyCustomFileSystem::new());

let deps = AppBuilder::new()
    .with_test_doubles()
    .with_custom_filesystem(custom_fs)
    .build()
    .unwrap();

// Uses custom filesystem, other deps are mocks
```

---

## Service Layer API

### FileSystem Trait

```rust
#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_to_string(&self, path: &Path) -> Result<String>;
    async fn write(&self, path: &Path, content: &str) -> Result<()>;
    async fn exists(&self, path: &Path) -> bool;
    async fn create_dir_all(&self, path: &Path) -> Result<()>;
    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
}
```

**Implementations**: `RealFileSystem`, `MockFileSystem`

**Example**:
```rust
let content = deps.filesystem.read_to_string(
    Path::new("config.json")
).await?;
```

### StorageService Trait

```rust
#[async_trait]
pub trait StorageService: Send + Sync {
    async fn load_token_stats(&self) -> Result<TokenStats>;
    async fn save_token_stats(&self, stats: &TokenStats) -> Result<()>;
    async fn load_system_prompts(&self) -> Result<SystemPrompts>;
    async fn save_system_prompts(&self, prompts: &SystemPrompts) -> Result<()>;
}
```

**Implementations**: `FileStorageService`, `MockStorageService`

**Example**:
```rust
let stats = deps.storage.load_token_stats().await?;
deps.storage.save_token_stats(&updated_stats).await?;
```

### AgentService Trait

```rust
#[async_trait]
pub trait AgentService: Send + Sync {
    async fn get_agent(&self, id: &str) -> Result<Arc<Agent>>;
    fn list_agents(&self) -> Vec<String>;
    async fn switch_agent(&mut self, id: &str) -> Result<()>;
    fn current_agent(&self) -> Arc<Agent>;
}
```

**Implementations**: `DefaultAgentService`, `MockAgentService`

**Example**:
```rust
let agent = deps.agents.read().await
    .get_agent("assistant").await?;

let agent_list = deps.agents.read().await
    .list_agents();
```

### ConfigService Trait

```rust
#[async_trait]
pub trait ConfigService: Send + Sync {
    async fn load_agent_configs(&self) -> Result<Vec<AgentConfig>>;
    fn get_api_key(&self) -> Result<String>;
    fn get_model(&self) -> String;
    fn get_agents_dir(&self) -> PathBuf;
}
```

**Implementations**: `FileConfigService`, `MockConfigService`

**Example**:
```rust
let api_key = deps.config.get_api_key()?;
let agent_configs = deps.config.load_agent_configs().await?;
```

---

## Testing Patterns

### Unit Testing with Mocks

```rust
use mockall::predicate::*;

#[tokio::test]
async fn test_my_feature() {
    // Setup mock
    let mut mock_fs = MockFileSystem::new();
    mock_fs.expect_read_to_string()
        .with(eq(Path::new("test.json")))
        .times(1)
        .returning(|_| Ok(r#"{"test":true}"#.to_string()));

    // Use mock
    let service = MyService::new(Arc::new(mock_fs));
    let result = service.do_something().await;

    assert!(result.is_ok());
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_workflow() {
    // Use test doubles
    let deps = AppBuilder::new()
        .with_test_doubles()
        .build()
        .unwrap();

    // Test full workflow
    deps.agents.write().await.register(
        "test".to_string(),
        Agent::new(AgentConfig::default())
    ).unwrap();

    let agent = deps.agents.read().await
        .get_agent("test").await.unwrap();

    assert_eq!(agent.id(), "test");
}
```

### Error Simulation

```rust
#[tokio::test]
async fn test_error_handling() {
    let mut mock_storage = MockStorageService::new();

    // Simulate error
    mock_storage.expect_load_token_stats()
        .returning(|| Err(RustbotError::FileNotFound("stats.json".to_string())));

    let deps = AppBuilder::new()
        .with_test_doubles()
        .with_custom_storage(Arc::new(mock_storage))
        .build()
        .unwrap();

    // Test error handling
    let result = deps.storage.load_token_stats().await;
    assert!(result.is_err());
}
```

### Single-Threaded Tests (for AppBuilder)

```bash
# Run AppBuilder tests single-threaded to avoid flaky test
cargo test --lib app_builder:: -- --test-threads=1
```

---

## Common Tasks

### Task: Add New Service

1. **Define Trait**:
```rust
#[async_trait]
pub trait MyService: Send + Sync {
    async fn do_something(&self) -> Result<Data>;
}
```

2. **Implement Production**:
```rust
pub struct RealMyService {
    deps: Arc<dyn OtherService>,
}

#[async_trait]
impl MyService for RealMyService {
    async fn do_something(&self) -> Result<Data> {
        // Implementation
    }
}
```

3. **Add to AppBuilder**:
```rust
pub struct AppDependencies {
    // ... existing fields
    pub my_service: Arc<dyn MyService>,
}

impl AppBuilder {
    pub fn with_my_service(mut self, service: Arc<dyn MyService>) -> Self {
        self.my_service = Some(service);
        self
    }
}
```

4. **Create Mock**:
```rust
#[cfg(test)]
use mockall::*;

#[automock]
#[async_trait]
pub trait MyService: Send + Sync {
    async fn do_something(&self) -> Result<Data>;
}
```

5. **Write Tests**:
```rust
#[tokio::test]
async fn test_my_service() {
    let mut mock = MockMyService::new();
    mock.expect_do_something()
        .returning(|| Ok(Data::default()));

    // Test code
}
```

### Task: Run Tests

```bash
# All tests
cargo test

# Service layer only
cargo test --lib services::

# AppBuilder only (single-threaded)
cargo test --lib app_builder:: -- --test-threads=1

# Examples only
cargo test --examples

# Specific test
cargo test test_name
```

### Task: Access Dependencies in App

```rust
impl RustbotApp {
    pub fn new(deps: AppDependencies) -> Result<Self> {
        // Access filesystem
        let config_data = deps.runtime.block_on(async {
            deps.filesystem.read_to_string(
                Path::new("config.json")
            ).await
        })?;

        // Access storage
        let stats = deps.runtime.block_on(async {
            deps.storage.load_token_stats().await
        })?;

        // Access agents
        let agent = deps.agents.read()
            .map(|guard| guard.current_agent())?;

        // Access config
        let api_key = deps.config.get_api_key()?;

        Ok(Self { deps, /* ... */ })
    }
}
```

### Task: Use Runtime

```rust
// Execute async code from sync context
let result = deps.runtime.block_on(async {
    deps.storage.load_token_stats().await
})?;

// Spawn background task
let storage = deps.storage.clone();
deps.runtime.spawn(async move {
    storage.save_token_stats(&stats).await
});
```

---

## Troubleshooting

### Issue: "Cannot start a runtime from within a runtime"

**Problem**: Trying to create `Runtime::new()` inside `#[tokio::test]`

**Solution**: Use existing tokio::test runtime or separate construction from initialization:

```rust
// ❌ WRONG:
#[tokio::test]
async fn test() {
    let runtime = Runtime::new().unwrap(); // Error!
    // ...
}

// ✅ CORRECT:
#[tokio::test]
async fn test() {
    // Use existing runtime
    let service = MyService::new(/* ... */);
    service.initialize().await.unwrap();
    // ...
}
```

### Issue: Mock expectations not met

**Problem**: Test fails with "expectation not satisfied"

**Solution**: Ensure `.times()` matches actual calls:

```rust
// ❌ WRONG:
mock.expect_read()
    .times(1)  // Expects exactly 1 call
    .returning(|| Ok(data));

// Code calls it 2 times - test fails!

// ✅ CORRECT:
mock.expect_read()
    .times(2)  // Matches actual calls
    .returning(|| Ok(data));

// Or use .times(..) for any number
mock.expect_read()
    .times(..)
    .returning(|| Ok(data));
```

### Issue: Flaky AppBuilder test

**Problem**: `test_builder_with_production_deps` fails intermittently

**Solution**: Run single-threaded:

```bash
cargo test --lib app_builder:: -- --test-threads=1
```

**Reason**: Race condition when multiple tests create tokio runtimes in parallel

### Issue: "trait object is not thread-safe"

**Problem**: `Arc<dyn Trait>` fails to compile

**Solution**: Add `Send + Sync` bounds to trait:

```rust
// ❌ WRONG:
pub trait MyService {
    fn do_something(&self);
}

// ✅ CORRECT:
pub trait MyService: Send + Sync {
    fn do_something(&self);
}
```

### Issue: Test cannot access `deps` fields

**Problem**: `deps.service` is private

**Solution**: All `AppDependencies` fields are public:

```rust
pub struct AppDependencies {
    pub filesystem: Arc<dyn FileSystem>,  // pub!
    pub storage: Arc<dyn StorageService>,  // pub!
    // ...
}
```

### Issue: Async function in non-async context

**Problem**: Cannot call async function from `main()` or non-async function

**Solution**: Use `deps.runtime.block_on()`:

```rust
fn non_async_function(deps: &AppDependencies) -> Result<()> {
    let data = deps.runtime.block_on(async {
        deps.storage.load_token_stats().await
    })?;

    Ok(())
}
```

---

## Quick Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run
cargo run --example app_builder_usage

# Test
cargo test                              # All tests
cargo test --lib services::             # Service tests
cargo test --lib app_builder::          # AppBuilder tests
cargo test --examples                   # Example tests
cargo test -- --test-threads=1          # Single-threaded
cargo test test_name                    # Specific test

# Format and Lint
cargo fmt --all
cargo clippy --all-targets

# Documentation
cargo doc --open
```

---

## File Locations

```
rustbot/
├── src/
│   ├── app_builder.rs              # AppBuilder pattern
│   ├── main.rs                      # Production entry point
│   ├── lib.rs                       # Library exports
│   └── services/
│       ├── mod.rs                   # Service module
│       ├── traits.rs                # Trait definitions
│       ├── filesystem.rs            # Filesystem service
│       ├── storage.rs               # Storage service
│       ├── agents.rs                # Agent service
│       ├── config.rs                # Config service
│       └── mocks.rs                 # Mock implementations
│
├── examples/
│   ├── app_builder_usage.rs         # Integration example
│   ├── before_refactoring.rs        # Old pattern
│   ├── after_refactoring.rs         # New pattern
│   └── mockall_testing.rs           # Mock examples
│
├── docs/
│   ├── architecture/
│   │   ├── implementation/
│   │   │   ├── PHASE2_COMPLETE_GUIDE.md      # Complete guide
│   │   │   ├── APP_BUILDER_GUIDE.md          # Builder patterns
│   │   │   ├── MOCK_IMPLEMENTATION_GUIDE.md  # Mock guide
│   │   │   └── MAIN_RS_INTEGRATION.md        # Integration
│   │   └── planning/
│   │       ├── RUSTBOT_REFACTORING_PLAN.md   # Overall plan
│   │       └── PHASE2_IMPLEMENTATION_PLAN.md # Phase 2 plan
│   │
│   ├── qa/
│   │   └── PHASE2_QA_REPORT.md              # QA validation
│   │
│   └── progress/
│       └── 2025-01-17-phase2-implementation.md  # Session log
│
└── CHANGELOG.md                     # Version history
```

---

## Key References

- **[PHASE2_COMPLETE_GUIDE.md](./implementation/PHASE2_COMPLETE_GUIDE.md)** - Comprehensive Phase 2 documentation
- **[APP_BUILDER_GUIDE.md](./implementation/APP_BUILDER_GUIDE.md)** - AppBuilder pattern details
- **[MOCK_IMPLEMENTATION_GUIDE.md](./implementation/MOCK_IMPLEMENTATION_GUIDE.md)** - Mock testing guide
- **[PHASE2_QA_REPORT.md](../qa/PHASE2_QA_REPORT.md)** - QA validation results
- **[CHANGELOG.md](../../CHANGELOG.md)** - Version history

---

## Success Metrics

| Metric | Value | Status |
|--------|-------|--------|
| Test Pass Rate | 99.4% | ✅ |
| Test Coverage (New) | 100% | ✅ |
| Performance Regression | 0% | ✅ |
| Production .expect() | 0 | ✅ |
| Code Formatting | 100% | ✅ |

---

**Phase 2 Status**: ✅ PRODUCTION READY

**Last Updated**: 2025-01-17
**Next Phase**: Phase 3 - UI Decoupling
