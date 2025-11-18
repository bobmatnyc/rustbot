# Rustbot Refactoring Checklist

Quick reference checklist for implementing the architectural refactoring plan.

---

## Phase 1: Extract Trait Interfaces (Week 1-2)

**Goal**: Define abstractions without changing existing behavior.

### FileSystem Trait
- [ ] Create `src/services/mod.rs`
- [ ] Create `src/services/filesystem.rs`
- [ ] Define `FileSystem` trait with methods:
  - [ ] `async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>`
  - [ ] `async fn read_to_string(&self, path: &Path) -> Result<String>`
  - [ ] `async fn exists(&self, path: &Path) -> bool`
- [ ] Add `Send + Sync` bounds
- [ ] Add comprehensive documentation

### ConfigService Trait
- [ ] Create `src/services/config.rs`
- [ ] Define `ConfigService` trait with methods:
  - [ ] `fn get_api_key(&self) -> Result<String>`
  - [ ] `fn get_model(&self) -> String`
  - [ ] `fn get_agents_dir(&self) -> PathBuf`
  - [ ] `fn get_mcp_config_path(&self) -> PathBuf`
- [ ] Add `Send + Sync` bounds
- [ ] Document environment variables used

### AgentService Trait
- [ ] Create `src/services/agent_service.rs`
- [ ] Define `AgentService` trait with methods:
  - [ ] `async fn load_all(&self) -> Result<Vec<AgentConfig>>`
  - [ ] `async fn load_agent(&self, path: &Path) -> Result<AgentConfig>`
  - [ ] `async fn list_agents(&self) -> Vec<String>`
- [ ] Add `Send + Sync` bounds
- [ ] Document error conditions

### McpService Trait
- [ ] Create `src/services/mcp_service.rs`
- [ ] Define `McpService` trait with methods:
  - [ ] `async fn list_servers(&self) -> Result<Vec<McpServer>>`
  - [ ] `async fn start_server(&self, name: &str) -> Result<()>`
  - [ ] `async fn stop_server(&self, name: &str) -> Result<()>`
  - [ ] `async fn call_tool(&self, server: &str, tool: &str, args: Value) -> Result<Value>`
- [ ] Add `Send + Sync` bounds
- [ ] Document MCP protocol version

### Documentation
- [ ] Add module-level docs to `src/services/mod.rs`
- [ ] Document dependency injection pattern
- [ ] Add usage examples in trait docs
- [ ] Update `ARCHITECTURE.md` (if exists) or create it

---

## Phase 2: Implement Services (Week 3-4)

**Goal**: Create concrete implementations using dependency injection.

### RealFileSystem Implementation
- [ ] Implement `RealFileSystem` struct
- [ ] Implement `FileSystem` trait for `RealFileSystem`
- [ ] Use `tokio::fs` for async I/O
- [ ] Add error handling with context
- [ ] Write basic smoke tests

### FileConfigService Implementation
- [ ] Implement `FileConfigService` struct
- [ ] Load configuration from `.env` file
- [ ] Implement `ConfigService` trait
- [ ] Add validation for required variables
- [ ] Handle missing/invalid config gracefully

### DefaultAgentService Implementation
- [ ] Implement `DefaultAgentService` struct
- [ ] Accept `Arc<dyn FileSystem>` in constructor
- [ ] Accept `Arc<dyn ConfigService>` in constructor
- [ ] Migrate logic from `AgentLoader` to `DefaultAgentService`
- [ ] Preserve all existing functionality
- [ ] Add tracing/logging

### DefaultMcpService Implementation
- [ ] Implement `DefaultMcpService` struct
- [ ] Accept `Arc<dyn ConfigService>` in constructor
- [ ] Migrate logic from `McpPluginManager` to `DefaultMcpService`
- [ ] Preserve all existing functionality
- [ ] Add tracing/logging

### AppBuilder
- [ ] Create `src/app_builder.rs`
- [ ] Implement `AppBuilder` struct
- [ ] Add `new()` constructor
- [ ] Add `with_production_deps() -> Result<Self>`
- [ ] Add `build() -> Result<App>`
- [ ] Document dependency construction flow

---

## Phase 3: Add Tests (Week 5)

**Goal**: Comprehensive test coverage using mocks.

### Mock Implementations
- [ ] Implement `MockFileSystem` in `src/services/filesystem.rs`
  - [ ] Use `HashMap<PathBuf, String>` for file storage
  - [ ] Add `add_file()` helper method
  - [ ] Implement all `FileSystem` trait methods
- [ ] Implement `MockConfigService` in `src/services/config.rs`
  - [ ] Hardcode test values
  - [ ] Add setter methods for test customization
- [ ] Add `#[cfg(test)]` guards to all mock code

### Unit Tests for AgentService
- [ ] Test `load_all()` with mock filesystem
  - [ ] Empty directory
  - [ ] Multiple agents
  - [ ] Invalid JSON (should skip)
  - [ ] Missing required fields
- [ ] Test `load_agent()` with various configs
  - [ ] Valid agent
  - [ ] Agent with personality
  - [ ] Agent with MCP extensions
  - [ ] Invalid JSON
- [ ] Test error handling
  - [ ] Missing file
  - [ ] Permission denied
  - [ ] Malformed JSON

### Unit Tests for McpService
- [ ] Test `list_servers()` with mock config
- [ ] Test `start_server()` with mock process
- [ ] Test `stop_server()` cleanup
- [ ] Test `call_tool()` JSON-RPC communication

### Integration Tests
- [ ] Create `tests/integration/` directory
- [ ] Test `AgentService` with `RealFileSystem` and `TempDir`
- [ ] Test `FileConfigService` with test `.env` file
- [ ] Test `AppBuilder` full dependency graph
- [ ] Test end-to-end agent loading

### Property-Based Tests
- [ ] Install `proptest` dependency
- [ ] Test agent name validation with random strings
- [ ] Test JSON parsing with fuzzed inputs
- [ ] Test configuration with random env vars

---

## Phase 4: Migrate UI (Week 6)

**Goal**: Update UI to use new service-based architecture.

### Update main.rs
- [ ] Replace `AgentLoader::new()` with `AppBuilder`
- [ ] Call `AppBuilder::new().with_production_deps()?.build()?`
- [ ] Pass services to `App` struct
- [ ] Remove old initialization code
- [ ] Test application startup

### Update App Struct
- [ ] Add `agent_service: Arc<dyn AgentService>` field
- [ ] Add `mcp_service: Arc<dyn McpService>` field
- [ ] Add `config: Arc<dyn ConfigService>` field
- [ ] Update constructor to receive services
- [ ] Remove direct filesystem access

### Update UI Components
- [ ] Replace `AgentLoader` calls with `AgentService`
- [ ] Replace direct config access with `ConfigService`
- [ ] Replace `McpPluginManager` with `McpService`
- [ ] Update error handling for async service calls
- [ ] Test all UI flows (agent selection, MCP tools, etc.)

### Deprecation and Cleanup
- [ ] Mark `AgentLoader` as `#[deprecated]`
- [ ] Add deprecation notice with migration guide
- [ ] Keep old code for one version (backward compatibility)
- [ ] Plan removal in next major version
- [ ] Update CHANGELOG.md

### Documentation Updates
- [ ] Update README.md with new architecture
- [ ] Update DEVELOPMENT.md with service layer info
- [ ] Add architecture diagram to docs
- [ ] Document testing strategy
- [ ] Update VERSION_MANAGEMENT.md if needed

---

## Testing Checklist

### Before Each Phase
- [ ] All existing tests pass
- [ ] No compiler warnings
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` applied

### After Each Phase
- [ ] All new tests pass
- [ ] Integration tests pass
- [ ] No regressions in existing functionality
- [ ] Documentation updated
- [ ] Code reviewed (if team project)

### Final Integration Testing
- [ ] Application starts without errors
- [ ] Agent loading works (presets and custom)
- [ ] MCP plugins load and function
- [ ] Configuration loads from `.env`
- [ ] All UI features work as before
- [ ] Performance is acceptable (no regressions)

---

## Validation Criteria

### Code Quality
- [ ] No `unwrap()` in production code (use `?` or `expect()` with message)
- [ ] All public APIs documented
- [ ] Error messages are helpful and actionable
- [ ] Tracing/logging added for debugging
- [ ] No blocking code in async functions

### Architecture
- [ ] Services depend on traits, not concrete types
- [ ] All dependencies injected via constructor
- [ ] No global state (except logging)
- [ ] Clear separation: domain, service, infrastructure
- [ ] Builder pattern used for complex construction

### Testing
- [ ] Test coverage >70% for service layer
- [ ] Unit tests run in <1 second (no I/O)
- [ ] Integration tests use isolated environments
- [ ] All error paths tested
- [ ] Mock implementations complete

---

## Common Pitfalls to Avoid

### During Implementation
- [ ] ❌ Don't rewrite everything at once (gradual migration)
- [ ] ❌ Don't skip tests (write tests as you go)
- [ ] ❌ Don't break backward compatibility (keep old APIs temporarily)
- [ ] ❌ Don't over-engineer (YAGNI principle)
- [ ] ❌ Don't forget `Send + Sync` bounds on traits

### Testing
- [ ] ❌ Don't test implementation details (test behavior)
- [ ] ❌ Don't use real filesystem in unit tests (use mocks)
- [ ] ❌ Don't forget to test error conditions
- [ ] ❌ Don't skip integration tests (complement unit tests)

### mockall Specific
- [ ] ❌ Don't forget: `#[automock]` BEFORE `#[async_trait]`
- [ ] ❌ Don't forget: `use mockall::*;` in test modules
- [ ] ❌ Don't mock everything (manual mocks fine for simple cases)

---

## Success Indicators

### Phase 1 Complete When:
- [ ] All trait interfaces defined
- [ ] Traits documented with examples
- [ ] No breaking changes to existing code
- [ ] Code compiles without warnings

### Phase 2 Complete When:
- [ ] All service implementations done
- [ ] `AppBuilder` working
- [ ] Services can be instantiated
- [ ] Old code still works (parallel implementation)

### Phase 3 Complete When:
- [ ] All mock implementations done
- [ ] Unit test coverage >70%
- [ ] All tests pass
- [ ] Integration tests added

### Phase 4 Complete When:
- [ ] UI migrated to new services
- [ ] Old code deprecated (not removed yet)
- [ ] All features working
- [ ] Documentation updated
- [ ] Ready for release

---

## Rollback Plan

If refactoring causes issues:

1. **Phase 1-2**: Easy rollback (new code, old code untouched)
2. **Phase 3**: Easy rollback (tests don't affect production)
3. **Phase 4**: Harder rollback (UI changes)
   - Keep old code path active initially
   - Feature flag the new architecture
   - Gradual rollout to users

**Git Strategy**: Create feature branch, merge to main after each phase.

---

## Post-Refactoring Tasks

- [ ] Create blog post / documentation about architecture
- [ ] Update session log with final results
- [ ] Measure test coverage improvement
- [ ] Measure code duplication reduction
- [ ] Plan next architectural improvements (if any)
- [ ] Celebrate! 🎉

---

## Quick Commands

```bash
# Run all tests
cargo test

# Run tests with coverage
cargo tarpaulin --out Html

# Run clippy
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Build release
cargo build --release

# Run application
./target/debug/rustbot
```

---

**Checklist Version**: 1.0
**Last Updated**: January 17, 2025
**Estimated Duration**: 6 weeks
**Status**: Ready to Begin
