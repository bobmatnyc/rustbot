# Mermaid Diagrams in Rustbot - User Guide

Rustbot now supports rendering Mermaid diagrams directly in chat messages! This guide shows you how to create beautiful diagrams using simple text syntax.

## Quick Start

To create a diagram, use a code block with the `mermaid` language identifier:

````markdown
```mermaid
graph TD
    A[Start] --> B[Process]
    B --> C[End]
```
````

When you send this message, Rustbot will automatically render it as an interactive diagram instead of showing code.

## Supported Diagram Types

### 1. Flowcharts

Perfect for showing processes and decision flows:

````markdown
```mermaid
graph LR
    A[User Request] --> B{Valid?}
    B -->|Yes| C[Process]
    B -->|No| D[Error]
    C --> E[Return Result]
```
````

**Direction options:**
- `TD` or `TB` - Top to bottom
- `LR` - Left to right
- `RL` - Right to left
- `BT` - Bottom to top

### 2. Sequence Diagrams

Great for showing interactions between systems:

````markdown
```mermaid
sequenceDiagram
    participant User
    participant App
    participant API

    User->>App: Click button
    App->>API: Send request
    API-->>App: Return data
    App-->>User: Display result
```
````

### 3. Class Diagrams

Document your code structure:

````markdown
```mermaid
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound()
    }
    class Dog {
        +bark()
    }
    Animal <|-- Dog
```
````

### 4. State Diagrams

Model state machines and workflows:

````markdown
```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Processing: Start
    Processing --> Complete: Success
    Processing --> Error: Failure
    Complete --> [*]
    Error --> Idle: Retry
```
````

### 5. Entity Relationship Diagrams

Database schema visualization:

````markdown
```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    CUSTOMER {
        string name
        string email
    }
    ORDER {
        int orderNumber
        date orderDate
    }
```
````

### 6. Gantt Charts

Project timelines and schedules:

````markdown
```mermaid
gantt
    title Project Schedule
    dateFormat  YYYY-MM-DD
    section Design
    Wireframes      :2024-01-01, 7d
    Mockups         :2024-01-08, 5d
    section Development
    Backend         :2024-01-15, 14d
    Frontend        :2024-01-20, 10d
```
````

### 7. Pie Charts

Data distribution visualization:

````markdown
```mermaid
pie title Expenses
    "Rent" : 35
    "Food" : 25
    "Transport" : 15
    "Entertainment" : 10
    "Savings" : 15
```
````

### 8. Git Graphs

Visualize branching strategies:

````markdown
```mermaid
gitGraph
    commit
    branch develop
    checkout develop
    commit
    commit
    checkout main
    merge develop
    commit
```
````

## Tips and Best Practices

### 1. Keep It Simple
- Start with simple diagrams
- Add complexity gradually
- Break large diagrams into smaller ones

### 2. Use Descriptive Labels
```mermaid
graph TD
    A[User Login] --> B{Credentials Valid?}
    B -->|Yes| C[Dashboard]
    B -->|No| D[Error Message]
```
Better than: `A --> B`, `B --> C`

### 3. Leverage Colors (in some diagram types)
```mermaid
graph LR
    A[Start]:::green --> B[Process]:::blue
    B --> C[End]:::red

    classDef green fill:#9f6,stroke:#333
    classDef blue fill:#69f,stroke:#333
    classDef red fill:#f96,stroke:#333
```

### 4. Add Comments
```mermaid
graph TD
    %% This is a comment - it won't appear in the diagram
    A[Start] --> B[Process]
    B --> C[End]
```

## Common Patterns

### Decision Tree
````markdown
```mermaid
graph TD
    A[Problem] --> B{Type?}
    B -->|Technical| C[Check Logs]
    B -->|User Error| D[Show Help]
    B -->|Unknown| E[Contact Support]
    C --> F[Debug]
    D --> G[Resolve]
    E --> G
    F --> G
    G --> H[Close Ticket]
```
````

### System Architecture
````markdown
```mermaid
graph TB
    Client[Web Browser]
    API[REST API]
    DB[(Database)]
    Cache[Redis Cache]

    Client --> API
    API --> Cache
    Cache --> DB
    API --> DB
```
````

### Workflow Process
````markdown
```mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> Review: Submit
    Review --> Approved: Accept
    Review --> Rejected: Decline
    Rejected --> Draft: Revise
    Approved --> Published
    Published --> [*]
```
````

## Troubleshooting

### Diagram Not Rendering?

**Check syntax:**
- Must use ` ```mermaid` (with backticks)
- Syntax must be valid Mermaid code
- Try simplifying the diagram to isolate issues

**Network connection:**
- First render requires internet connection
- Cached diagrams work offline
- Check if you can access https://mermaid.ink

**Error messages:**
- Invalid syntax shows the code block instead
- Check console logs for detailed error messages

### Diagram Too Large?

If your diagram is very complex:
- Break it into multiple smaller diagrams
- Simplify node labels
- Reduce the number of connections
- Consider a different diagram type

### Caching Issues?

Diagrams are cached for performance:
- Same diagram content reuses cached render
- Changing diagram code creates new render
- Cache clears on app restart (for now)

## Performance Notes

**First Render:**
- Takes 100-500ms (network request to mermaid.ink)
- Subsequent views of same diagram are instant

**Cached Diagrams:**
- Render in <5ms
- No network request needed
- Cache persists during session

**Memory Usage:**
- Each diagram uses ~10-20KB
- Typical usage: <1MB total
- Cache size grows with unique diagrams

## Examples Gallery

See `MERMAID_EXAMPLES.md` for a comprehensive collection of example diagrams you can copy and modify.

## Learn More

**Official Mermaid Documentation:**
- https://mermaid.js.org/
- Comprehensive syntax reference
- Interactive live editor

**Quick Syntax Reference:**
- Flowchart: https://mermaid.js.org/syntax/flowchart.html
- Sequence: https://mermaid.js.org/syntax/sequenceDiagram.html
- Class: https://mermaid.js.org/syntax/classDiagram.html

**Try It Online:**
- Live Editor: https://mermaid.live/
- Test diagrams before using in Rustbot

## Privacy Note

⚠️ **Important:** Diagram code is sent to mermaid.ink API for rendering.

**What this means:**
- Your diagram content is transmitted over HTTPS
- mermaid.ink is a public, free service
- Diagrams are rendered server-side, not stored permanently

**Recommendations:**
- Don't include sensitive data in diagrams
- Don't include API keys, passwords, or secrets
- Keep confidential architecture details private
- Use generic labels for sensitive systems

**Future Enhancement:**
Local rendering option is planned for privacy-sensitive use cases.

## Feedback

Found a bug or have a suggestion?
- Report issues in the project repository
- Share example diagrams that don't render correctly
- Suggest diagram types you'd like to see supported

---

**Happy Diagramming!** 📊✨
