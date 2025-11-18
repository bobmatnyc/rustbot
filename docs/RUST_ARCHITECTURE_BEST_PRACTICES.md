# Rust Architecture Best Practices

## Research Summary (January 2025)

This document provides comprehensive guidance on dependency injection, service-oriented architecture, and architectural patterns in Rust, specifically tailored for the Rustbot project. All information is based on current Rust best practices as of January 2025.

---

## Table of Contents

1. [Dependency Injection in Rust](#dependency-injection-in-rust)
2. [Service-Oriented Architecture](#service-oriented-architecture)
3. [Architectural Patterns](#architectural-patterns)
4. [Testing Strategies](#testing-strategies)
5. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
6. [Tokio Async Best Practices](#tokio-async-best-practices)
7. [Recommendations for Rustbot](#recommendations-for-rustbot)

---

## 1. Dependency Injection in Rust

### Overview

Rust achieves dependency injection through **trait-based abstractions** and **constructor injection**, leveraging the type system and ownership model instead of runtime reflection.

### Key Approaches

#### 1.1 Constructor Injection with Trait Bounds (Compile-Time Polymorphism)

**Use when**: You know all types at compile time and want zero-cost abstractions.

```rust
// Define trait interface (contract)
trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, DbError>;
    async fn save(&self, user: &User) -> Result<(), DbError>;
}

// Service depends on trait, not concrete implementation
struct UserService<R: UserRepository> {
    repository: R,
    cache: Arc<dyn Cache>,
}

impl<R: UserRepository> UserService<R> {
    // Constructor injection
    pub fn new(repository: R, cache: Arc<dyn Cache>) -> Self {
        Self { repository, cache }
    }

    pub async fn get_user(&self, id: u64) -> Result<User, ServiceError> {
        // Check cache first
        if let Some(cached) = self.cache.get(&format!("user:{}", id)).await? {
            return Ok(cached);
        }

        // Fetch from repository
        let user = self.repository.find_by_id(id).await?
            .ok_or(ServiceError::NotFound)?;

        // Update cache
        self.cache.set(&format!("user:{}", id), &user).await?;

        Ok(user)
    }
}
```

**Benefits**:
- Zero-cost abstraction (monomorphization)
- Compile-time type checking
- No vtable overhead

**Drawbacks**:
- Type must be known at compile time
- Can lead to code bloat with many implementations

#### 1.2 Trait Objects for Runtime Polymorphism (Dynamic Dispatch)

**Use when**: Implementation type needs to be determined at runtime or you want to avoid code bloat.

```rust
// Use trait objects when type must be determined at runtime
struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: Arc<dyn Cache>,
}

impl UserService {
    pub fn new(
        repository: Arc<dyn UserRepository>,
        cache: Arc<dyn Cache>,
    ) -> Self {
        Self { repository, cache }
    }

    pub async fn get_user(&self, id: u64) -> Result<User, ServiceError> {
        // Implementation same as above
        todo!()
    }
}
```

**Benefits**:
- Runtime flexibility
- Smaller binary size (no monomorphization)
- Easy to swap implementations

**Drawbacks**:
- Slight runtime overhead (vtable lookup)
- Requires `dyn Trait` with size-known wrapper (`Box`, `Arc`, `&`)

#### 1.3 Builder Pattern for Complex Construction

**Use when**: Services have many dependencies or optional configuration.

```rust
struct AppBuilder {
    db_url: Option<String>,
    cache_ttl: Option<Duration>,
    log_level: Option<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            db_url: None,
            cache_ttl: None,
            log_level: None,
        }
    }

    pub fn with_database(mut self, url: String) -> Self {
        self.db_url = Some(url);
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    pub async fn build(self) -> Result<App, BuildError> {
        let db_url = self.db_url.ok_or(BuildError::MissingDatabase)?;
        let cache_ttl = self.cache_ttl.unwrap_or(Duration::from_secs(300));

        // Construct dependencies
        let db_pool = create_pool(&db_url).await?;
        let repository = Arc::new(PostgresUserRepository::new(db_pool));
        let cache = Arc::new(RedisCache::new(cache_ttl));

        // Inject into services
        let user_service = Arc::new(UserService::new(repository, cache));

        Ok(App { user_service })
    }
}

// Usage
let app = AppBuilder::new()
    .with_database("postgres://localhost/db".to_string())
    .with_cache_ttl(Duration::from_secs(600))
    .build()
    .await?;
```

### DI Frameworks vs Manual Patterns

**Manual Patterns (Recommended for Most Projects)**:
- Simple, explicit, no magic
- Compile-time safety
- Easy to understand and debug

**DI Frameworks** (e.g., `teloc`, `coi`):
- Useful for very large codebases
- Auto-wiring of dependencies
- Can reduce boilerplate
- **Tradeoff**: Added complexity and learning curve

**Recommendation for Rustbot**: Use manual trait-based DI. The codebase is small enough that a framework would add more complexity than value.

---

## 2. Service-Oriented Architecture

### Overview

Service-oriented architecture in Rust separates concerns into distinct layers:
- **Domain Layer**: Business logic and domain models
- **Service Layer**: Application logic coordinating domain and infrastructure
- **Repository Layer**: Data access abstraction
- **Infrastructure Layer**: External dependencies (HTTP, DB, files)

### 2.1 Repository Pattern

**Purpose**: Abstract data access behind trait interfaces, allowing business logic to be decoupled from storage details.

```rust
use async_trait::async_trait;

// Repository trait (port)
#[async_trait]
trait Repository<T>: Send + Sync {
    async fn find(&self, id: u64) -> Result<Option<T>, DbError>;
    async fn save(&self, entity: &T) -> Result<(), DbError>;
    async fn delete(&self, id: u64) -> Result<(), DbError>;
}

// Concrete implementation (adapter)
struct PostgresUserRepository {
    pool: PgPool,
}

#[async_trait]
impl Repository<User> for PostgresUserRepository {
    async fn find(&self, id: u64) -> Result<Option<User>, DbError> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn save(&self, user: &User) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO users (id, email, name) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET email = $2, name = $3",
            user.id as i64, user.email, user.name
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: u64) -> Result<(), DbError> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// In-memory implementation for testing
struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<u64, User>>>,
}

#[async_trait]
impl Repository<User> for InMemoryUserRepository {
    async fn find(&self, id: u64) -> Result<Option<User>, DbError> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DbError> {
        let mut users = self.users.write().await;
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn delete(&self, id: u64) -> Result<(), DbError> {
        let mut users = self.users.write().await;
        users.remove(&id);
        Ok(())
    }
}
```

### 2.2 Service Layer Pattern

**Purpose**: Encapsulate business logic, coordinate between domain and infrastructure, handle transactions.

```rust
// Service layer coordinates multiple repositories
struct UserService {
    user_repo: Arc<dyn Repository<User>>,
    audit_repo: Arc<dyn Repository<AuditLog>>,
    event_bus: Arc<dyn EventBus>,
}

impl UserService {
    pub fn new(
        user_repo: Arc<dyn Repository<User>>,
        audit_repo: Arc<dyn Repository<AuditLog>>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            user_repo,
            audit_repo,
            event_bus,
        }
    }

    pub async fn create_user(
        &self,
        email: String,
        name: String,
    ) -> Result<User, ServiceError> {
        // Business logic: validate email
        if !email.contains('@') {
            return Err(ServiceError::InvalidEmail);
        }

        // Create domain object
        let user = User::new(email, name);

        // Save to repository
        self.user_repo.save(&user).await?;

        // Create audit log
        let audit = AuditLog::new(
            format!("User created: {}", user.id),
            user.id,
        );
        self.audit_repo.save(&audit).await?;

        // Publish event
        self.event_bus.publish(UserCreatedEvent {
            user_id: user.id,
            email: user.email.clone(),
        }).await?;

        Ok(user)
    }
}
```

### 2.3 Domain Layer

**Purpose**: Pure business logic, no infrastructure dependencies.

```rust
// Domain model
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, name: String) -> Self {
        Self {
            id: generate_id(),
            email,
            name,
            created_at: Utc::now(),
        }
    }

    // Domain logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.email.is_empty() {
            return Err(ValidationError::EmptyEmail);
        }
        if self.name.len() < 2 {
            return Err(ValidationError::NameTooShort);
        }
        Ok(())
    }

    pub fn update_email(&mut self, new_email: String) -> Result<(), ValidationError> {
        if !new_email.contains('@') {
            return Err(ValidationError::InvalidEmail);
        }
        self.email = new_email;
        Ok(())
    }
}
```

### When to Use Service Layer Architecture

**✅ Use for**:
- Web services and REST APIs
- Applications with multiple data sources
- Complex business logic requiring coordination
- Systems requiring extensive testing/mocking
- Long-lived services with evolving requirements

**❌ Avoid for**:
- Simple CLI tools
- One-off scripts
- Prototypes
- Single-responsibility binaries
- Performance-critical tight loops where abstraction overhead matters

---

## 3. Architectural Patterns

### 3.1 Hexagonal Architecture (Ports and Adapters)

**Purpose**: Decouple business logic from external dependencies. Business logic defines "ports" (trait interfaces), and external systems provide "adapters" (trait implementations).

```rust
// Port (trait) - defined by business logic
#[async_trait]
trait NotificationService: Send + Sync {
    async fn send(&self, recipient: &str, message: &str) -> Result<(), Error>;
}

// Adapter - Email implementation
struct EmailNotificationService {
    smtp_client: SmtpClient,
}

#[async_trait]
impl NotificationService for EmailNotificationService {
    async fn send(&self, recipient: &str, message: &str) -> Result<(), Error> {
        self.smtp_client.send_email(recipient, message).await
    }
}

// Adapter - SMS implementation
struct SmsNotificationService {
    twilio_client: TwilioClient,
}

#[async_trait]
impl NotificationService for SmsNotificationService {
    async fn send(&self, recipient: &str, message: &str) -> Result<(), Error> {
        self.twilio_client.send_sms(recipient, message).await
    }
}

// Business logic uses the port, not concrete adapters
struct OrderService {
    notification: Arc<dyn NotificationService>,
}

impl OrderService {
    pub async fn place_order(&self, order: Order) -> Result<(), Error> {
        // Business logic
        self.process_payment(&order).await?;

        // Use notification port
        self.notification.send(
            &order.customer_email,
            &format!("Order {} confirmed", order.id),
        ).await?;

        Ok(())
    }
}
```

**Benefits**:
- Business logic is completely independent of infrastructure
- Easy to swap implementations (email → SMS → push notification)
- Testable with mock adapters

**Drawbacks**:
- More upfront design
- Can be overkill for simple applications

### 3.2 Clean Architecture (Layered)

**Layers** (dependency direction: outer → inner):
1. **Entities/Domain**: Core business logic (innermost)
2. **Use Cases/Services**: Application-specific business rules
3. **Interface Adapters**: Controllers, presenters, gateways
4. **Frameworks/Drivers**: UI, database, web frameworks (outermost)

```rust
// Layer 1: Domain (no external dependencies)
mod domain {
    pub struct User {
        pub id: u64,
        pub email: String,
    }

    impl User {
        pub fn validate_email(&self) -> Result<(), String> {
            if !self.email.contains('@') {
                return Err("Invalid email".to_string());
            }
            Ok(())
        }
    }
}

// Layer 2: Use Cases (depends only on domain)
mod use_cases {
    use super::domain::User;
    use async_trait::async_trait;

    #[async_trait]
    pub trait UserRepository: Send + Sync {
        async fn save(&self, user: &User) -> Result<(), String>;
    }

    pub struct CreateUserUseCase<R: UserRepository> {
        repository: R,
    }

    impl<R: UserRepository> CreateUserUseCase<R> {
        pub async fn execute(&self, email: String) -> Result<User, String> {
            let user = User {
                id: 1,
                email,
            };
            user.validate_email()?;
            self.repository.save(&user).await?;
            Ok(user)
        }
    }
}

// Layer 3: Interface Adapters (implements use case ports)
mod adapters {
    use super::use_cases::UserRepository;
    use super::domain::User;
    use async_trait::async_trait;

    pub struct PostgresUserRepository {
        pool: sqlx::PgPool,
    }

    #[async_trait]
    impl UserRepository for PostgresUserRepository {
        async fn save(&self, user: &User) -> Result<(), String> {
            // Database-specific implementation
            Ok(())
        }
    }
}

// Layer 4: Frameworks (web framework, main)
mod web {
    use super::use_cases::CreateUserUseCase;
    use axum::{Json, extract::State};

    pub async fn create_user_handler(
        State(use_case): State<CreateUserUseCase<PostgresUserRepository>>,
        Json(request): Json<CreateUserRequest>,
    ) -> Result<Json<User>, String> {
        let user = use_case.execute(request.email).await?;
        Ok(Json(user))
    }
}
```

### 3.3 When to Use Each Pattern

| Pattern | Best For | Avoid When |
|---------|----------|------------|
| **Hexagonal** | Apps with many external integrations, microservices | Simple single-purpose apps |
| **Clean Architecture** | Large enterprise applications, evolving requirements | Prototypes, small utilities |
| **Layered (Simple)** | Most web services, medium complexity | Extremely simple or extremely complex apps |
| **Flat (No Architecture)** | CLI tools, scripts, proofs-of-concept | Anything with business logic |

**Recommendation for Rustbot**:
- Current size (~5k LOC): Simple layered architecture is sufficient
- If grows to 10k+ LOC: Consider hexagonal for agent/MCP integrations
- Keep it simple: Don't over-engineer for future that may not come

---

## 4. Testing Strategies

### 4.1 Unit Testing with Mocks

**Using `mockall` crate** (most popular Rust mocking library):

```rust
use mockall::*;
use async_trait::async_trait;

// Define trait
#[async_trait]
trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Error>;
}

// Create mock
#[automock]
#[async_trait]
trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u64) -> Result<Option<User>, Error>;
}

// Test
#[tokio::test]
async fn test_get_user() {
    let mut mock_repo = MockUserRepository::new();

    // Set expectation
    mock_repo
        .expect_find_by_id()
        .with(eq(1))
        .times(1)
        .returning(|_| Ok(Some(User::test_user())));

    let service = UserService::new(Arc::new(mock_repo));
    let user = service.get_user(1).await.unwrap();

    assert_eq!(user.id, 1);
}
```

**CRITICAL**: When using `mockall` with `async_trait`, the order of macros matters:

```rust
// ✅ CORRECT ORDER
#[automock]
#[async_trait]
trait MyTrait {
    async fn foo(&self) -> u32;
}

// ❌ WRONG ORDER (will not compile)
#[async_trait]
#[automock]
trait MyTrait {
    async fn foo(&self) -> u32;
}
```

### 4.2 Test Doubles Without Mocking Libraries

For simpler cases, manual test implementations:

```rust
// Production implementation
struct RealEmailService {
    smtp: SmtpClient,
}

#[async_trait]
impl EmailService for RealEmailService {
    async fn send(&self, to: &str, body: &str) -> Result<(), Error> {
        self.smtp.send(to, body).await
    }
}

// Test implementation
#[cfg(test)]
struct FakeEmailService {
    sent_emails: Arc<Mutex<Vec<(String, String)>>>,
}

#[cfg(test)]
#[async_trait]
impl EmailService for FakeEmailService {
    async fn send(&self, to: &str, body: &str) -> Result<(), Error> {
        self.sent_emails.lock().unwrap().push((to.to_string(), body.to_string()));
        Ok(())
    }
}

#[tokio::test]
async fn test_order_sends_email() {
    let fake_email = Arc::new(FakeEmailService {
        sent_emails: Arc::new(Mutex::new(Vec::new())),
    });

    let service = OrderService::new(fake_email.clone());
    service.place_order(Order::test_order()).await.unwrap();

    let emails = fake_email.sent_emails.lock().unwrap();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].1.contains("Order confirmed"));
}
```

### 4.3 Integration Testing

Test with real implementations but isolated environment:

```rust
#[tokio::test]
async fn test_user_creation_integration() {
    // Setup test database
    let pool = create_test_db_pool().await;

    // Real implementations, test environment
    let repo = Arc::new(PostgresUserRepository::new(pool.clone()));
    let service = UserService::new(repo);

    // Execute
    let user = service.create_user("test@example.com".to_string()).await.unwrap();

    // Verify in database
    let saved = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(saved.email, "test@example.com");

    // Cleanup
    cleanup_test_db(&pool).await;
}
```

### 4.4 Property-Based Testing

For testing invariants and edge cases:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_email_validation(email in "\\PC*@\\PC+\\.\\PC+") {
        let user = User::new(email.clone(), "Test".to_string());
        prop_assert!(user.validate().is_ok());
    }

    #[test]
    fn test_invalid_email_rejected(email in "\\PC+") {
        // Emails without '@' should fail
        if !email.contains('@') {
            let user = User::new(email, "Test".to_string());
            prop_assert!(user.validate().is_err());
        }
    }
}
```

---

## 5. Anti-Patterns to Avoid

### 5.1 Common Rust Anti-Patterns

#### ❌ Excessive `clone()`

**Problem**: Cloning everywhere due to unfamiliarity with borrowing.

```rust
// ❌ BAD: Unnecessary clones
fn process_user(user: User) {
    let user_clone1 = user.clone();
    validate(&user_clone1);

    let user_clone2 = user.clone();
    save(&user_clone2);
}

// ✅ GOOD: Use borrowing
fn process_user(user: &User) {
    validate(user);
    save(user);
}
```

#### ❌ `unwrap()` Everywhere

**Problem**: Panics on None/Err, not recoverable.

```rust
// ❌ BAD: Can panic
let user = repository.find(id).await.unwrap();

// ✅ GOOD: Proper error handling
let user = repository.find(id).await?;

// ✅ ALSO GOOD: Explicit handling
let user = match repository.find(id).await {
    Ok(Some(u)) => u,
    Ok(None) => return Err(Error::NotFound),
    Err(e) => return Err(Error::Database(e)),
};
```

#### ❌ String When &str Would Work

**Problem**: Unnecessary allocations.

```rust
// ❌ BAD: Forces allocation
fn greet(name: String) {
    println!("Hello, {}", name);
}
greet("Alice".to_string()); // Allocation!

// ✅ GOOD: Accept &str
fn greet(name: &str) {
    println!("Hello, {}", name);
}
greet("Alice"); // No allocation
```

#### ❌ Ignoring Clippy Warnings

**Problem**: Clippy catches common mistakes and non-idiomatic code.

```rust
// Always run:
cargo clippy --all-targets --all-features

// In CI, treat warnings as errors:
cargo clippy -- -D warnings
```

#### ❌ Arc<Mutex<T>> Everywhere

**Problem**: Over-synchronization, contention.

```rust
// ❌ BAD: Shared mutable state
struct App {
    counter: Arc<Mutex<u64>>,
}

// ✅ GOOD: Message passing
use tokio::sync::mpsc;

enum Command {
    Increment,
    GetValue(oneshot::Sender<u64>),
}

async fn counter_task(mut rx: mpsc::Receiver<Command>) {
    let mut count = 0;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Increment => count += 1,
            Command::GetValue(tx) => { let _ = tx.send(count); }
        }
    }
}
```

#### ❌ Generic Error Types

**Problem**: Callers can't handle specific errors.

```rust
// ❌ BAD: Generic error
async fn fetch_user(id: u64) -> Result<User, Box<dyn std::error::Error>> {
    // Caller can't match on specific errors
}

// ✅ GOOD: Specific error enum
#[derive(Debug, thiserror::Error)]
enum UserError {
    #[error("User not found: {0}")]
    NotFound(u64),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Validation failed: {0}")]
    Validation(String),
}

async fn fetch_user(id: u64) -> Result<User, UserError> {
    // Caller can match UserError variants
}
```

### 5.2 Architecture Anti-Patterns

#### ❌ Concrete Dependencies in Service Layer

**Problem**: Tightly coupled, hard to test.

```rust
// ❌ BAD: Depends on concrete type
struct UserService {
    repository: PostgresUserRepository, // Concrete!
}

// ✅ GOOD: Depends on trait
struct UserService {
    repository: Arc<dyn UserRepository>, // Abstract!
}
```

#### ❌ Global State for Dependencies

**Problem**: Hard to test, hidden dependencies.

```rust
// ❌ BAD: Global state
lazy_static! {
    static ref DB_POOL: PgPool = create_pool().await;
}

fn get_user(id: u64) -> User {
    DB_POOL.fetch_user(id) // Hidden dependency!
}

// ✅ GOOD: Explicit dependencies
fn get_user(pool: &PgPool, id: u64) -> User {
    pool.fetch_user(id)
}
```

#### ❌ Blocking Code in Async Context

**Problem**: Blocks the entire async runtime.

```rust
// ❌ BAD: Blocking call in async
async fn process_file(path: &str) {
    let contents = std::fs::read_to_string(path).unwrap(); // BLOCKS!
}

// ✅ GOOD: Use async I/O or spawn_blocking
async fn process_file(path: &str) {
    // Option 1: Async I/O
    let contents = tokio::fs::read_to_string(path).await.unwrap();

    // Option 2: Spawn blocking operation
    let path = path.to_string();
    let contents = tokio::task::spawn_blocking(move || {
        std::fs::read_to_string(path).unwrap()
    }).await.unwrap();
}
```

---

## 6. Tokio Async Best Practices

### 6.1 Avoid Blocking in Async

**Rule**: All async functions should be non-blocking.

```rust
// ❌ BAD: Blocking operation
async fn bad_example() {
    let result = std::fs::read_to_string("file.txt"); // BLOCKS RUNTIME!
}

// ✅ GOOD: Use tokio::fs
async fn good_example() {
    let result = tokio::fs::read_to_string("file.txt").await;
}

// ✅ GOOD: Wrap blocking with spawn_blocking
async fn also_good() {
    let result = tokio::task::spawn_blocking(|| {
        std::fs::read_to_string("file.txt")
    }).await.unwrap();
}
```

### 6.2 Mutex Usage Guidelines

**Rule**: Use `std::sync::Mutex` for short-held locks, `tokio::sync::Mutex` only if holding across `.await`.

```rust
// ✅ GOOD: std::sync::Mutex for short critical sections
struct Cache {
    data: std::sync::Mutex<HashMap<String, String>>,
}

impl Cache {
    async fn get(&self, key: &str) -> Option<String> {
        // Lock held briefly, no .await inside
        self.data.lock().unwrap().get(key).cloned()
    }
}

// ✅ GOOD: tokio::sync::Mutex when holding across .await
struct AsyncCache {
    data: tokio::sync::Mutex<HashMap<String, String>>,
}

impl AsyncCache {
    async fn get_or_fetch(&self, key: &str) -> String {
        let mut cache = self.data.lock().await;

        if let Some(val) = cache.get(key) {
            return val.clone();
        }

        // Holding lock across await - need tokio::sync::Mutex
        let value = fetch_from_network(key).await;
        cache.insert(key.to_string(), value.clone());
        value
    }
}
```

**Better**: Avoid holding locks across `.await` entirely.

```rust
// ✅ BEST: Don't hold lock across await
impl Cache {
    async fn get_or_fetch(&self, key: &str) -> String {
        // Check cache
        {
            let cache = self.data.lock().unwrap();
            if let Some(val) = cache.get(key) {
                return val.clone();
            }
        } // Lock released here

        // Fetch without holding lock
        let value = fetch_from_network(key).await;

        // Update cache
        self.data.lock().unwrap().insert(key.to_string(), value.clone());
        value
    }
}
```

### 6.3 Prefer Message Passing Over Shared State

**Rule**: Use channels instead of `Arc<Mutex<T>>` when possible.

```rust
use tokio::sync::mpsc;

// Actor pattern with message passing
enum CacheCommand {
    Get {
        key: String,
        respond_to: oneshot::Sender<Option<String>>,
    },
    Set {
        key: String,
        value: String,
    },
}

async fn cache_actor(mut rx: mpsc::Receiver<CacheCommand>) {
    let mut cache = HashMap::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            CacheCommand::Get { key, respond_to } => {
                let _ = respond_to.send(cache.get(&key).cloned());
            }
            CacheCommand::Set { key, value } => {
                cache.insert(key, value);
            }
        }
    }
}

// Usage
let (tx, rx) = mpsc::channel(32);
tokio::spawn(cache_actor(rx));

// Get value
let (send, recv) = oneshot::channel();
tx.send(CacheCommand::Get {
    key: "foo".to_string(),
    respond_to: send,
}).await.unwrap();
let value = recv.await.unwrap();
```

### 6.4 Graceful Shutdown

```rust
use tokio::signal;
use tokio::sync::broadcast;

async fn run_server() -> Result<(), Error> {
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

    // Spawn background tasks
    let task1 = tokio::spawn(async move {
        let mut rx = shutdown_tx.subscribe();
        loop {
            tokio::select! {
                _ = rx.recv() => {
                    println!("Task 1 shutting down");
                    break;
                }
                _ = do_work() => {}
            }
        }
    });

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    println!("Shutdown signal received");

    // Broadcast shutdown
    let _ = shutdown_tx.send(());

    // Wait for tasks to complete
    let _ = task1.await;

    println!("Graceful shutdown complete");
    Ok(())
}
```

---

## 7. Recommendations for Rustbot

### Current Architecture Assessment

**Strengths**:
- Clear module separation (`agent`, `api`, `mcp`, `ui`)
- Good use of async/await with tokio
- Error handling with proper Result types

**Areas for Improvement**:
1. **Tight Coupling**: Services directly instantiate dependencies
2. **Testing**: Limited testability due to concrete dependencies
3. **Configuration**: Hardcoded values scattered across modules

### Recommended Refactoring Strategy

#### Phase 1: Extract Traits for Core Services (Low Risk, High Value)

```rust
// src/services/mod.rs

#[async_trait]
pub trait AgentService: Send + Sync {
    async fn load_agents(&self) -> Result<Vec<Agent>, Error>;
    async fn get_agent(&self, id: &str) -> Result<Option<Agent>, Error>;
    async fn execute_agent(&self, id: &str, input: &str) -> Result<String, Error>;
}

#[async_trait]
pub trait McpService: Send + Sync {
    async fn list_servers(&self) -> Result<Vec<McpServer>, Error>;
    async fn call_tool(&self, server: &str, tool: &str, args: Value) -> Result<Value, Error>;
}

#[async_trait]
pub trait ConfigService: Send + Sync {
    fn get_api_key(&self) -> Result<String, Error>;
    fn get_model(&self) -> String;
    fn set_model(&mut self, model: String);
}
```

#### Phase 2: Implement Services with Dependency Injection

```rust
// src/services/agent_service.rs

pub struct DefaultAgentService {
    config: Arc<dyn ConfigService>,
    file_loader: Arc<dyn FileLoader>,
}

impl DefaultAgentService {
    pub fn new(
        config: Arc<dyn ConfigService>,
        file_loader: Arc<dyn FileLoader>,
    ) -> Self {
        Self { config, file_loader }
    }
}

#[async_trait]
impl AgentService for DefaultAgentService {
    async fn load_agents(&self) -> Result<Vec<Agent>, Error> {
        let agents_dir = self.config.get_agents_dir();
        self.file_loader.load_from_dir(&agents_dir).await
    }

    // ... other methods
}
```

#### Phase 3: Update App Initialization

```rust
// src/main.rs

async fn build_app() -> Result<App, Error> {
    // Initialize core services
    let config = Arc::new(FileConfigService::load(".env")?);
    let file_loader = Arc::new(DefaultFileLoader::new());

    // Initialize domain services
    let agent_service = Arc::new(DefaultAgentService::new(
        config.clone(),
        file_loader.clone(),
    ));

    let mcp_service = Arc::new(DefaultMcpService::new(
        config.clone(),
    ));

    // Build UI with injected dependencies
    Ok(App::new(agent_service, mcp_service, config))
}
```

#### Phase 4: Add Tests

```rust
// src/services/agent_service_test.rs

#[tokio::test]
async fn test_load_agents() {
    let mock_config = Arc::new(MockConfigService::new());
    let mock_loader = Arc::new(MockFileLoader::with_agents(vec![
        Agent::test_agent("agent1"),
        Agent::test_agent("agent2"),
    ]));

    let service = DefaultAgentService::new(mock_config, mock_loader);
    let agents = service.load_agents().await.unwrap();

    assert_eq!(agents.len(), 2);
}
```

### Gradual Migration Path

**DO NOT** attempt to refactor everything at once. Instead:

1. **Week 1-2**: Extract trait interfaces (no behavior change)
2. **Week 3-4**: Implement one service with DI (e.g., AgentService)
3. **Week 5**: Add tests for refactored service
4. **Week 6+**: Repeat for other services

### When NOT to Refactor

**Keep it simple for**:
- UI components (egui widgets) - they're already testable
- Pure functions (no state, no I/O)
- Small utilities (< 50 lines)

**Only refactor**:
- Services with external dependencies (file I/O, HTTP, MCP)
- Business logic that needs testing
- Code that changes frequently

### Success Metrics

- ✅ Can run tests without file system or network
- ✅ Can swap implementations (e.g., file config → environment config)
- ✅ Service logic decoupled from infrastructure
- ✅ Test coverage > 70% for service layer
- ❌ Over-abstraction: Don't create interfaces for everything

---

## Summary

### Key Takeaways

1. **Dependency Injection in Rust**:
   - Use trait-based DI with constructor injection
   - Prefer `Arc<dyn Trait>` for runtime flexibility
   - Manual DI is sufficient for most projects (no framework needed)

2. **Service Architecture**:
   - Repository pattern for data access
   - Service layer for business logic
   - Keep domain logic pure (no infrastructure dependencies)

3. **Testing**:
   - Use `mockall` for mocking traits
   - Manual test implementations for simpler cases
   - Integration tests with real implementations in isolated environments

4. **Async Best Practices**:
   - Never block in async functions
   - Use `std::sync::Mutex` for short locks, message passing for coordination
   - Implement graceful shutdown

5. **Anti-Patterns to Avoid**:
   - Excessive cloning, unwrapping, and Arc<Mutex<T>>
   - Concrete dependencies in services
   - Global state for dependencies

### For Rustbot Specifically

- **Start Simple**: Extract traits for `AgentService`, `McpService`, `ConfigService`
- **Gradual Migration**: One service at a time, with tests
- **Don't Over-Engineer**: UI and utilities can stay simple
- **Focus on Value**: Refactor what changes frequently or needs testing

### Additional Resources

- [Rust Design Patterns Book](https://rust-unofficial.github.io/patterns/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [mockall Documentation](https://docs.rs/mockall)
- [async-trait Crate](https://docs.rs/async-trait)
- [Hexagonal Architecture in Rust](https://www.howtocodeit.com/articles/master-hexagonal-architecture-rust)

---

**Document Version**: 1.0
**Last Updated**: January 2025
**Research Date**: January 17, 2025
