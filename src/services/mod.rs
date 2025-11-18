// Service layer for dependency injection and testability
//
// Design Decision: Service-oriented architecture with trait-based DI
//
// Rationale: This module provides trait abstractions for all infrastructure
// concerns (filesystem, storage, configuration) to enable:
// 1. Unit testing with mock implementations (no filesystem/network I/O)
// 2. Dependency injection via constructor injection
// 3. Loose coupling between business logic and infrastructure
// 4. Easy substitution of implementations (file → database, local → cloud)
//
// Trade-offs:
// - Abstraction overhead: Trait indirection vs. direct implementation calls
// - Testing benefits: Comprehensive unit tests vs. integration-only tests
// - Flexibility: Easy to swap implementations vs. simpler direct calls
//
// Architecture Pattern: Ports and Adapters (Hexagonal Architecture)
// - Traits define "ports" (interfaces to outside world)
// - Services implement business logic using ports
// - "Adapters" (RealFileSystem, FileStorageService) connect to infrastructure
//
// Usage Example:
//     // Production code
//     let fs = Arc::new(RealFileSystem);
//     let storage = Arc::new(FileStorageService::new(fs, PathBuf::from("data")));
//     let tokens = storage.load_token_stats().await?;
//
//     // Test code
//     let mock_fs = Arc::new(MockFileSystem::new());
//     let storage = Arc::new(FileStorageService::new(mock_fs, PathBuf::from("test")));
//     let tokens = storage.load_token_stats().await?; // No real filesystem I/O
//
// Extension Points: Add new service traits as infrastructure needs grow
// (database, cache, message queue, etc.)

pub mod agents;
pub mod config;
pub mod filesystem;
#[cfg(test)]
pub mod integration_tests;
#[cfg(test)]
pub mod mocks;
pub mod storage;
pub mod traits;

// Re-export commonly used types
pub use agents::DefaultAgentService;
pub use config::FileConfigService;
pub use filesystem::RealFileSystem;
pub use storage::FileStorageService;
pub use traits::{AgentService, ConfigService, FileSystem, StorageService};
