# API-First Architecture Implementation

## Overview

This document describes the API-first architecture implementation for Rustbot, addressing the fundamental design principle:

> **All user actions must have programmatic API equivalents.**

## Problem Statement

**Original Issue**: The system relied on UI interactions to trigger functionality. This created several problems:

1. **No programmatic access** - Couldn't send messages or control agents from code
2. **Testing limitations** - Required UI to test functionality
3. **Integration barriers** - Couldn't embed in other applications
4. **No scriptability** - Couldn't automate workflows
5. **Tight coupling** - Business logic mixed with UI code

**User Requirement** (direct quote):
> "But you should be able to push messages through the system directly through APIs. And if not, we need to design APIs to allow that... We shouldn't rely on just user interaction. All major functionality should have API equivalents."

## Solution: Layered API Architecture

### Architecture Layers

```
┌─────────────────────────────────────────┐
│          UI Layer (egui)                │
│  - Renders UI                           │
│  - Handles user input                   │
│  - Calls API methods                    │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│       RustbotApi (API Layer)            │
│  - send_message()                       │
│  - switch_agent()                       │
│  - clear_history()                      │
│  - manage agents                        │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│       Core Services                     │
│  - Agent (processing)                   │
│  - EventBus (pub/sub)                   │
│  - LlmAdapter (LLM calls)              │
└─────────────────────────────────────────┘
```

### Design Principles

1. **API-First**: All functionality accessible via API
2. **UI as Consumer**: UI calls APIs, doesn't implement logic
3. **Testability**: Can test without UI
4. **Flexibility**: Both blocking and non-blocking APIs
5. **Discoverability**: Builder pattern with clear method names

## Implementation Details

### Created Files

#### 1. `src/api.rs` - Core API Module

**Purpose**: Provide programmatic access to all Rustbot functionality

**Key Components**:

```rust
pub struct RustbotApi {
    event_bus: Arc<EventBus>,
    runtime: Arc<Runtime>,
    agents: Vec<Agent>,
    active_agent_id: String,
    message_history: VecDeque<LlmMessage>,
    max_history_size: usize,
}
```

**API Methods**:

| Method | Purpose | Use Case |
|--------|---------|----------|
| `send_message()` | Send message, get streaming response | Real-time display |
| `send_message_blocking()` | Send message, wait for full response | Scripts, automation |
| `switch_agent()` | Change active agent | Multi-agent workflows |
| `list_agents()` | Get available agents | Discovery |
| `clear_history()` | Clear conversation history | Reset state |
| `get_history()` | Get message history | Context retrieval |
| `register_agent()` | Add new agent | Extensibility |
| `publish_event()` | Send custom events | Integration |
| `subscribe_events()` | Receive events | Monitoring |

**Builder Pattern**:

```rust
let api = RustbotApiBuilder::new()
    .llm_adapter(llm_adapter)
    .max_history_size(20)
    .system_instructions("You are helpful.".to_string())
    .add_agent(custom_agent_config)
    .build()?;
```

#### 2. `src/lib.rs` - Library Interface

**Purpose**: Expose Rustbot as a library for external use

**Exports**:
- `RustbotApi`, `RustbotApiBuilder` - Core API
- `Agent`, `AgentConfig` - Agent types
- `Event`, `EventBus`, `EventKind` - Event system
- `LlmAdapter`, `LlmMessage`, `LlmRequest` - LLM types

**Enables**:
- Using Rustbot as a dependency
- Importing types and APIs
- Building custom applications

#### 3. `examples/api_demo.rs` - API Usage Examples

**Purpose**: Demonstrate programmatic API usage

**Examples**:
1. **Blocking API call** - Simple Q&A
2. **Streaming API call** - Real-time response
3. **History management** - View and clear history
4. **Multiple examples** - Various use cases

**Running**:
```bash
export OPENROUTER_API_KEY="your-key"
cargo run --example api_demo
```

#### 4. `docs/API.md` - Comprehensive Documentation

**Contents**:
- Quick start guide
- API reference
- Usage patterns
- Code examples
- Design principles
- Testing strategies

**Covers**:
- Message operations (blocking/streaming)
- Agent management
- History management
- Event system integration
- Error handling
- Best practices

### Updated Files

#### `Cargo.toml`

Added library and binary targets:

```toml
[lib]
name = "rustbot"
path = "src/lib.rs"

[[bin]]
name = "rustbot"
path = "src/main.rs"
```

**Benefits**:
- Can use Rustbot as a library
- Can build examples with `cargo run --example`
- Separates library API from binary application

#### `src/main.rs`

Added module declaration:

```rust
mod api;  // New API module
```

**Next Step** (not yet implemented):
- Refactor UI code to use `RustbotApi`
- Remove direct agent/message handling from UI
- UI becomes thin consumer of API

## Usage Examples

### 1. Simple Script

```rust
let mut api = RustbotApiBuilder::new()
    .llm_adapter(llm_adapter)
    .build()?;

let response = api.send_message_blocking("What is Rust?")?;
println!("{}", response);
```

### 2. Streaming Response

```rust
let mut result_rx = api.send_message("Explain async")?;

runtime.block_on(async {
    if let Some(Ok(mut stream_rx)) = result_rx.recv().await {
        while let Some(chunk) = stream_rx.recv().await {
            print!("{}", chunk);
        }
    }
});
```

### 3. Agent Switching

```rust
api.switch_agent("researcher")?;
let research = api.send_message_blocking("Research topic X")?;

api.switch_agent("writer")?;
let summary = api.send_message_blocking("Summarize the research")?;
```

### 4. Batch Processing

```rust
for task in tasks {
    match api.send_message_blocking(task) {
        Ok(result) => save_result(result),
        Err(e) => log_error(e),
    }
}
```

## Benefits Achieved

### 1. Programmatic Access ✅

- Send messages from code
- Control agents programmatically
- Manage history via API
- Access event system

### 2. Testability ✅

```rust
#[test]
fn test_agent_switching() {
    let mut api = create_test_api();
    api.switch_agent("researcher")?;
    assert_eq!(api.active_agent(), "researcher");
}
```

### 3. Scriptability ✅

```bash
#!/bin/bash
cargo run --example api_demo
```

### 4. Integration ✅

```rust
// In your application
use rustbot::api::RustbotApiBuilder;

let api = RustbotApiBuilder::new()
    .llm_adapter(my_adapter)
    .build()?;
```

### 5. Separation of Concerns ✅

- **API Layer**: Business logic
- **UI Layer**: Presentation
- **Core Services**: Infrastructure

## API Patterns

### Blocking vs Non-Blocking

**Blocking** (`.._blocking()` methods):
- Waits for complete response
- Simpler to use
- Good for scripts/automation
- May block UI thread if called from UI

**Non-Blocking** (standard methods):
- Returns immediately with channels
- More complex to use
- Good for real-time/interactive
- Doesn't block UI thread

**When to Use**:
- Scripts: Blocking
- UI: Non-blocking
- Tests: Either (depending on test)
- Batch: Blocking

### Builder Pattern

**Benefits**:
- Discoverable API
- Optional parameters
- Sensible defaults
- Chainable configuration
- Type-safe

**Example**:
```rust
RustbotApiBuilder::new()
    .llm_adapter(adapter)        // Required
    .max_history_size(50)        // Optional
    .system_instructions(text)   // Optional
    .add_agent(config)           // Optional, repeatable
    .build()?                    // Creates API
```

## Testing Strategy

### Unit Tests

Test API methods in isolation:

```rust
#[test]
fn test_clear_history() {
    let mut api = create_test_api();
    api.send_message_blocking("Test")?;
    api.clear_history();
    assert_eq!(api.get_history().len(), 0);
}
```

### Integration Tests

Test API with real LLM:

```rust
#[test]
fn test_message_flow() {
    let api = create_real_api();
    let response = api.send_message_blocking("Hello")?;
    assert!(!response.is_empty());
}
```

### Example-Based Tests

Run examples as tests:

```bash
cargo run --example api_demo
# Verify output manually or with automation
```

## Future Enhancements

### Near-Term

1. **Refactor UI to use API**
   - Remove direct agent access from UI
   - Call `api.send_message()` instead
   - UI becomes pure presentation layer

2. **Add more API methods**
   - `api.save_session(path)?`
   - `api.load_session(path)?`
   - `api.export_conversation(format)?`

3. **Async-first APIs**
   - `async fn send_message_async()`
   - Native async support
   - Better integration with tokio

### Long-Term

1. **REST API Wrapper**
   - HTTP server exposing API
   - OpenAPI specification
   - Remote access

2. **WebSocket API**
   - Real-time bidirectional
   - Streaming support
   - Event notifications

3. **CLI Tool**
   - Command-line interface
   - Piping support
   - Shell integration

4. **Language Bindings**
   - Python wrapper (PyO3)
   - JavaScript (WASM)
   - C FFI

## Migration Path

### Current State

UI directly calls Agent methods:

```rust
// In UI code (current)
if let Some(agent) = self.agents.iter().find(|a| a.id() == self.active_agent_id) {
    let rx = agent.process_message_nonblocking(message, context);
    self.pending_agent_result = Some(rx);
}
```

### Target State

UI calls API methods:

```rust
// In UI code (target)
match self.api.send_message(&message) {
    Ok(rx) => self.pending_agent_result = Some(rx),
    Err(e) => self.show_error(e),
}
```

### Migration Steps

1. ✅ **Create API layer** (Done)
2. ✅ **Add documentation** (Done)
3. ✅ **Create examples** (Done)
4. ⏳ **Refactor UI to use API** (Next)
5. ⏳ **Remove direct agent access from UI** (After refactor)
6. ⏳ **Add comprehensive tests** (After refactor)

## Conclusion

The API-first architecture implementation addresses the core requirement:

> "All user actions should have programmatic API equivalents."

**Achievements**:
- ✅ Complete API layer (`src/api.rs`)
- ✅ Library interface (`src/lib.rs`)
- ✅ Working examples (`examples/api_demo.rs`)
- ✅ Comprehensive documentation (`docs/API.md`)
- ✅ Both blocking and streaming APIs
- ✅ Agent management APIs
- ✅ Event system integration
- ✅ Builder pattern for configuration

**Impact**:
- Enables scripting and automation
- Supports testing without UI
- Allows integration in other apps
- Separates concerns (API vs UI)
- Provides foundation for future features

**Next Steps**:
- Refactor UI to use new API
- Add comprehensive test suite
- Consider additional API methods
- Plan REST/WebSocket wrappers
