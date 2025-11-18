# Mermaid Diagram Examples for Testing

This document contains various Mermaid diagram examples to test the rendering functionality.

## Test 1: Flowchart

```mermaid
graph TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Great!]
    B -->|No| D[Debug]
    D --> E[Fix bugs]
    E --> B
    C --> F[End]
```

## Test 2: Sequence Diagram

```mermaid
sequenceDiagram
    participant User
    participant Rustbot
    participant OpenRouter
    participant Claude

    User->>Rustbot: Send message
    Rustbot->>OpenRouter: Forward request
    OpenRouter->>Claude: Process with AI
    Claude-->>OpenRouter: Generate response
    OpenRouter-->>Rustbot: Return response
    Rustbot-->>User: Display result
```

## Test 3: Class Diagram

```mermaid
classDiagram
    class RustbotApp {
        -messages: Vec~ChatMessage~
        -mermaid_renderer: MermaidRenderer
        -markdown_cache: CommonMarkCache
        +new() RustbotApp
        +preprocess_mermaid() String
        +send_message()
    }

    class MermaidRenderer {
        -client: reqwest::Client
        -cache: HashMap
        +new() MermaidRenderer
        +render_to_svg() Result~Vec~u8~~
    }

    RustbotApp --> MermaidRenderer: uses
```

## Test 4: State Diagram

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Processing: User sends message
    Processing --> Streaming: Receive response
    Streaming --> Complete: Stream ends
    Complete --> Idle: Ready for next
    Processing --> Error: Request fails
    Error --> Idle: Reset
```

## Test 5: Entity Relationship Diagram

```mermaid
erDiagram
    USER ||--o{ MESSAGE : sends
    MESSAGE ||--|| RESPONSE : generates
    MESSAGE {
        string content
        int input_tokens
        timestamp created_at
    }
    RESPONSE {
        string content
        int output_tokens
        float cost
    }
    USER {
        string id
        string name
    }
```

## Test 6: Gantt Chart

```mermaid
gantt
    title Rustbot Development Timeline
    dateFormat  YYYY-MM-DD
    section Phase 1
    Core Chat UI           :done, 2024-01-01, 7d
    OpenRouter Integration :done, 2024-01-08, 5d
    section Phase 2
    MCP Support           :done, 2024-01-13, 10d
    Marketplace UI        :done, 2024-01-23, 7d
    section Phase 3
    Mermaid Rendering     :active, 2024-01-30, 3d
    Testing & Polish      :2024-02-02, 5d
```

## Test 7: Pie Chart

```mermaid
pie title Token Usage Distribution
    "Input Tokens" : 35
    "Output Tokens" : 45
    "Cached Tokens" : 15
    "System Prompt" : 5
```

## Test 8: Git Graph

```mermaid
gitGraph
    commit id: "Initial commit"
    branch develop
    checkout develop
    commit id: "Add MCP support"
    commit id: "Add marketplace"
    checkout main
    merge develop
    branch feature/mermaid
    checkout feature/mermaid
    commit id: "Add mermaid module"
    commit id: "Integrate with UI"
    checkout main
    merge feature/mermaid
```

## Instructions for Testing

To test these diagrams in Rustbot:

1. Build and run Rustbot: `cargo build && ./target/debug/rustbot`
2. Copy one of the mermaid code blocks above
3. Paste it into a chat message
4. Send the message
5. The diagram should render as an SVG image inline

## Expected Behavior

- ✅ Diagrams render as SVG images
- ✅ No network requests for cached diagrams
- ✅ Graceful fallback to code block on error
- ✅ Fast rendering (< 500ms for new diagrams)
- ✅ Clear error messages if syntax is invalid

## Error Testing

Test with invalid syntax:

```mermaid
graph TD
    A --> B
    C --> [Invalid syntax here
```

Expected: Should show code block or error message, not crash.
