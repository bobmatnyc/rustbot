# Rustbot

Event-driven AI assistant built in Rust with real-time streaming and extensible tool system.

## Current Status: Phase 2 Complete ✅ - Production-Ready Architecture

**Version**: 0.2.5 (2025-01-17)
**Status**: Production Ready with AppBuilder & Dependency Injection

### Phase 2 Achievements
- ✅ **80 new tests** added (99.4% pass rate)
- ✅ **AppBuilder pattern** for clean dependency injection
- ✅ **Comprehensive mocking** with mockall
- ✅ **Main.rs integration** complete
- ✅ **Zero breaking changes**
- ✅ **QA-approved** for production

## Documentation

### Phase 2 Documentation (New!) ✅
- **[Phase 2 Complete Guide](docs/architecture/implementation/PHASE2_COMPLETE_GUIDE.md)** - Comprehensive overview
- **[AppBuilder Guide](docs/architecture/implementation/APP_BUILDER_GUIDE.md)** - Dependency injection patterns
- **[Quick Reference](docs/architecture/QUICK_REFERENCE.md)** - Developer cheat sheet
- **[CHANGELOG](CHANGELOG.md)** - Version history and release notes
- **[Phase 2 QA Report](docs/qa/PHASE2_QA_REPORT.md)** - QA validation results

### Getting Started
- **[Development Guide](DEVELOPMENT.md)** - Complete 600+ line development workflow and best practices
- **[Quick Start Refactoring](docs/guides/QUICK_START_REFACTORING.md)** - Phase 2 patterns and examples
- **[Agent Configuration](agents/README.md)** - Agent setup and customization guide
- **[Version Management](VERSION_MANAGEMENT.md)** - Version numbering and release guide

### Architecture & Design
- **[Refactoring Plan](docs/architecture/planning/RUSTBOT_REFACTORING_PLAN.md)** - 4-phase transformation roadmap
- **[System Architecture](docs/ARCHITECTURE.md)** - Core system design and principles
- **[Agent/Event Architecture](docs/AGENT_EVENT_ARCHITECTURE.md)** - Detailed agent system (46KB spec)
- **[API Documentation](docs/API.md)** - Complete programmatic API reference
- **[Design Documents](docs/design/)** - Feature-specific design specifications

### Project Management
- **[Development Roadmap](docs/PRD/development-plan.md)** - Current phase and planning
- **[Documentation Hub](docs/README.md)** - Complete documentation catalog
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

## Refactoring Roadmap

### Phase 1: Service Layer Foundation ✅ Complete (2025-01-17)
- [x] Define trait interfaces (FileSystem, Storage, Config, Agent)
- [x] Implement production services
- [x] Create test implementations
- [x] Comprehensive documentation

### Phase 2: AppBuilder & Dependency Injection ✅ Complete (2025-01-17)
- [x] Fix Phase 1 blockers
- [x] Create mock implementations (80 tests)
- [x] Implement AppBuilder pattern
- [x] Integrate with main.rs
- [x] QA validation (99.4% pass rate)

### Phase 3: UI Decoupling 🔜 Next (2-3 weeks)
- [ ] Migrate UI to use services
- [ ] Remove direct filesystem access
- [ ] Event-driven state updates
- [ ] UI integration testing

### Phase 4: Production Deployment ⏳ Planned (1 week)
- [ ] Final QA validation
- [ ] Performance benchmarking
- [ ] Release v0.3.0
- [ ] Production monitoring

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
