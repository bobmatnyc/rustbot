//! **AFTER REFACTORING - Dependency Injection Pattern**
//!
//! This example demonstrates how to refactor the code to use dependency injection
//! with trait-based abstractions, making it testable, mockable, and maintainable.
//!
//! **Benefits of this approach:**
//! - ✅ Unit test business logic without filesystem
//! - ✅ Easy to mock for testing
//! - ✅ Clear separation of concerns (business logic vs I/O)
//! - ✅ Dependency injection through constructor
//! - ✅ Easy to swap implementations (JSON → Database → Cloud)
//! - ✅ Easy to test error conditions
//! - ✅ Thread-safe async operations
//!
//! **Run this example:**
//! ```bash
//! cargo run --example after_refactoring
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenStats {
    pub daily_input: u32,
    pub daily_output: u32,
    pub total_input: u32,
    pub total_output: u32,
    #[serde(default)]
    pub last_reset_date: String,
}

impl Default for TokenStats {
    fn default() -> Self {
        Self {
            daily_input: 0,
            daily_output: 0,
            total_input: 0,
            total_output: 0,
            last_reset_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        }
    }
}

// ============================================================================
// TRAIT ABSTRACTION - The key to testability
// ============================================================================

/// **Storage abstraction** - Defines WHAT operations we need, not HOW they work
///
/// This trait can be implemented by:
/// - FileStorageService (production - saves to disk)
/// - InMemoryStorageService (testing - in-memory HashMap)
/// - DatabaseStorageService (future - PostgreSQL)
/// - MockStorageService (testing - mockall)
#[async_trait]
pub trait StorageService: Send + Sync {
    /// Load token stats from storage
    /// Returns default if not found (first run)
    async fn load_token_stats(&self) -> Result<TokenStats, String>;

    /// Save token stats to storage
    async fn save_token_stats(&self, stats: &TokenStats) -> Result<(), String>;
}

// ============================================================================
// PRODUCTION IMPLEMENTATION - File-based storage
// ============================================================================

/// **Production implementation** - Saves to JSON files on disk
pub struct FileStorageService {
    base_path: PathBuf,
}

impl FileStorageService {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn token_stats_path(&self) -> PathBuf {
        self.base_path.join("token_stats.json")
    }
}

#[async_trait]
impl StorageService for FileStorageService {
    async fn load_token_stats(&self) -> Result<TokenStats, String> {
        let path = self.token_stats_path();

        if !path.exists() {
            println!("No token_stats.json found, using defaults");
            return Ok(TokenStats::default());
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read token stats: {}", e))?;

        let stats = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse token stats: {}", e))?;

        println!("✓ Loaded token stats from {:?}", path);
        Ok(stats)
    }

    async fn save_token_stats(&self, stats: &TokenStats) -> Result<(), String> {
        let path = self.token_stats_path();

        let content = serde_json::to_string_pretty(stats)
            .map_err(|e| format!("Failed to serialize token stats: {}", e))?;

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| format!("Failed to write token stats: {}", e))?;

        println!("✓ Saved token stats to {:?}", path);
        Ok(())
    }
}

// ============================================================================
// TEST IMPLEMENTATION - In-memory storage (no filesystem I/O)
// ============================================================================

/// **Test implementation** - In-memory HashMap, perfect for testing
/// No filesystem access, instant, isolated tests
pub struct InMemoryStorageService {
    storage: Arc<Mutex<HashMap<String, TokenStats>>>,
}

impl InMemoryStorageService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Test helper: Pre-populate storage with test data
    pub async fn seed(&self, stats: TokenStats) {
        let mut storage = self.storage.lock().await;
        storage.insert("token_stats".to_string(), stats);
    }
}

#[async_trait]
impl StorageService for InMemoryStorageService {
    async fn load_token_stats(&self) -> Result<TokenStats, String> {
        let storage = self.storage.lock().await;
        Ok(storage.get("token_stats").cloned().unwrap_or_default())
    }

    async fn save_token_stats(&self, stats: &TokenStats) -> Result<(), String> {
        let mut storage = self.storage.lock().await;
        storage.insert("token_stats".to_string(), stats.clone());
        Ok(())
    }
}

// ============================================================================
// APPLICATION WITH DEPENDENCY INJECTION
// ============================================================================

/// **Refactored application** - Business logic separated from I/O
///
/// Key differences from before:
/// 1. Takes `StorageService` as dependency (constructor injection)
/// 2. Uses trait object `Arc<dyn StorageService>` for runtime polymorphism
/// 3. Business logic methods don't know about storage implementation
pub struct RustbotApp {
    /// **DEPENDENCY INJECTION**: Storage abstraction injected via constructor
    storage: Arc<dyn StorageService>,
    pub token_stats: TokenStats,
}

impl RustbotApp {
    /// **Constructor injection** - Storage service provided by caller
    ///
    /// This enables:
    /// - Production: FileStorageService
    /// - Testing: InMemoryStorageService or MockStorageService
    /// - Future: DatabaseStorageService
    pub async fn new(storage: Arc<dyn StorageService>) -> Result<Self, String> {
        // Load initial stats using injected storage
        let token_stats = storage.load_token_stats().await?;

        Ok(Self {
            storage,
            token_stats,
        })
    }

    /// Save current stats using injected storage
    /// **Benefit**: Tests can verify this is called without touching filesystem
    pub async fn save_token_stats(&self) -> Result<(), String> {
        self.storage.save_token_stats(&self.token_stats).await
    }

    /// **Pure business logic** - No I/O dependencies!
    /// This can now be tested in complete isolation
    pub fn update_token_usage(&mut self, input_tokens: u32, output_tokens: u32) {
        self.token_stats.daily_input += input_tokens;
        self.token_stats.daily_output += output_tokens;
        self.token_stats.total_input += input_tokens;
        self.token_stats.total_output += output_tokens;
    }

    /// **Pure calculation** - Easy to test
    pub fn calculate_cost(&self) -> f64 {
        let input_cost_per_1k = 0.003;
        let output_cost_per_1k = 0.015;

        let input_cost = (self.token_stats.total_input as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (self.token_stats.total_output as f64 / 1000.0) * output_cost_per_1k;

        input_cost + output_cost
    }

    /// **Complete workflow** - Load, update, save
    /// Business logic orchestration without knowing storage details
    pub async fn process_api_call(
        &mut self,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<(), String> {
        // Update stats (pure logic)
        self.update_token_usage(input_tokens, output_tokens);

        // Persist changes (delegated to storage)
        self.save_token_stats().await?;

        Ok(())
    }
}

// ============================================================================
// MAIN - PRODUCTION USAGE
// ============================================================================

#[tokio::main]
async fn main() {
    println!("=== AFTER REFACTORING - Dependency Injection Pattern ===\n");

    // **Production**: Use file storage
    let storage: Arc<dyn StorageService> = Arc::new(FileStorageService::new(PathBuf::from(".")));

    // Create app with injected storage
    let mut app = RustbotApp::new(storage)
        .await
        .expect("Failed to create app");

    println!("\nInitial stats:");
    println!("  Daily input:  {}", app.token_stats.daily_input);
    println!("  Daily output: {}", app.token_stats.daily_output);
    println!("  Total input:  {}", app.token_stats.total_input);
    println!("  Total output: {}", app.token_stats.total_output);

    // Simulate API call
    println!("\nSimulating API call with 1000 input tokens, 500 output tokens...");
    app.process_api_call(1000, 500)
        .await
        .expect("Failed to process API call");

    println!("\nUpdated stats:");
    println!("  Daily input:  {}", app.token_stats.daily_input);
    println!("  Daily output: {}", app.token_stats.daily_output);
    println!("  Total input:  {}", app.token_stats.total_input);
    println!("  Total output: {}", app.token_stats.total_output);
    println!("  Total cost:   ${:.4}", app.calculate_cost());

    println!("\n=== Benefits of this approach ===");
    println!("✅ Business logic tested without filesystem");
    println!("✅ Easy to swap storage (File → DB → Cloud)");
    println!("✅ Error conditions easily tested");
    println!("✅ Clear dependencies (explicit in constructor)");
    println!("✅ Thread-safe async operations");
    println!("✅ Run tests in parallel (no shared filesystem state)");
    println!("\n💡 See tests below for examples!");
}

// ============================================================================
// UNIT TESTS - NO FILESYSTEM I/O REQUIRED
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// **Test 1**: Business logic in complete isolation
    /// No filesystem, no mocking, just pure logic testing
    #[tokio::test]
    async fn test_update_token_usage_isolated() {
        // Use in-memory storage (no filesystem!)
        let storage: Arc<dyn StorageService> = Arc::new(InMemoryStorageService::new());

        let mut app = RustbotApp::new(storage).await.unwrap();

        // Test the business logic
        app.update_token_usage(100, 50);

        assert_eq!(app.token_stats.daily_input, 100);
        assert_eq!(app.token_stats.daily_output, 50);
        assert_eq!(app.token_stats.total_input, 100);
        assert_eq!(app.token_stats.total_output, 50);

        // ✅ No filesystem pollution!
    }

    /// **Test 2**: Cost calculation in isolation
    #[tokio::test]
    async fn test_calculate_cost() {
        let storage: Arc<dyn StorageService> = Arc::new(InMemoryStorageService::new());
        let mut app = RustbotApp::new(storage).await.unwrap();

        app.token_stats.total_input = 10000;
        app.token_stats.total_output = 5000;

        let cost = app.calculate_cost();

        // Expected: (10000/1000 * 0.003) + (5000/1000 * 0.015) = 0.105
        assert!((cost - 0.105).abs() < 0.001);

        // ✅ Pure calculation test, no I/O
    }

    /// **Test 3**: Verify storage operations (load/save)
    #[tokio::test]
    async fn test_save_and_load() {
        let storage = Arc::new(InMemoryStorageService::new());

        // Create app and update stats
        let mut app = RustbotApp::new(Arc::clone(&storage) as Arc<dyn StorageService>)
            .await
            .unwrap();

        app.update_token_usage(200, 100);
        app.save_token_stats().await.unwrap();

        // Create new app instance - should load saved stats
        let new_app = RustbotApp::new(Arc::clone(&storage) as Arc<dyn StorageService>)
            .await
            .unwrap();

        assert_eq!(new_app.token_stats.daily_input, 200);
        assert_eq!(new_app.token_stats.daily_output, 100);

        // ✅ Tests persistence without filesystem
    }

    /// **Test 4**: Complete workflow test
    #[tokio::test]
    async fn test_process_api_call_workflow() {
        let storage: Arc<dyn StorageService> = Arc::new(InMemoryStorageService::new());
        let mut app = RustbotApp::new(storage).await.unwrap();

        // Process multiple API calls
        app.process_api_call(100, 50).await.unwrap();
        app.process_api_call(200, 100).await.unwrap();

        assert_eq!(app.token_stats.daily_input, 300);
        assert_eq!(app.token_stats.daily_output, 150);

        // ✅ Tests business logic + persistence together
    }

    /// **Test 5**: Test with pre-seeded data
    #[tokio::test]
    async fn test_with_existing_data() {
        let storage = Arc::new(InMemoryStorageService::new());

        // Seed storage with existing stats
        let existing_stats = TokenStats {
            daily_input: 5000,
            daily_output: 2500,
            total_input: 50000,
            total_output: 25000,
            last_reset_date: "2025-11-16".to_string(),
        };
        storage.seed(existing_stats).await;

        // Create app - should load seeded data
        let app = RustbotApp::new(Arc::clone(&storage) as Arc<dyn StorageService>)
            .await
            .unwrap();

        assert_eq!(app.token_stats.daily_input, 5000);
        assert_eq!(app.token_stats.total_input, 50000);

        // ✅ Easy to test with existing data scenarios
    }

    /// **Test 6**: Multiple apps can share same storage
    #[tokio::test]
    async fn test_shared_storage() {
        let storage: Arc<dyn StorageService> = Arc::new(InMemoryStorageService::new());

        // Create first app, update stats
        let mut app1 = RustbotApp::new(Arc::clone(&storage)).await.unwrap();
        app1.update_token_usage(100, 50);
        app1.save_token_stats().await.unwrap();

        // Create second app - should see updates from first
        let app2 = RustbotApp::new(Arc::clone(&storage)).await.unwrap();

        assert_eq!(app2.token_stats.daily_input, 100);
        assert_eq!(app2.token_stats.daily_output, 50);

        // ✅ Tests thread-safe shared state
    }

    // ✅ POSSIBLE: Test error conditions with mock storage
    // (See docs/PROTOTYPE_REFACTORING.md for mockall examples)
}
