# Rustbot Development Plan
## Phase 1: POC Chatbot with OpenRouter Streaming

**Last Updated:** 2025-11-12  
**Status:** Planning

---

## Phase 1: POC - Basic Streaming Chatbot

### Goal
Build a minimal viable chatbot that can:
- Connect to OpenRouter API
- Use Claude Sonnet 4.5 model
- Stream responses in real-time
- Display in a simple egui UI

### Success Criteria
- [ ] User can type messages in UI
- [ ] Messages are sent to OpenRouter API
- [ ] Streaming responses appear in real-time in the UI
- [ ] Application runs without crashes
- [ ] API key is loaded from environment or config

---

## Implementation Steps

### Step 1: Project Initialization
**Goal:** Set up basic Rust project structure

**Tasks:**
- [ ] Create Cargo.toml with initial dependencies
- [ ] Set up main.rs with basic entry point
- [ ] Initialize git repository
- [ ] Create .gitignore for Rust projects
- [ ] Add .env file for API key (not committed)

**Dependencies needed:**
```toml
tokio = { version = "1.40", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
egui = "0.29"
eframe = { version = "0.29", default-features = false, features = ["default_fonts", "glow"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Estimated time:** 30 minutes

---

### Step 2: OpenRouter API Integration
**Goal:** Create a basic client that can send messages and receive streaming responses

**Tasks:**
- [ ] Create `src/llm/` module
- [ ] Implement OpenRouter API client struct
- [ ] Add method to send chat completion request
- [ ] Handle streaming SSE (Server-Sent Events) responses
- [ ] Parse response chunks into usable format
- [ ] Add basic error handling

**Key Implementation Details:**

- OpenRouter API endpoint: `https://openrouter.ai/api/v1/chat/completions`
- Model identifier: `anthropic/claude-sonnet-4.5:beta`
- Use streaming mode: `"stream": true` in request body
- Response format: Server-Sent Events (SSE)
- Each chunk contains: `data: {json_object}\n\n`
- Final chunk: `data: [DONE]`

**Example API Request:**
```json
{
  "model": "anthropic/claude-sonnet-4.5:beta",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "stream": true
}
```

**Example Response Stream:**
```
data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Hello"}}]}

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"!"}}]}

data: [DONE]
```

**Estimated time:** 2-3 hours

---

### Step 3: Message Types and Conversation Management
**Goal:** Define data structures for messages and conversation history

**Tasks:**
- [ ] Create `src/types/` module
- [ ] Define `Message` struct (role, content)
- [ ] Define `ConversationHistory` struct
- [ ] Add methods to append messages
- [ ] Implement simple in-memory storage

**Data structures:**
```rust
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

pub enum MessageRole {
    User,
    Assistant,
    System,
}

pub struct ConversationHistory {
    messages: Vec<Message>,
}
```

**Estimated time:** 1 hour

---

### Step 4: Basic egui UI
**Goal:** Create minimal chat interface

**Tasks:**
- [ ] Create `src/ui/` module
- [ ] Implement basic window layout
- [ ] Add scrollable chat message area
- [ ] Add text input field at bottom
- [ ] Add send button
- [ ] Display messages with role indicators (User/Assistant)
- [ ] Auto-scroll to bottom on new messages

**UI Layout:**
```
┌─────────────────────────────────┐
│  Rustbot POC                    │
├─────────────────────────────────┤
│                                 │
│  User: Hello                    │
│  Assistant: Hi! How can I...    │
│  User: What's the weather?      │
│  Assistant: [streaming...]      │
│                                 │
│                                 │
├─────────────────────────────────┤
│  [Type message here...    ]     │
│                          [Send] │
└─────────────────────────────────┘
```

**Estimated time:** 2-3 hours

---

### Step 5: Wire UI to LLM Client
**Goal:** Connect UI events to API calls

**Tasks:**
- [ ] Set up tokio runtime in main
- [ ] Create channel for UI -> LLM communication
- [ ] Handle send button clicks
- [ ] Spawn async task to call LLM on user input
- [ ] Stream response chunks back to UI
- [ ] Update UI in real-time as chunks arrive
- [ ] Handle errors gracefully

**Architecture:**
```
UI (egui)  -->  [mpsc channel]  -->  LLM Task (async)
   ^                                      |
   |            [mpsc channel]  <---------+
   +---------- (stream chunks)
```

**Estimated time:** 3-4 hours

---

### Step 6: Configuration and API Key Management
**Goal:** Load API key and configuration safely

**Tasks:**
- [ ] Create `config.toml` template
- [ ] Add `dotenvy` or similar for .env file loading
- [ ] Load OpenRouter API key from environment
- [ ] Add config struct for model selection
- [ ] Validate configuration on startup

**Configuration file example:**
```toml
[llm]
model = "anthropic/claude-sonnet-4.5:beta"
api_base_url = "https://openrouter.ai/api/v1"
max_tokens = 4096
temperature = 1.0

[ui]
window_width = 800
window_height = 600
```

**Environment variables:**
```bash
OPENROUTER_API_KEY=your_key_here
```

**Estimated time:** 1 hour

---

### Step 7: Error Handling and Polish
**Goal:** Make the POC robust and usable

**Tasks:**
- [ ] Add loading indicator while waiting for response
- [ ] Show error messages in UI if API call fails
- [ ] Prevent sending empty messages
- [ ] Add keyboard shortcut (Enter to send)
- [ ] Add basic logging with tracing
- [ ] Test with various message lengths
- [ ] Add README with setup instructions

**Estimated time:** 2 hours

---

## Total Estimated Time: 12-16 hours

---

## Testing Strategy

### Manual Testing Checklist
- [ ] Application starts without errors
- [ ] Can type in text field
- [ ] Send button triggers message send
- [ ] User message appears immediately
- [ ] Assistant response streams in real-time
- [ ] Multiple back-and-forth messages work
- [ ] Long messages display correctly
- [ ] API errors show user-friendly message
- [ ] Application can be closed cleanly

### Edge Cases to Test
- [ ] Empty message handling
- [ ] Very long messages (>1000 chars)
- [ ] Network timeout
- [ ] Invalid API key
- [ ] Rapid message sending
- [ ] Special characters in messages

---

## Success Metrics

**POC is complete when:**
1. User can have a multi-turn conversation with Claude
2. Streaming works smoothly with no visible lag
3. UI is responsive and doesn't freeze
4. Errors are handled gracefully
5. Code is clean and documented

---

## What's NOT in POC

**Deliberately excluded to keep scope focused:**
- Event-driven architecture
- Priority queue
- Tool/plugin system
- State machine
- Interrupt capability
- Multiple UI panels
- Event visualization
- Metrics collection
- Conversation persistence
- Multiple LLM providers

These will be added in subsequent phases after the POC proves the basic streaming works.

---

## Deliverables

### Code
- [ ] Working Rust application
- [ ] Source organized in logical modules
- [ ] Cargo.toml with all dependencies

### Documentation
- [ ] README.md with setup instructions
- [ ] Code comments for non-obvious logic
- [ ] Config file examples

### Testing
- [ ] Manual test results documented
- [ ] Screenshots or recording of working app

---

## Next Phases (After POC)

### Phase 2: Event System Foundation
- Implement priority event queue
- Add basic event types
- Create event dispatcher
- Add state machine

### Phase 3: Tool Integration
- Design tool trait
- Create tool registry
- Implement first few basic tools
- Add tool calling to LLM workflow

### Phase 4: Full UI with Visualization
- Multi-panel layout
- Event stream visualization
- System stats panel
- Activity graphs

### Phase 5: Production Hardening
- Error recovery
- Conversation persistence
- Configuration management
- Performance optimization

---

## Notes and Decisions

### OpenRouter vs Direct Anthropic API
**Decision:** Use OpenRouter for POC
**Rationale:**
- Single API for multiple models
- Simpler billing management
- Can easily switch models for testing
- Standard OpenAI-compatible format


### Streaming Implementation Approach
**Decision:** Use reqwest with streaming support
**Rationale:**
- Native Rust HTTP client
- Good async/await support
- Built-in streaming via `bytes_stream()`
- Well-maintained and popular

**Alternative considered:** `eventsource` crate
- Would handle SSE parsing automatically
- But adds complexity for minimal benefit
- Manual SSE parsing is straightforward

### UI Framework Choice
**Decision:** egui
**Rationale:**
- Immediate mode GUI (no complex state management)
- Native rendering (no web overhead)
- Good for desktop applications
- Active development and community

**Alternative considered:** Iced
- More declarative
- Elm-like architecture
- But higher learning curve for POC

---

## Development Environment

### Required Tools
- Rust 1.70+ (stable channel)
- Cargo (comes with Rust)
- Git
- Text editor / IDE with Rust support

### Recommended IDE Setup
- VS Code with rust-analyzer extension
- Or RustRover / IntelliJ IDEA with Rust plugin
- Or any editor with LSP support

### Environment Setup Steps
```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone/create project
cd ~/Projects
mkdir rustbot
cd rustbot

# 3. Initialize Cargo project (done by this plan)
# cargo init --name rustbot

# 4. Create .env file
echo "OPENROUTER_API_KEY=your_key_here" > .env

# 5. Build and run
cargo build
cargo run
```

---

## Risk Assessment

### Technical Risks
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|---------|------------|
| SSE parsing issues | Medium | High | Start with simple manual parsing, add tests |
| UI freezing during API calls | Low | High | Use async properly, spawn tasks |
| API rate limits | Low | Medium | Add delay between requests if needed |
| Token limit exceeded | Medium | Low | Track conversation length, truncate if needed |

### Schedule Risks
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|---------|------------|
| Learning curve with egui | Medium | Medium | Start with simple UI, iterate |
| SSE streaming complexity | Low | Medium | Reference existing examples |
| Scope creep | High | High | Stick to POC checklist strictly |

---

## Questions to Resolve

- [ ] Should conversation history be limited? (Suggest: 50 messages)
- [ ] Should we show token usage in UI? (Suggest: Yes, in status bar)
- [ ] Default system message? (Suggest: None for POC)
- [ ] Should message history persist between runs? (Suggest: No for POC)
- [ ] Window size configurable or fixed? (Suggest: Fixed for POC)

---

## Ready to Begin

Once you've reviewed this plan and provided your OpenRouter API key, we can start with Step 1: Project Initialization.

The key is to keep the POC scope tight and focused. We're proving that:
1. We can connect to OpenRouter
2. We can stream responses
3. We can display them in a native UI

Everything else comes later.
