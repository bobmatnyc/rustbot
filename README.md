# Rustbot

Event-driven AI assistant built in Rust with real-time streaming and extensible tool system.

## Current Status: Phase 1 - POC Development

Building a basic chatbot with OpenRouter integration and streaming responses.

## Documentation

### Getting Started
- **[Development Guide](DEVELOPMENT.md)** - Complete 600+ line development workflow and best practices
- **[Agent Configuration](agents/README.md)** - Agent setup and customization guide
- **[Version Management](VERSION_MANAGEMENT.md)** - Version numbering and release guide

### Architecture & Design
- **[System Architecture](docs/ARCHITECTURE.md)** - Core system design and principles
- **[Agent/Event Architecture](docs/AGENT_EVENT_ARCHITECTURE.md)** - Detailed agent system (46KB spec)
- **[API Documentation](docs/API.md)** - Complete programmatic API reference
- **[Design Documents](docs/design/)** - Feature-specific design specifications

### Project Management
- **[Development Roadmap](docs/PRD/development-plan.md)** - Current phase and planning
- **[Documentation Index](docs/INDEX.md)** - Complete documentation catalog
- **[AI Assistant Guide](CLAUDE.md)** - Quick reference for AI assistants

## Quick Start

### Prerequisites
- Rust 1.70+ (stable)
- OpenRouter API key

### Setup
```bash
# Clone and navigate to project
cd ~/Projects/rustbot

# Create environment file
echo "OPENROUTER_API_KEY=your_key_here" > .env

# Build and run
cargo build
cargo run
```

## Phase 1 Goals

- [x] Project scaffolding and documentation
- [ ] Basic OpenRouter API integration
- [ ] Streaming response handling
- [ ] Simple egui chat interface
- [ ] Wire UI to LLM backend

## Technology Stack

- **Language:** Rust (edition 2021)
- **Async Runtime:** Tokio
- **UI Framework:** egui/eframe
- **HTTP Client:** reqwest
- **LLM Provider:** OpenRouter (Claude Sonnet 4.5)

## Project Structure

```
rustbot/
├── docs/
│   ├── design/          # Architecture and design specs
│   │   └── ARCHITECTURE.md
│   └── PRD/             # Development plans
│       └── development-plan.md
├── src/                 # Source code (to be created)
├── Claude.md            # AI assistant guide
└── README.md           # This file
```

## License

TBD
