# Rust Architecture Research Summary

**Date**: January 17, 2025
**Objective**: Research Rust best practices for dependency injection, service-oriented architecture, and architectural patterns to inform Rustbot refactoring.

---

## Executive Summary

Comprehensive research completed on Rust architectural patterns, dependency injection, and service-oriented design. Key findings:

1. **Dependency Injection in Rust** is achieved through **trait-based abstractions** and **constructor injection**, not runtime reflection frameworks.

2. **Service-Oriented Architecture** works well in Rust using the **repository pattern** and **hexagonal architecture** (ports and adapters).

3. **Testing strategies** leverage `mockall` crate or manual mock implementations for unit tests, with clear separation between business logic and infrastructure.

4. **Anti-patterns** to avoid include excessive cloning, unwrapping, concrete dependencies in services, and blocking code in async contexts.

---

## Key Findings

### 1. Dependency Injection Patterns

**Recommended Approach**: Manual trait-based DI over frameworks

| Pattern | Use Case | Example |
|---------|----------|---------|
| **Generic Bounds** | Compile-time polymorphism | `struct Service<R: Repository>` |
| **Trait Objects** | Runtime polymorphism | `Arc<dyn Repository>` |
| **Builder Pattern** | Complex dependency graphs | `AppBuilder::new().build()` |

**Verdict for Rustbot**: Use `Arc<dyn Trait>` for runtime flexibility without framework overhead.

### 2. Service Layer Architecture

**Repository Pattern**:
- Abstract data access behind trait interfaces
- Separate domain logic from infrastructure
- Easy to test with mock repositories

**Service Layer**:
- Coordinate between domain and infrastructure
- Handle transactions and business logic
- Depend on abstractions (traits), not concrete types

**Example Structure**:
```
Domain Layer (pure logic)
    ↑ uses
Service Layer (business logic)
    ↑ uses traits
Repository Layer (data access abstraction)
    ↑ implements
Infrastructure Layer (filesystem, network, DB)
```

### 3. Architectural Patterns

**Hexagonal Architecture (Ports & Adapters)**:
- **Ports**: Trait interfaces defined by business logic
- **Adapters**: Implementations for external systems
- **Benefit**: Business logic independent of infrastructure

**Clean Architecture**:
- Dependency direction: Outer layers → Inner layers
- Domain layer has zero external dependencies
- Use cases depend only on domain

**When to Use**:
- ✅ Web services, microservices, evolving applications
- ❌ Simple CLIs, one-off scripts, prototypes

**Recommendation for Rustbot**: Start with **simple layered architecture**, consider hexagonal if MCP integrations become complex.

### 4. Testing Strategies

**Unit Testing with Mocks**:
```rust
#[automock]  // Must come BEFORE #[async_trait]
#[async_trait]
trait Repository {
    async fn find(&self, id: u64) -> Result<User>;
}
```

**Test Doubles**:
- `mockall` crate for automatic mocking
- Manual test implementations for simpler cases
- Integration tests with real implementations in isolated environments

**Critical Ordering**: `#[automock]` must appear before `#[async_trait]` (common gotcha).

### 5. Tokio Async Best Practices

**Key Rules**:
1. Never block in async functions (use `tokio::task::spawn_blocking`)
2. Use `std::sync::Mutex` for short locks, not `tokio::sync::Mutex`
3. Prefer message passing (channels) over `Arc<Mutex<T>>`
4. Implement graceful shutdown with broadcast channels

### 6. Anti-Patterns to Avoid

| Anti-Pattern | Why Bad | Solution |
|--------------|---------|----------|
| Excessive `clone()` | Performance, indicates design issue | Use borrowing, `Cow<T>`, `Arc` |
| `unwrap()` everywhere | Panics, not recoverable | Use `?` operator, proper error handling |
| `String` when `&str` works | Unnecessary allocations | Accept `&str` in function params |
| Concrete dependencies | Hard to test, tight coupling | Depend on `Arc<dyn Trait>` |
| Global state | Hidden dependencies, hard to test | Constructor injection |
| Blocking in async | Blocks entire runtime | Use `tokio::fs`, `spawn_blocking` |

---

## Research Sources

### Authoritative Resources
- **Rust Design Patterns Book**: https://rust-unofficial.github.io/patterns/
- **Tokio Documentation**: https://tokio.rs/
- **Hexagonal Architecture in Rust**: https://www.howtocodeit.com/articles/master-hexagonal-architecture-rust
- **mockall Documentation**: https://docs.rs/mockall
- **async-trait Crate**: https://docs.rs/async-trait

### Community Insights
- Dependency injection discussions on Rust forums
- Microsoft's Cookiecutter Rust Actix Clean Architecture template
- Multiple production examples of DDD, clean architecture in Rust
- Recent articles on trait-based DI (2024-2025)

---

## Recommendations for Rustbot

### Immediate Actions (Phase 1)

1. **Extract Trait Interfaces**:
   - `FileSystem` trait for file operations
   - `ConfigService` trait for configuration
   - `AgentService` trait for agent management
   - `McpService` trait for MCP operations

2. **Create Service Implementations**:
   - `RealFileSystem` wrapping `tokio::fs`
   - `FileConfigService` loading `.env` files
   - `DefaultAgentService` using `FileSystem` trait
   - `DefaultMcpService` using process spawning

3. **Add Builder Pattern**:
   - `AppBuilder` for dependency construction
   - Separate `with_production_deps()` and `with_test_deps()` methods

### Testing Strategy

1. **Unit Tests**: Use mock implementations (no filesystem/network)
2. **Integration Tests**: Use real implementations in isolated environments
3. **Property-Based Tests**: For invariants and edge cases (with `proptest`)

### Migration Path

**DO NOT** attempt big-bang rewrite. Instead:

1. Week 1-2: Extract traits (no behavior change)
2. Week 3-4: Implement services with DI
3. Week 5: Add comprehensive tests
4. Week 6: Migrate UI to use new services

### Success Criteria

- ✅ Can run all tests without filesystem/network
- ✅ Services decoupled from infrastructure
- ✅ Test coverage >70% for service layer
- ✅ Can swap implementations (file config → env config)
- ✅ No breaking changes to existing functionality

---

## Architectural Decision Records

### ADR-1: Use Manual DI Over Framework

**Decision**: Use manual trait-based dependency injection instead of a DI framework.

**Rationale**:
- Rustbot codebase is small (~5k LOC)
- Manual DI is simple, explicit, and has zero overhead
- Frameworks add complexity and learning curve
- Rust's type system provides compile-time safety without framework

**Alternatives Considered**:
- `teloc`: Compile-time DI framework
- `coi`: Container-based DI
- **Rejected**: Too much overhead for current scale

### ADR-2: Use Arc<dyn Trait> for Runtime Polymorphism

**Decision**: Use `Arc<dyn Trait>` instead of generic bounds for service dependencies.

**Rationale**:
- Runtime flexibility to swap implementations
- Smaller binary size (no monomorphization)
- Easier to work with in UI layer (egui)
- Slight performance cost acceptable for Rustbot's use case

**Trade-off**: ~1-2ns vtable lookup vs. compile-time dispatch (negligible for I/O-bound operations).

### ADR-3: Start with Layered Architecture, Not Hexagonal

**Decision**: Begin with simple layered architecture, defer hexagonal to future if needed.

**Rationale**:
- Current complexity doesn't justify hexagonal
- Can refactor to hexagonal later if MCP integrations become complex
- Simple layers easier to understand and maintain
- YAGNI principle (You Aren't Gonna Need It)

**Extension Point**: If agent count exceeds 100 or multi-tenant deployment needed, revisit hexagonal.

---

## Knowledge Gaps & Future Research

### Resolved ✅
- ✅ Dependency injection patterns in Rust
- ✅ Service layer architecture
- ✅ Testing strategies with mocks
- ✅ Tokio async best practices
- ✅ Anti-patterns to avoid

### Open Questions ❓
- ⏳ Domain-driven design in Rust (web search failed, defer to later)
- ⏳ Event sourcing patterns (not critical for Rustbot)
- ⏳ CQRS in Rust (over-engineering for current needs)

---

## Next Steps

1. **Review Documentation**: Share this research with team/user
2. **Get Approval**: Confirm refactoring approach aligns with project goals
3. **Start Phase 1**: Extract trait interfaces (low risk, additive only)
4. **Create Session Log**: Document progress in `docs/progress/2025-01-17-architecture-research.md`

---

## Deliverables

| Document | Purpose | Status |
|----------|---------|--------|
| `RUST_ARCHITECTURE_BEST_PRACTICES.md` | Comprehensive guide to Rust architectural patterns | ✅ Complete |
| `RUSTBOT_REFACTORING_PLAN.md` | Concrete refactoring plan with code examples | ✅ Complete |
| `ARCHITECTURE_RESEARCH_SUMMARY.md` | Executive summary of research findings | ✅ Complete |
| Session log (`2025-01-17-session.md`) | Progress tracking for continuity | ⏳ To be created |

---

## Appendix: Code Examples

### Example 1: Service with Dependency Injection

```rust
// Service depends on trait, not concrete implementation
pub struct AgentService {
    filesystem: Arc<dyn FileSystem>,
    config: Arc<dyn ConfigService>,
}

impl AgentService {
    pub fn new(
        filesystem: Arc<dyn FileSystem>,
        config: Arc<dyn ConfigService>,
    ) -> Self {
        Self { filesystem, config }
    }

    pub async fn load_all(&self) -> Result<Vec<Agent>> {
        let agents_dir = self.config.get_agents_dir();
        let entries = self.filesystem.read_dir(&agents_dir).await?;
        // ... load agents
    }
}
```

### Example 2: Mock for Testing

```rust
#[cfg(test)]
struct MockFileSystem {
    files: HashMap<PathBuf, String>,
}

#[cfg(test)]
#[async_trait]
impl FileSystem for MockFileSystem {
    async fn read_to_string(&self, path: &Path) -> Result<String> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| Error::NotFound)
    }
}

#[tokio::test]
async fn test_agent_loading() {
    let mut mock_fs = MockFileSystem::new();
    mock_fs.add_file("agent.json", r#"{"name": "test"}"#);

    let service = AgentService::new(Arc::new(mock_fs), Arc::new(mock_config));
    let agents = service.load_all().await.unwrap();

    assert_eq!(agents.len(), 1);
}
```

### Example 3: Application Builder

```rust
pub struct AppBuilder {
    config: Option<Arc<dyn ConfigService>>,
    filesystem: Option<Arc<dyn FileSystem>>,
}

impl AppBuilder {
    pub fn with_production_deps(mut self) -> Result<Self> {
        self.config = Some(Arc::new(FileConfigService::load()?));
        self.filesystem = Some(Arc::new(RealFileSystem));
        Ok(self)
    }

    pub fn build(self) -> Result<App> {
        let config = self.config.expect("Config required");
        let fs = self.filesystem.expect("Filesystem required");

        let agent_service = Arc::new(AgentService::new(fs, config.clone()));

        Ok(App { agent_service, config })
    }
}
```

---

**Document Version**: 1.0
**Research Completed**: January 17, 2025
**Next Review**: After Phase 1 implementation
**Status**: Ready for Implementation
