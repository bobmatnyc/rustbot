# Rustbot API Documentation

## Overview

Rustbot provides a comprehensive programmatic API for all its functionality. The API follows a key design principle:

> **All functionality should be accessible via APIs, not just through UI interactions.**

This enables:
- **Scripting** - Automate conversations and workflows
- **Testing** - Write comprehensive tests without UI
- **Integration** - Embed Rustbot in other applications
- **Headless Operation** - Run without GUI
- **Programmatic Control** - Full access to all features

## Quick Start

### 1. Add Rustbot as a Dependency

```toml
[dependencies]
rustbot = { path = "../rustbot" }  # Or from crates.io when published
```

### 2. Basic Usage

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;
use std::env;

fn main() -> anyhow::Result<()> {
    // Get API key
    let api_key = env::var("OPENROUTER_API_KEY")?;

    // Create LLM adapter
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Build API
    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .build()?;

    // Send message and get response
    let response = api.send_message_blocking("Hello, what is Rust?")?;
    println!("{}", response);

    Ok(())
}
```

## Core API: `RustbotApi`

### Creating an API Instance

Use `RustbotApiBuilder` for flexible configuration:

```rust
let mut api = RustbotApiBuilder::new()
    .llm_adapter(llm_adapter)
    .max_history_size(20)
    .system_instructions("You are a helpful assistant.".to_string())
    .build()?;
```

### Message Operations

#### Send Message (Blocking)

Wait for complete response before continuing:

```rust
let response = api.send_message_blocking("What is Rust?")?;
println!("Response: {}", response);
```

**Use Cases:**
- Scripts and automation
- Simple Q&A flows
- Testing scenarios
- Batch processing

#### Send Message (Streaming)

Get response chunks as they arrive:

```rust
let mut result_rx = api.send_message("Explain Rust ownership")?;

// Create runtime for async operations
let runtime = tokio::runtime::Runtime::new()?;

runtime.block_on(async {
    if let Some(Ok(mut stream_rx)) = result_rx.recv().await {
        while let Some(chunk) = stream_rx.recv().await {
            print!("{}", chunk);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }
});
```

**Use Cases:**
- Real-time display
- Progress indicators
- Interactive applications
- Low-latency requirements

### Agent Management

#### List Available Agents

```rust
let agents = api.list_agents();
println!("Available agents: {:?}", agents);
```

#### Switch Active Agent

```rust
api.switch_agent("researcher")?;
println!("Now using: {}", api.active_agent());
```

#### Register Custom Agent

```rust
use rustbot::agent::{Agent, AgentConfig};

let config = AgentConfig {
    id: "coder".to_string(),
    name: "Coding Assistant".to_string(),
    instructions: "You are an expert programmer.".to_string(),
    personality: "Be concise and code-focused.".to_string(),
    model: "anthropic/claude-sonnet-4.5".to_string(),
    enabled: true,
};

let agent = Agent::new(
    config,
    llm_adapter,
    event_bus,
    runtime,
    system_instructions,
);

api.register_agent(agent);
```

### History Management

#### Get Conversation History

```rust
let history = api.get_history();
for (i, msg) in history.iter().enumerate() {
    println!("{}: {} - {}", i, msg.role, msg.content);
}
```

#### Clear History

```rust
api.clear_history();
println!("History cleared");
```

### Event System Integration

#### Subscribe to Events

```rust
let mut event_rx = api.subscribe_events();

// Spawn task to handle events
tokio::spawn(async move {
    while let Ok(event) = event_rx.recv().await {
        println!("Event: {:?}", event);
    }
});
```

#### Publish Custom Events

```rust
use rustbot::events::{Event, EventKind};

let event = Event::new(
    "my_app".to_string(),
    "broadcast".to_string(),
    EventKind::UserMessage("Test".to_string()),
);

api.publish_event(event)?;
```

## Examples

### Example 1: Simple Q&A Bot

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .build()?;

    let questions = vec![
        "What is Rust?",
        "What are the main benefits?",
        "How does ownership work?",
    ];

    for question in questions {
        println!("\nQ: {}", question);
        let answer = api.send_message_blocking(question)?;
        println!("A: {}", answer);
    }

    Ok(())
}
```

### Example 2: Interactive Chat Loop

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::llm::{create_adapter, AdapterType};
use std::io::{self, Write};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .system_instructions("You are a friendly AI assistant.".to_string())
        .build()?;

    println!("Chat with Rustbot (type 'exit' to quit)");

    loop {
        print!("\nYou: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            break;
        }

        print!("Bot: ");
        let response = api.send_message_blocking(input)?;
        println!("{}", response);
    }

    Ok(())
}
```

### Example 3: Batch Processing

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .build()?;

    // Load tasks from file or database
    let tasks = vec![
        "Summarize the benefits of Rust",
        "Explain async/await in 2 sentences",
        "List 3 popular Rust frameworks",
    ];

    for (i, task) in tasks.iter().enumerate() {
        println!("\nTask {}: {}", i + 1, task);

        match api.send_message_blocking(task) {
            Ok(result) => {
                println!("✓ Result: {}", result);
            }
            Err(e) => {
                eprintln!("✗ Error: {}", e);
            }
        }
    }

    Ok(())
}
```

### Example 4: Agent Switching

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::agent::AgentConfig;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Create two different agents
    let mut coder_config = AgentConfig::default_assistant();
    coder_config.id = "coder".to_string();
    coder_config.instructions = "You are a coding expert. Be concise.".to_string();

    let mut writer_config = AgentConfig::default_assistant();
    writer_config.id = "writer".to_string();
    writer_config.instructions = "You are a creative writer. Be expressive.".to_string();

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .add_agent(coder_config)
        .add_agent(writer_config)
        .build()?;

    // Use coder
    api.switch_agent("coder")?;
    let code_response = api.send_message_blocking("Write a hello world in Rust")?;
    println!("Coder: {}", code_response);

    // Switch to writer
    api.switch_agent("writer")?;
    let story_response = api.send_message_blocking("Write a short poem about Rust")?;
    println!("Writer: {}", story_response);

    Ok(())
}
```

## Design Principles

### 1. API-First Architecture

All functionality is accessible programmatically:
- UI actions call the same APIs
- No "UI-only" features
- Consistent behavior across interfaces
- Full testability

### 2. Blocking and Non-Blocking Options

Both synchronous and asynchronous patterns supported:
- `send_message()` - Streaming, non-blocking
- `send_message_blocking()` - Wait for complete response
- Choose based on your use case

### 3. Builder Pattern

Flexible, discoverable configuration:
- `RustbotApiBuilder` for setup
- Sensible defaults
- Optional parameters
- Chainable methods

### 4. Event-Driven

Pub/sub event system for integration:
- Subscribe to system events
- Publish custom events
- Agent status changes
- Message flow tracking

## Testing

The API enables comprehensive testing without UI:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_flow() {
        let api_key = "test-key".to_string();
        let llm_adapter: Arc<dyn LlmAdapter> =
            Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

        let mut api = RustbotApiBuilder::new()
            .llm_adapter(llm_adapter)
            .build()
            .unwrap();

        // Test agent switching
        assert_eq!(api.active_agent(), "assistant");

        // Test history management
        api.clear_history();
        assert_eq!(api.get_history().len(), 0);
    }
}
```

## Running the Demo

A complete working example is available:

```bash
# Set your API key
export OPENROUTER_API_KEY="your-key-here"

# Run the demo
cargo run --example api_demo
```

The demo shows:
1. Blocking message sending
2. Streaming response handling
3. History management
4. Multi-turn conversations

## Error Handling

All API methods return `anyhow::Result` for flexible error handling:

```rust
match api.send_message_blocking("Hello") {
    Ok(response) => println!("Success: {}", response),
    Err(e) => eprintln!("Error: {}", e),
}
```

## JSON-Based Agent System

### Loading Agents from JSON

Rustbot now supports JSON-based agent configuration for flexible, code-free agent management.

#### Basic Agent Loading

```rust
use rustbot::agent::AgentLoader;

// Load all agents from JSON files
let loader = AgentLoader::new();
let agents = loader.load_all()?;

for agent_config in agents {
    println!("Loaded agent: {}", agent_config.name);
}
```

#### Agent JSON Format

Create `agents/custom/my_agent.json`:

```json
{
  "version": "1.0",
  "name": "my_agent",
  "description": "My custom AI agent",
  "provider": "openrouter",
  "model": "anthropic/claude-3.5-sonnet",
  "apiKey": "${OPENROUTER_API_KEY}",
  "instruction": "You are a helpful assistant...",
  "parameters": {
    "temperature": 0.7,
    "maxTokens": 4096
  },
  "capabilities": {
    "webSearch": false,
    "streaming": true
  },
  "enabled": true
}
```

#### Supported Providers

- **openrouter** - Access to many models through one API
- **openai** - Direct OpenAI API access
- **anthropic** - Direct Claude API access
- **ollama** - Local LLMs (no API key needed)

#### Environment Variables

Use `${VAR_NAME}` syntax for secure API key management:

```json
{
  "apiKey": "${OPENROUTER_API_KEY}"
}
```

With defaults:
```json
{
  "apiKey": "${OPENROUTER_API_KEY:-fallback-key}"
}
```

#### Complete Example with JSON Agents

```rust
use rustbot::api::RustbotApiBuilder;
use rustbot::agent::AgentLoader;
use rustbot::llm::{create_adapter, AdapterType};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    // Load agents from JSON
    let loader = AgentLoader::new();
    let agents = loader.load_all()?;

    // Create API with loaded agents
    let api_key = std::env::var("OPENROUTER_API_KEY")?;
    let llm_adapter: Arc<dyn rustbot::llm::LlmAdapter> =
        Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    let mut api = RustbotApiBuilder::new()
        .llm_adapter(llm_adapter)
        .build()?;

    // Register loaded agents
    for agent_config in agents {
        // Convert to Agent and register
        println!("Registered: {}", agent_config.name);
    }

    // Use the agents
    api.switch_agent("web_search")?;
    let response = api.send_message_blocking("What's the latest news?")?;
    println!("{}", response);

    Ok(())
}
```

For complete JSON agent documentation, see `agents/README.md`.

## Future Enhancements

Planned API additions:
- [x] JSON-based agent configuration
- [x] Multi-provider LLM support
- [ ] Session save/load
- [ ] Conversation export
- [ ] Agent hot-reloading
- [ ] Streaming history updates
- [ ] Batch message processing
- [ ] Async-first APIs
- [ ] WebSocket API for remote access
- [ ] REST API wrapper

## Support

For issues or questions:
- GitHub Issues: https://github.com/yourusername/rustbot/issues
- Documentation: https://github.com/yourusername/rustbot/tree/main/docs

## License

Same as Rustbot project license.

---

## Architecture & Implementation

This section describes the API-first architecture implementation and design principles.

### Architecture Layers

The Rustbot API follows a layered architecture:

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

### Implementation

**Core API Structure** (`src/api.rs`):

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

### Library Interface

Rustbot is exposed as a library via `src/lib.rs`:

**Exports**:
- `RustbotApi`, `RustbotApiBuilder` - Core API
- `Agent`, `AgentConfig` - Agent types
- `Event`, `EventBus`, `EventKind` - Event system
- `LlmAdapter`, `LlmMessage`, `LlmRequest` - LLM types

**Cargo.toml Configuration**:
```toml
[lib]
name = "rustbot"
path = "src/lib.rs"

[[bin]]
name = "rustbot"
path = "src/main.rs"
```

This enables:
- Using Rustbot as a dependency
- Importing types and APIs
- Building custom applications

### Problem Statement

The API-first architecture addressed fundamental issues:

**Original Issues**:
1. No programmatic access - couldn't send messages from code
2. Testing limitations - required UI to test
3. Integration barriers - couldn't embed in other apps
4. No scriptability - couldn't automate workflows
5. Tight coupling - business logic mixed with UI

**Solution Benefits**:
- Complete programmatic access to all features
- UI-independent testing
- Easy integration into other applications
- Script automation support
- Clean separation of concerns

---

For more architectural details, see:
- `/docs/ARCHITECTURE.md` - Core system architecture
- `/docs/AGENT_EVENT_ARCHITECTURE.md` - Agent system design
- `/docs/design/` - Feature-specific design documents
