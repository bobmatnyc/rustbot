# Testing Strategy for Rustbot

## Overview

Rustbot implements a comprehensive testing strategy that covers all layers of the application through the API-first architecture.

## Test Structure

### Location
- **Unit Tests**: `tests/api_tests.rs` - Core API functionality
- **Integration Tests**: Same file, marked with `#[ignore]` attribute
- **Module Tests**: Within individual modules (e.g., `src/api.rs`)

### Test Categories

#### 1. Unit Tests (No API Key Required)

These tests verify API functionality without making actual LLM calls:

- ✅ **API Creation** - `test_api_creation()`
  - Verifies API initializes with correct defaults
  - Checks default agent is "assistant"
  - Validates agent list contains default agent

- ✅ **Agent Registration** - `test_agent_registration()`
  - Tests adding custom agents dynamically
  - Verifies agent list updates correctly
  - Validates multiple agents can coexist

- ✅ **Agent Switching** - `test_agent_switching()`
  - Tests switching between registered agents
  - Validates active agent tracking
  - Ensures invalid agent IDs are rejected

- ✅ **History Management** - `test_history_management()`
  - Verifies history starts empty
  - Tests clear_history() functionality
  - Validates history structure

- ✅ **Agent Status** - `test_agent_status()`
  - Checks agent status reporting
  - Validates status of active agent
  - Ensures status is initially Idle

- ✅ **Event System** - `test_event_subscription()`
  - Tests event publishing
  - Verifies event subscription
  - Validates event delivery

- ✅ **Builder Pattern** - `test_builder_pattern()`
  - Tests API configuration options
  - Validates builder method chaining
  - Ensures custom configurations work

- ✅ **Builder Validation** - `test_builder_requires_llm_adapter()`
  - Verifies LLM adapter is required
  - Tests builder error handling
  - Ensures invalid configurations fail gracefully

- ✅ **Max History** - `test_max_history_size()`
  - Tests history size limits
  - Validates history trimming (conceptually)

- ✅ **Event Bus Integration** - `test_event_bus_integration()`
  - Tests custom event bus injection
  - Validates event bus sharing
  - Ensures proper integration

#### 2. Integration Tests (API Key Required)

These tests make actual LLM API calls and are ignored by default:

- ⏭️ **Send Message** - `test_send_message_integration()`
  - Tests blocking message sending
  - Verifies actual LLM response
  - Validates message history tracking

- ⏭️ **Streaming** - `test_streaming_integration()`
  - Tests streaming message responses
  - Verifies chunk-by-chunk delivery
  - Validates complete response assembly

**Running Integration Tests:**
```bash
# Set API key
export OPENROUTER_API_KEY="your-key-here"

# Run all tests including integration
cargo test --test api_tests -- --include-ignored

# Run only integration tests
cargo test --test api_tests -- --ignored
```

## Running Tests

### Quick Test Run (Unit Tests Only)
```bash
cargo test --test api_tests
```

**Expected Output:**
```
running 12 tests
test test_send_message_integration ... ignored
test test_streaming_integration ... ignored
test test_builder_requires_llm_adapter ... ok
test test_event_bus_integration ... ok
test test_history_management ... ok
test test_agent_status ... ok
test test_max_history_size ... ok
test test_api_creation ... ok
test test_agent_registration ... ok
test test_event_subscription ... ok
test test_builder_pattern ... ok
test test_agent_switching ... ok

test result: ok. 10 passed; 0 failed; 2 ignored
```

### Full Test Run (Including Integration Tests)
```bash
# Load environment variables
export OPENROUTER_API_KEY="sk-or-v1-..."

# Run all tests
cargo test --test api_tests -- --include-ignored
```

### Continuous Integration

For CI/CD pipelines:

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: cargo test --test api_tests

      # Optional: Run integration tests if API key is available
      - name: Run integration tests
        if: ${{ secrets.OPENROUTER_API_KEY }}
        env:
          OPENROUTER_API_KEY: ${{ secrets.OPENROUTER_API_KEY }}
        run: cargo test --test api_tests -- --include-ignored
```

## Test Coverage

### Current Coverage

| Component | Coverage | Notes |
|-----------|----------|-------|
| RustbotApi | ✅ High | All public methods tested |
| Agent Management | ✅ High | Registration, switching, status |
| Event System | ✅ High | Publishing, subscription, delivery |
| Builder Pattern | ✅ High | Configuration, validation, errors |
| Message History | ⚠️ Partial | Structure tested, trimming requires integration test |
| Message Sending | ⚠️ Integration | Requires API key for full test |
| Streaming | ⚠️ Integration | Requires API key for full test |

### Coverage Goals

- **Unit Tests**: 100% of API methods without external dependencies
- **Integration Tests**: Critical user flows with real LLM
- **End-to-End**: UI interactions through API (manual/automated)

## Testing Best Practices

### 1. Test Naming Convention

```rust
// Pattern: test_<component>_<scenario>
test_api_creation()           // Component: api, Scenario: creation
test_agent_switching()        // Component: agent, Scenario: switching
test_builder_requires_llm()   // Component: builder, Scenario: validation
```

### 2. Test Structure (Arrange-Act-Assert)

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data
    let mut api = create_test_api();

    // Act: Perform the operation
    let result = api.switch_agent("researcher");

    // Assert: Verify the outcome
    assert!(result.is_ok());
    assert_eq!(api.active_agent(), "researcher");
}
```

### 3. Test Independence

- Each test should be independent
- No shared mutable state between tests
- Create fresh API instances per test
- Clean up after tests if needed

### 4. Integration Test Isolation

```rust
#[test]
#[ignore]  // Requires external API
fn test_integration_scenario() {
    // Only runs when explicitly requested
    // Doesn't affect quick test runs
}
```

## Testing the Event-Driven System

### Manual Verification

The event-driven message system can be verified manually:

1. **Start the application**:
```bash
cargo run
```

2. **Send a message through the UI**:
   - Type a message in the input box
   - Press Send or Enter
   - Verify response appears

3. **Verify event flow**:
   - Check console logs for event publishing
   - Monitor event visualizer (if enabled)
   - Confirm agent status changes

### Automated Event Testing

The event system is tested in `test_event_subscription()`:

```rust
#[test]
fn test_event_subscription() {
    let api = create_test_api();

    // Subscribe to events
    let mut event_rx = api.subscribe_events();

    // Publish test event
    let event = Event::new(...);
    api.publish_event(event).expect("Failed to publish");

    // Verify event received
    let received = runtime.block_on(async {
        event_rx.recv().await
    });

    assert!(received.is_ok());
}
```

## Future Testing Enhancements

### Planned Additions

1. **UI Testing**:
   - End-to-end tests using egui test harness
   - Screenshot comparison for visual regression
   - Interaction testing (click, type, scroll)

2. **Performance Testing**:
   - Message throughput benchmarks
   - Memory usage monitoring
   - Response time tracking

3. **Load Testing**:
   - Multiple concurrent messages
   - Agent switching under load
   - Event bus capacity testing

4. **Error Scenario Testing**:
   - Network failures
   - Invalid API keys
   - Malformed responses
   - Timeout handling

5. **Property-Based Testing**:
   - Using `proptest` for random inputs
   - Fuzzing message content
   - Testing edge cases automatically

### Test Automation

```bash
# Watch mode for rapid development
cargo watch -x "test --test api_tests"

# Test with coverage
cargo tarpaulin --test api_tests

# Continuous benchmarking
cargo bench
```

## Troubleshooting Tests

### Common Issues

#### Test Hangs
- **Cause**: Async operations not properly awaited
- **Fix**: Ensure `runtime.block_on()` is used for async tests

#### Event Not Received
- **Cause**: Race condition in event delivery
- **Fix**: Use timeout with `tokio::time::timeout()`

#### API Key Error
- **Cause**: Environment variable not set
- **Fix**: Run `export OPENROUTER_API_KEY="..."`

#### Test Isolation Failure
- **Cause**: Shared state between tests
- **Fix**: Create fresh instances in each test

## Test Metrics

### Current Status

```
Total Tests: 12
Unit Tests: 10 ✅
Integration Tests: 2 ⏭️ (ignored by default)
Pass Rate: 100% (10/10 unit tests)
Execution Time: ~0.03s (unit tests only)
```

### Quality Gates

For production readiness:
- ✅ All unit tests pass
- ✅ No compiler warnings in test code
- ✅ Test coverage > 80% for API layer
- ⚠️ Integration tests pass (requires API key)
- ⚠️ Performance benchmarks meet targets (not yet implemented)

## Contributing Tests

When adding new features:

1. **Write tests first** (TDD approach):
```rust
#[test]
fn test_new_feature() {
    let api = create_test_api();
    // Test the feature before implementing it
}
```

2. **Ensure tests fail initially** (red)
3. **Implement feature** to make tests pass (green)
4. **Refactor** while keeping tests green
5. **Document** test purpose and scenarios

## Summary

The testing strategy ensures:
- ✅ API functionality works without UI
- ✅ Core features are verified automatically
- ✅ Integration tests available for full validation
- ✅ Event-driven system is testable
- ✅ Fast feedback loop (0.03s for unit tests)
- ✅ CI/CD ready
- ✅ Comprehensive coverage of API layer

**Next Steps:**
- Consider REST/WebSocket wrapper testing
- Add UI interaction tests
- Implement performance benchmarks
- Set up automated test reporting
