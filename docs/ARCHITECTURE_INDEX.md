# Rustbot Architecture Documentation Index

This document serves as the central index for all architecture-related documentation in the Rustbot project.

---

## ⚡ Quick Start

**New to the refactoring project?** Start here: **[QUICK_START.md](./QUICK_START.md)**

- **5-minute orientation** for new developers
- **Decision tree**: "Which doc should I read?"
- **Fast-track guides** by role (developer, PM, reviewer)
- **Critical gotchas** to avoid

---

## 📚 Documentation Structure

### Core Architecture Guides

1. **[QUICK_START.md](./QUICK_START.md)** **← START HERE**
   - **Purpose**: 5-minute orientation and fast-track guide
   - **Audience**: Everyone (first-time readers)
   - **Length**: ~5 minutes reading time
   - **Topics**:
     - Decision tree: "Which doc should I read?"
     - Fast-track guides by role
     - Critical gotchas to avoid
     - Current progress overview
   - **Use When**: First time, need quick orientation

2. **[RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md)**
   - **Purpose**: Comprehensive guide to Rust architectural patterns (general, applicable to any Rust project)
   - **Audience**: Rust developers, architects
   - **Length**: ~800 lines
   - **Topics**:
     - Dependency injection patterns in Rust
     - Service-oriented architecture
     - Hexagonal and clean architecture
     - Testing strategies (unit, integration, property-based)
     - Anti-patterns to avoid
     - Tokio async best practices
   - **Use When**: Learning Rust architecture patterns, making architectural decisions

3. **[RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md)**
   - **Purpose**: Concrete refactoring plan specific to Rustbot codebase
   - **Audience**: Rustbot contributors
   - **Length**: ~600 lines
   - **Topics**:
     - Current architecture assessment
     - Proposed architecture with diagrams
     - Phase-by-phase migration strategy (6 weeks)
     - Before/after code examples
     - Testing strategy
     - Success criteria
   - **Use When**: Implementing the refactoring, understanding the migration path

4. **[ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md)**
   - **Purpose**: Executive summary of architecture research findings
   - **Audience**: Decision-makers, quick reference
   - **Length**: ~400 lines
   - **Topics**:
     - Key findings (bullet-point format)
     - Architectural Decision Records (ADRs)
     - Recommendations for Rustbot
     - Next steps
   - **Use When**: Quick reference, presenting to stakeholders

5. **[REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md)**
   - **Purpose**: Step-by-step implementation checklist
   - **Audience**: Developers implementing the refactoring
   - **Length**: ~300 lines
   - **Topics**:
     - Phase-by-phase task lists
     - Validation criteria
     - Common pitfalls to avoid
     - Success indicators
     - Quick commands
   - **Use When**: Actively implementing refactoring, tracking progress

---

## 🗂️ Quick Navigation

### By Task

| What You Need | Where to Look |
|---------------|---------------|
| **Learn Rust DI patterns** | [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §1 |
| **Understand service architecture** | [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §2 |
| **See Rustbot-specific examples** | [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) §4 |
| **Start refactoring** | [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md) |
| **Understand testing strategy** | [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §4 |
| **Avoid anti-patterns** | [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §5 |
| **Quick reference ADRs** | [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md) §ADRs |
| **Implementation timeline** | [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) §3 |

### By Audience

| Audience | Recommended Reading Order |
|----------|---------------------------|
| **New to Rust Architecture** | 1. Summary → 2. Best Practices → 3. Refactoring Plan → 4. Checklist |
| **Experienced Rust Developer** | 1. Summary → 2. Refactoring Plan → 3. Checklist |
| **Implementing Refactoring** | 1. Refactoring Plan → 2. Checklist → 3. Best Practices (reference) |
| **Project Manager/Stakeholder** | 1. Summary (ADRs section) → 2. Refactoring Plan (timeline) |

---

## 🎯 Key Concepts

### Dependency Injection in Rust

**Pattern**: Trait-based constructor injection

```rust
// Define trait (port)
#[async_trait]
trait FileSystem: Send + Sync {
    async fn read_to_string(&self, path: &Path) -> Result<String>;
}

// Service depends on trait, not concrete type
struct AgentService {
    filesystem: Arc<dyn FileSystem>,
}

// Constructor injection
impl AgentService {
    pub fn new(filesystem: Arc<dyn FileSystem>) -> Self {
        Self { filesystem }
    }
}
```

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §1

### Service-Oriented Architecture

**Layers**:
1. **Domain**: Pure business logic (no infrastructure dependencies)
2. **Service**: Application logic (coordinates domain + infrastructure)
3. **Repository**: Data access abstraction (trait interfaces)
4. **Infrastructure**: External systems (filesystem, network, DB)

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §2

### Testing Strategy

**Unit Tests**: Mock all dependencies (no I/O)
**Integration Tests**: Real implementations in isolated environments
**Property-Based Tests**: Invariants with `proptest`

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §4

---

## 📋 Architectural Decision Records (ADRs)

### ADR-1: Manual DI Over Framework
**Decision**: Use manual trait-based DI instead of framework
**Rationale**: Rustbot is small (~5k LOC), manual DI is simple and explicit
**Read More**: [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md) §ADR-1

### ADR-2: Arc<dyn Trait> for Runtime Polymorphism
**Decision**: Use `Arc<dyn Trait>` instead of generic bounds
**Rationale**: Runtime flexibility, smaller binary, easier UI integration
**Read More**: [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md) §ADR-2

### ADR-3: Layered Architecture (Not Hexagonal)
**Decision**: Start with simple layered architecture, defer hexagonal
**Rationale**: Current complexity doesn't justify hexagonal, YAGNI principle
**Read More**: [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md) §ADR-3

---

## 🗓️ Refactoring Timeline

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| **Phase 1** | Week 1-2 | Trait definitions (FileSystem, ConfigService, AgentService, McpService) |
| **Phase 2** | Week 3-4 | Service implementations (RealFileSystem, DefaultAgentService, AppBuilder) |
| **Phase 3** | Week 5 | Mock implementations, unit tests, integration tests |
| **Phase 4** | Week 6 | UI migration, deprecate old code, documentation updates |

**Total Duration**: 6 weeks
**Read More**: [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) §3

---

## 🧪 Testing Philosophy

### Test Pyramid

```
     /\
    /  \  Unit Tests (70%)
   /____\  - Fast (<1s total)
  /      \ - No I/O (mocks)
 /  Int.  \ Integration Tests (20%)
/__________\  - Real implementations
/   E2E     \ - Isolated environments
/____________\ End-to-End Tests (10%)
                - Full app testing
```

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §4

---

## 🚨 Common Pitfalls

### Anti-Patterns to Avoid

1. **Excessive `clone()`**: Use borrowing, `Cow<T>`, or `Arc`
2. **`unwrap()` everywhere**: Use `?` operator for error propagation
3. **Concrete dependencies**: Depend on `Arc<dyn Trait>`, not concrete types
4. **Blocking in async**: Use `tokio::fs`, not `std::fs`
5. **Global state**: Use constructor injection instead

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §5

### Critical Gotcha: mockall + async_trait

**Correct Order**:
```rust
#[automock]  // ← Must come FIRST
#[async_trait]
trait MyTrait {
    async fn foo(&self) -> u32;
}
```

**Wrong Order** (will not compile):
```rust
#[async_trait]
#[automock]  // ← Wrong!
trait MyTrait { ... }
```

**Read More**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §4.1

---

## 📊 Success Metrics

### Quantitative
- ✅ **Test Coverage**: >70% for service layer
- ✅ **Unit Test Speed**: <1 second (no I/O)
- ✅ **Code Duplication**: Reduced by >30%
- ✅ **Integration Test Coverage**: All public service methods

### Qualitative
- ✅ Services decoupled from infrastructure
- ✅ Business logic testable in isolation
- ✅ Easy to swap implementations
- ✅ Clear separation of concerns

**Read More**: [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) §6

---

## 🔗 External Resources

### Official Documentation
- [Rust Design Patterns Book](https://rust-unofficial.github.io/patterns/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [async-trait Crate](https://docs.rs/async-trait)
- [mockall Documentation](https://docs.rs/mockall)

### Community Resources
- [Hexagonal Architecture in Rust](https://www.howtocodeit.com/articles/master-hexagonal-architecture-rust)
- [Microsoft's Rust Actix Clean Architecture Template](https://github.com/microsoft/cookiecutter-rust-actix-clean-architecture)

---

## 📝 Session Logs

Progress logs documenting implementation work:

- **[2025-01-17: Architecture Research](./progress/2025-01-17-architecture-research.md)**
  - Research methodology and findings
  - Web search queries and sources
  - Code examples and insights
  - Session statistics

---

## 🎓 Learning Path

### For New Contributors

**⚡ Fast Track** (5 minutes):
1. [QUICK_START.md](./QUICK_START.md) - Get oriented quickly

**Complete Path** (2-3 hours):
1. **Start Here**: [QUICK_START.md](./QUICK_START.md)
   - 5-minute orientation
   - Decision tree for finding right docs

2. **Overview**: [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md)
   - Quick overview of key concepts
   - Architectural decisions with rationale

3. **Deep Dive**: [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md)
   - Comprehensive patterns and examples
   - Testing strategies
   - Anti-patterns to avoid

4. **Apply to Rustbot**: [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md)
   - See how patterns apply to actual codebase
   - Before/after examples
   - Migration strategy

5. **Implement**: [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md)
   - Step-by-step tasks
   - Validation criteria
   - Common pitfalls

### For Experienced Rust Developers

**Fast Track** (1 hour):
1. [QUICK_START.md](./QUICK_START.md) - Decision tree and gotchas
2. [ARCHITECTURE_RESEARCH_SUMMARY.md](./ARCHITECTURE_RESEARCH_SUMMARY.md) (ADRs section)
3. [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) (concrete examples)
4. [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md) (implementation tasks)

---

## 🔄 Document Relationships

```
ARCHITECTURE_INDEX.md (you are here)
    ├── RUST_ARCHITECTURE_BEST_PRACTICES.md
    │   ├── Dependency Injection patterns
    │   ├── Service architecture patterns
    │   ├── Testing strategies
    │   └── Anti-patterns
    │
    ├── RUSTBOT_REFACTORING_PLAN.md
    │   ├── Current state assessment
    │   ├── Proposed architecture
    │   ├── Migration strategy
    │   └── Concrete examples
    │
    ├── ARCHITECTURE_RESEARCH_SUMMARY.md
    │   ├── Key findings
    │   ├── ADRs
    │   ├── Recommendations
    │   └── Next steps
    │
    ├── REFACTORING_CHECKLIST.md
    │   ├── Phase 1-4 tasks
    │   ├── Validation criteria
    │   └── Success indicators
    │
    └── progress/2025-01-17-architecture-research.md
        ├── Research methodology
        ├── Session statistics
        └── Lessons learned
```

---

## 📞 Getting Help

### Questions About Architecture?
- Read [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md)
- Check [Rust Design Patterns Book](https://rust-unofficial.github.io/patterns/)
- Ask in Rustbot discussions

### Questions About Refactoring Plan?
- Read [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md)
- Check [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md)
- Review session logs in `progress/`

### Stuck on Implementation?
- Consult [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md) (Common Pitfalls section)
- Review code examples in [RUSTBOT_REFACTORING_PLAN.md](./RUSTBOT_REFACTORING_PLAN.md) §4
- Check anti-patterns in [RUST_ARCHITECTURE_BEST_PRACTICES.md](./RUST_ARCHITECTURE_BEST_PRACTICES.md) §5

---

## 📅 Last Updated

- **Index Created**: January 17, 2025
- **Research Completed**: January 17, 2025
- **Documentation Version**: 1.0
- **Status**: Ready for Implementation

---

## 🎯 Next Steps

1. **Review**: Read through documentation suite
2. **Discuss**: Review with team/stakeholders
3. **Approve**: Get sign-off on refactoring approach
4. **Begin**: Start Phase 1 (extract trait interfaces)
5. **Track**: Update [REFACTORING_CHECKLIST.md](./REFACTORING_CHECKLIST.md) as you progress

---

**Happy refactoring! 🚀**
