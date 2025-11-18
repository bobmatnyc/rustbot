---
title: Rustbot Quick Start
category: Guide
audience: All
reading_time: 8 minutes
last_updated: 2025-01-17
status: Complete
---

# Rustbot Architecture Refactoring - Quick Start Guide

**⏱️ Reading Time**: 5 minutes
**Goal**: Get oriented fast and find the right documentation for your needs

---

## 🎯 What's Happening?

Rustbot is being refactored from a **monolithic architecture** to a **service-oriented architecture with dependency injection**. This improves:
- ✅ Testability (unit tests without filesystem/network)
- ✅ Maintainability (clear separation of concerns)
- ✅ Flexibility (swap implementations easily)

**Current Status**: **Phase 1 Complete** (25% done, 6-week timeline)

---

## 🧭 Quick Decision Tree: "Which Doc Should I Read?"

```
START HERE
    │
    ├─ 🆕 New to the project?
    │   └─► Read this guide (you're here!)
    │       Then: ARCHITECTURE_INDEX.md
    │
    ├─ 👨‍💼 Project Manager / Stakeholder?
    │   └─► diagrams/REFACTORING_TIMELINE.md (Gantt chart, progress)
    │       Then: ARCHITECTURE_RESEARCH_SUMMARY.md (ADRs)
    │
    ├─ 👨‍💻 Developer implementing refactoring?
    │   └─► RUSTBOT_REFACTORING_PLAN.md (detailed plan)
    │       Then: REFACTORING_CHECKLIST.md (task list)
    │       Then: Start coding!
    │
    ├─ 🔍 Technical Reviewer?
    │   └─► RUST_ARCHITECTURE_BEST_PRACTICES.md (patterns)
    │       Then: diagrams/ARCHITECTURE_DIAGRAMS.md (visual)
    │       Then: ARCHITECTURE_RESEARCH_SUMMARY.md (decisions)
    │
    ├─ 🎓 Learning Rust architecture patterns?
    │   └─► RUST_ARCHITECTURE_BEST_PRACTICES.md (comprehensive guide)
    │       Then: Examples in RUSTBOT_REFACTORING_PLAN.md
    │
    └─ 🧪 Writing tests for new services?
        └─► RUST_ARCHITECTURE_BEST_PRACTICES.md §4 (testing)
            Then: REFACTORING_CHECKLIST.md §Phase 3 (test tasks)
```

---

## ⚡ 5-Minute Overview

### The Problem (Before)

```rust
// ❌ Current: Tight coupling, hard to test
struct RustbotApp {
    // 20+ fields, mixed concerns
    api: Arc<Mutex<RustbotApi>>,  // Direct filesystem access
    agents: Vec<AgentConfig>,      // Hard-coded loading
    // ... many more fields
}

impl RustbotApp {
    fn load_agents(&mut self) {
        // Direct filesystem I/O - cannot mock!
        let files = std::fs::read_dir("agents").unwrap();
        // ...
    }
}
```

**Problems**:
- Can't test without real filesystem
- God Object with 20+ fields
- Mixed UI, business logic, and infrastructure

### The Solution (After)

```rust
// ✅ Proposed: Dependency injection, testable
struct RustbotApp {
    agent_service: Arc<dyn AgentService>,   // Injected!
    storage_service: Arc<dyn StorageService>,
    config_service: Arc<dyn ConfigService>,
}

impl RustbotApp {
    // Constructor injection
    fn new(
        agent_service: Arc<dyn AgentService>,
        storage_service: Arc<dyn StorageService>,
        config_service: Arc<dyn ConfigService>,
    ) -> Self {
        Self { agent_service, storage_service, config_service }
    }
}

// Testing with mocks (no filesystem I/O!)
#[test]
fn test_app() {
    let mock_agents = Arc::new(MockAgentService::new());
    let app = RustbotApp::new(mock_agents, ...);
    // Test without filesystem!
}
```

**Benefits**:
- ✅ Testable with mocks (no I/O)
- ✅ Clear separation of concerns
- ✅ Easy to swap implementations

### Key Concepts (2 minutes)

**Dependency Injection** = Services receive dependencies via constructor, not create them internally

**Trait = Interface** = Contract defining behavior (like Java/C# interfaces)

**Service = Business Logic** = Coordinates domain and infrastructure

**Port = Trait Interface** = Abstract boundary (hexagonal architecture)

**Adapter = Implementation** = Concrete implementation of port (RealFileSystem, MockFileSystem)

---

## 📚 Documentation Map

### Core Documents (Read in Order)

1. **[ARCHITECTURE_INDEX.md](./ARCHITECTURE_INDEX.md)** - Central navigation hub
   - **When**: First time, getting oriented
   - **Time**: 10 minutes
   - **Takeaway**: Understand documentation structure

2. **[ARCHITECTURE_RESEARCH_SUMMARY.md](architecture/planning/ARCHITECTURE_RESEARCH_SUMMARY.md)** - Executive summary
   - **When**: Need high-level overview
   - **Time**: 15 minutes
   - **Takeaway**: Key findings and decisions (ADRs)

3. **[RUSTBOT_REFACTORING_PLAN.md](architecture/planning/RUSTBOT_REFACTORING_PLAN.md)** - Detailed refactoring plan
   - **When**: Ready to implement
   - **Time**: 45 minutes
   - **Takeaway**: Concrete examples and migration strategy

4. **[REFACTORING_CHECKLIST.md](architecture/planning/REFACTORING_CHECKLIST.md)** - Task list
   - **When**: Actively coding
   - **Time**: Reference as needed
   - **Takeaway**: Step-by-step tasks and validation

### Deep Dive Documents (As Needed)

5. **[RUST_ARCHITECTURE_BEST_PRACTICES.md](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)** - Comprehensive guide
   - **When**: Learning Rust architecture patterns
   - **Time**: 2-3 hours (800 lines)
   - **Takeaway**: DI patterns, testing, anti-patterns

6. **[diagrams/ARCHITECTURE_DIAGRAMS.md](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md)** - Visual architecture
   - **When**: Need visual understanding
   - **Time**: 30 minutes
   - **Takeaway**: 10 Mermaid diagrams of architecture

7. **[diagrams/REFACTORING_TIMELINE.md](architecture/diagrams/REFACTORING_TIMELINE.md)** - Project timeline
   - **When**: Project management, progress tracking
   - **Time**: 20 minutes
   - **Takeaway**: Gantt chart, milestones, progress

8. **[diagrams/DATA_FLOW.md](architecture/diagrams/DATA_FLOW.md)** - Message flow analysis
   - **When**: Understanding concurrency and data flow
   - **Time**: 40 minutes
   - **Takeaway**: Before/after message flow, performance

9. **[PHASE1_IMPLEMENTATION_SUMMARY.md](architecture/implementation/PHASE1_IMPLEMENTATION_SUMMARY.md)** - Phase 1 results
   - **When**: Reviewing completed work
   - **Time**: 15 minutes
   - **Takeaway**: What was built, test results, issues

### Quick References

- **[diagrams/README.md](architecture/diagrams/README.md)** - Diagram directory index
- **API.md** - Rustbot API documentation
- **TESTING_METHODS.md** - Testing strategies

---

## 🚀 Fast Track by Role

### I'm a Developer (Implementing Refactoring)

**Read This (30 minutes)**:
1. This guide (you're here!) → 5 min
2. [RUSTBOT_REFACTORING_PLAN.md](architecture/planning/RUSTBOT_REFACTORING_PLAN.md) §1-3 → 15 min
3. [REFACTORING_CHECKLIST.md](architecture/planning/REFACTORING_CHECKLIST.md) (skim) → 10 min

**Then Do This**:
1. Find your phase in checklist (Phase 1 complete, start Phase 2)
2. Follow tasks step-by-step
3. Refer to RUST_ARCHITECTURE_BEST_PRACTICES.md as needed

**Bookmark These**:
- Code examples: RUSTBOT_REFACTORING_PLAN.md §4
- Testing guide: RUST_ARCHITECTURE_BEST_PRACTICES.md §4
- Anti-patterns: RUST_ARCHITECTURE_BEST_PRACTICES.md §5

---

### I'm a Project Manager

**Read This (20 minutes)**:
1. This guide (you're here!) → 5 min
2. [diagrams/REFACTORING_TIMELINE.md](architecture/diagrams/REFACTORING_TIMELINE.md) → 10 min
3. [ARCHITECTURE_RESEARCH_SUMMARY.md](architecture/planning/ARCHITECTURE_RESEARCH_SUMMARY.md) → 5 min

**Key Metrics**:
- **Progress**: 25% complete (Phase 1 done)
- **Timeline**: 6 weeks total (4 weeks remaining)
- **Resources**: ~200 developer hours estimated
- **LOC Impact**: +5,700 LOC (includes tests)
- **Risks**: Low (gradual migration, no breaking changes)

**Milestones**:
- ✅ Phase 1 (Week 1-2): Trait interfaces - COMPLETE
- ⏳ Phase 2 (Week 3-4): Service implementations - IN PROGRESS
- 📋 Phase 3 (Week 5): Mocks and tests - PLANNED
- 📋 Phase 4 (Week 6): UI migration - PLANNED

---

### I'm a Technical Reviewer

**Read This (45 minutes)**:
1. This guide (you're here!) → 5 min
2. [RUST_ARCHITECTURE_BEST_PRACTICES.md](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) §1-3 → 25 min
3. [diagrams/ARCHITECTURE_DIAGRAMS.md](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md) → 15 min

**Key Review Points**:
- Dependency injection: Constructor injection with `Arc<dyn Trait>`
- Service architecture: 4 traits (FileSystem, Storage, Config, Agent)
- Testing: Unit tests with mocks, integration tests with real impls
- Timeline: 6 weeks, phased migration
- Risks: Low (additive changes, no breaking changes)

**Architectural Decisions** (ADRs):
1. Manual DI over framework (simplicity, explicit, zero overhead)
2. `Arc<dyn Trait>` over generics (runtime flexibility, smaller binaries)
3. Layered architecture initially, not hexagonal (YAGNI principle)

---

### I'm Learning Rust Architecture

**Read This (3 hours)**:
1. This guide (you're here!) → 5 min
2. [RUST_ARCHITECTURE_BEST_PRACTICES.md](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md) → 2.5 hours
3. [RUSTBOT_REFACTORING_PLAN.md](architecture/planning/RUSTBOT_REFACTORING_PLAN.md) §4 (examples) → 25 min

**Focus Areas**:
- Trait-based dependency injection (§1)
- Service-oriented architecture (§2)
- Testing strategies with mocks (§4)
- Anti-patterns to avoid (§5)
- Tokio async best practices (§6)

**Hands-On Practice**:
- Copy examples from RUSTBOT_REFACTORING_PLAN.md
- Modify examples to fit your use case
- Run tests to verify understanding

---

## 🔥 Critical Gotchas (Avoid These!)

### 1. mockall + async_trait Ordering

**❌ WRONG** (will not compile):
```rust
#[async_trait]
#[automock]
trait MyTrait { ... }
```

**✅ CORRECT**:
```rust
#[automock]  // Must come FIRST!
#[async_trait]
trait MyTrait { ... }
```

**Why**: Macro expansion order matters. `#[automock]` must see the `async_trait` macro.

**Reference**: RUST_ARCHITECTURE_BEST_PRACTICES.md §4.1

---

### 2. Holding Mutex Across .await

**❌ WRONG** (blocks runtime):
```rust
let mut data = mutex.lock().unwrap();
let result = async_operation().await;  // Lock held during await!
data.update(result);
```

**✅ CORRECT**:
```rust
let result = async_operation().await;
let mut data = mutex.lock().unwrap();
data.update(result);
```

**Why**: Holding mutex across `.await` blocks other tasks. Use `tokio::sync::Mutex` if unavoidable.

**Reference**: RUST_ARCHITECTURE_BEST_PRACTICES.md §6.2

---

### 3. Blocking in Async Functions

**❌ WRONG** (blocks runtime):
```rust
async fn bad() {
    let data = std::fs::read_to_string("file.txt");  // BLOCKS!
}
```

**✅ CORRECT**:
```rust
async fn good() {
    let data = tokio::fs::read_to_string("file.txt").await;
}
```

**Why**: Blocking operations freeze the entire async runtime.

**Reference**: RUST_ARCHITECTURE_BEST_PRACTICES.md §6.1

---

## 📊 Current Progress

### Phase 1 (COMPLETE) ✅

**Completed**:
- [x] FileSystem trait (read, write, exists, create_dir, read_dir)
- [x] StorageService trait (token stats, system prompts)
- [x] ConfigService trait (agents, API key, model)
- [x] AgentService trait (get, list, switch, current)
- [x] RealFileSystem implementation
- [x] FileStorageService implementation
- [x] FileConfigService implementation
- [x] DefaultAgentService implementation
- [x] 16/22 tests passing (73%)

**Known Issues**:
- ⚠️ 6 agent service tests failing (tokio runtime in test setup)
- Resolution: Fix in Phase 2 with proper test infrastructure

---

### Phase 2 (IN PROGRESS) ⏳

**Next Tasks**:
- [ ] Fix agent service tests (tokio runtime issue)
- [ ] Create AppBuilder for dependency construction
- [ ] Add mock implementations (MockFileSystem, etc.)
- [ ] Integration tests with AppBuilder

**ETA**: 2 weeks remaining

---

## 🛠️ Essential Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_agent_service

# Build project
cargo build

# Format code
cargo fmt

# Lint code
cargo clippy --all-targets --all-features

# Build docs
cargo doc --open

# Run application
./target/debug/rustbot
```

---

## 🆘 Getting Help

### Common Questions

**Q: Where do I start coding?**
A: Check [REFACTORING_CHECKLIST.md](architecture/planning/REFACTORING_CHECKLIST.md) for your current phase tasks.

**Q: How do I test without filesystem?**
A: Use `MockFileSystem`. See example in RUSTBOT_REFACTORING_PLAN.md §4.1.

**Q: Why use `Arc<dyn Trait>` instead of generics?**
A: Runtime flexibility and smaller binaries. See ADR-2 in ARCHITECTURE_RESEARCH_SUMMARY.md.

**Q: What's the difference between `std::sync::Mutex` and `tokio::sync::Mutex`?**
A: Use `std::sync` for short locks, `tokio::sync` if holding across `.await`. See RUST_ARCHITECTURE_BEST_PRACTICES.md §6.2.

**Q: How do I add a new service?**
A: Follow Phase 1 pattern: (1) Define trait, (2) Implement real version, (3) Add tests, (4) Create mock for testing.

### Stuck? Read These

**Compile errors**:
→ RUST_ARCHITECTURE_BEST_PRACTICES.md §5 (Anti-patterns)

**Test failures**:
→ RUST_ARCHITECTURE_BEST_PRACTICES.md §4 (Testing)

**Architecture questions**:
→ ARCHITECTURE_RESEARCH_SUMMARY.md (ADRs)

**Timeline questions**:
→ diagrams/REFACTORING_TIMELINE.md

---

## 📖 Full Documentation Index

1. **[QUICK_START.md](guides/QUICK_START.md)** ← You are here
2. **[ARCHITECTURE_INDEX.md](./ARCHITECTURE_INDEX.md)** - Central navigation
3. **[RUST_ARCHITECTURE_BEST_PRACTICES.md](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)** - Patterns guide
4. **[RUSTBOT_REFACTORING_PLAN.md](architecture/planning/RUSTBOT_REFACTORING_PLAN.md)** - Detailed plan
5. **[ARCHITECTURE_RESEARCH_SUMMARY.md](architecture/planning/ARCHITECTURE_RESEARCH_SUMMARY.md)** - Executive summary
6. **[REFACTORING_CHECKLIST.md](architecture/planning/REFACTORING_CHECKLIST.md)** - Task list
7. **[PHASE1_IMPLEMENTATION_SUMMARY.md](architecture/implementation/PHASE1_IMPLEMENTATION_SUMMARY.md)** - Phase 1 results
8. **[diagrams/ARCHITECTURE_DIAGRAMS.md](architecture/diagrams/ARCHITECTURE_DIAGRAMS.md)** - Visual architecture
9. **[diagrams/REFACTORING_TIMELINE.md](architecture/diagrams/REFACTORING_TIMELINE.md)** - Timeline
10. **[diagrams/DATA_FLOW.md](architecture/diagrams/DATA_FLOW.md)** - Message flow
11. **[diagrams/README.md](architecture/diagrams/README.md)** - Diagram index

---

## ✅ Ready to Start?

**Choose your path**:

- 🏃 **I'm ready to code** → [REFACTORING_CHECKLIST.md](architecture/planning/REFACTORING_CHECKLIST.md)
- 📚 **I need more context** → [RUSTBOT_REFACTORING_PLAN.md](architecture/planning/RUSTBOT_REFACTORING_PLAN.md)
- 🎓 **I want to learn patterns** → [RUST_ARCHITECTURE_BEST_PRACTICES.md](architecture/best-practices/RUST_ARCHITECTURE_BEST_PRACTICES.md)
- 🗺️ **I want full overview** → [ARCHITECTURE_INDEX.md](./ARCHITECTURE_INDEX.md)

**Good luck! 🚀**

---

**Guide Version**: 1.0
**Last Updated**: November 17, 2025
**Estimated Reading Time**: 5 minutes
**Next Review**: After Phase 2 completion
