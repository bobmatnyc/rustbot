//! **ADVANCED TESTING - Error Conditions with Mockall**
//!
//! This example demonstrates how to use mockall to test error conditions
//! that are difficult or impossible to simulate with real implementations.
//!
//! **Run this example:**
//! ```bash
//! cargo test --example mockall_testing
//! ```

use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Token stats type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenStats {
    pub daily_input: u32,
    pub daily_output: u32,
    pub total_input: u32,
    pub total_output: u32,
}

impl Default for TokenStats {
    fn default() -> Self {
        Self {
            daily_input: 0,
            daily_output: 0,
            total_input: 0,
            total_output: 0,
        }
    }
}

// Storage service trait
#[async_trait]
pub trait StorageService: Send + Sync {
    async fn load_token_stats(&self) -> Result<TokenStats, String>;
    async fn save_token_stats(&self, stats: &TokenStats) -> Result<(), String>;
}

// Application using storage
pub struct RustbotApp {
    storage: Arc<dyn StorageService>,
    pub token_stats: TokenStats,
}

impl RustbotApp {
    pub async fn new(storage: Arc<dyn StorageService>) -> Result<Self, String> {
        let token_stats = storage.load_token_stats().await?;
        Ok(Self {
            storage,
            token_stats,
        })
    }

    pub async fn save_token_stats(&self) -> Result<(), String> {
        self.storage.save_token_stats(&self.token_stats).await
    }

    pub fn update_token_usage(&mut self, input_tokens: u32, output_tokens: u32) {
        self.token_stats.daily_input += input_tokens;
        self.token_stats.daily_output += output_tokens;
        self.token_stats.total_input += input_tokens;
        self.token_stats.total_output += output_tokens;
    }

    /// Complete API call workflow with error handling
    pub async fn process_api_call(
        &mut self,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<(), String> {
        // Update stats
        self.update_token_usage(input_tokens, output_tokens);

        // Try to save (may fail)
        self.save_token_stats().await?;

        Ok(())
    }
}

// ============================================================================
// MOCKALL - Create mock implementation for testing
// ============================================================================

mock! {
    pub StorageService {}

    #[async_trait]
    impl StorageService for StorageService {
        async fn load_token_stats(&self) -> Result<TokenStats, String>;
        async fn save_token_stats(&self, stats: &TokenStats) -> Result<(), String>;
    }
}

// ============================================================================
// TESTS - Error Condition Testing
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// **Test 1**: Handle storage load error
    #[tokio::test]
    async fn test_load_error_on_startup() {
        let mut mock = MockStorageService::new();

        // Simulate storage error (corrupt file, permission denied, etc.)
        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Err("Failed to read file: Permission denied".to_string()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);

        // App creation should fail gracefully
        let result = RustbotApp::new(storage).await;

        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert_eq!(error_msg, "Failed to read file: Permission denied");
    }

    /// **Test 2**: Handle storage save error (disk full)
    #[tokio::test]
    async fn test_save_error_disk_full() {
        let mut mock = MockStorageService::new();

        // Load succeeds
        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Save fails (disk full)
        mock.expect_save_token_stats()
            .times(1)
            .returning(|_| Err("Disk full".to_string()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let mut app = RustbotApp::new(storage).await.unwrap();

        app.update_token_usage(100, 50);

        // Save should fail
        let result = app.save_token_stats().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Disk full");

        // Stats still updated in memory (not lost)
        assert_eq!(app.token_stats.daily_input, 100);
    }

    /// **Test 3**: Intermittent failures (flaky storage)
    #[tokio::test]
    async fn test_intermittent_save_failures() {
        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // First save succeeds
        mock.expect_save_token_stats()
            .times(1)
            .returning(|_| Ok(()));

        // Second save fails (network timeout)
        mock.expect_save_token_stats()
            .times(1)
            .returning(|_| Err("Network timeout".to_string()));

        // Third save succeeds again
        mock.expect_save_token_stats()
            .times(1)
            .returning(|_| Ok(()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let mut app = RustbotApp::new(storage).await.unwrap();

        // First call: success
        app.update_token_usage(100, 50);
        assert!(app.save_token_stats().await.is_ok());

        // Second call: failure
        app.update_token_usage(100, 50);
        assert!(app.save_token_stats().await.is_err());

        // Third call: success again
        app.update_token_usage(100, 50);
        assert!(app.save_token_stats().await.is_ok());
    }

    /// **Test 4**: Verify save is called with correct data
    #[tokio::test]
    async fn test_save_called_with_correct_data() {
        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Verify save is called with expected stats
        mock.expect_save_token_stats()
            .times(1)
            .withf(|stats: &TokenStats| stats.daily_input == 100 && stats.daily_output == 50)
            .returning(|_| Ok(()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let mut app = RustbotApp::new(storage).await.unwrap();

        app.update_token_usage(100, 50);
        app.save_token_stats().await.unwrap();

        // Test passes if mock expectations met
    }

    /// **Test 5**: Test save is NOT called on read-only operations
    #[tokio::test]
    async fn test_no_save_on_readonly_operations() {
        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Expect save to NEVER be called
        mock.expect_save_token_stats().times(0); // ← Important: verify no saves

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let app = RustbotApp::new(storage).await.unwrap();

        // Just read data, don't save
        let _input = app.token_stats.daily_input;

        // Test passes if save was never called
    }

    /// **Test 6**: Test retry logic demonstration
    #[tokio::test]
    async fn test_save_retry_logic() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let call_count = StdArc::new(AtomicUsize::new(0));
        let call_count_clone = StdArc::clone(&call_count);

        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Fail twice, succeed third time
        mock.expect_save_token_stats().times(3).returning(move |_| {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err("Temporary failure".to_string())
            } else {
                Ok(())
            }
        });

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let mut app = RustbotApp::new(storage).await.unwrap();

        app.update_token_usage(100, 50);

        // Manually retry (in real app, this would be automatic)
        let mut attempts = 0;
        let mut result = Err("Not attempted".to_string());

        while attempts < 3 && result.is_err() {
            result = app.save_token_stats().await;
            attempts += 1;
        }

        assert!(result.is_ok());
        assert_eq!(attempts, 3);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    /// **Test 7**: Test concurrent access (race condition testing)
    #[tokio::test]
    async fn test_concurrent_saves() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Expect multiple concurrent saves
        mock.expect_save_token_stats()
            .times(10)
            .returning(|_| Ok(()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let app = Arc::new(Mutex::new(RustbotApp::new(storage).await.unwrap()));

        // Spawn 10 concurrent save operations
        let mut handles = vec![];

        for i in 0..10 {
            let app_clone = Arc::clone(&app);
            let handle = tokio::spawn(async move {
                let mut app_guard = app_clone.lock().await;
                app_guard.update_token_usage(i * 10, i * 5);
                app_guard.save_token_stats().await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
    }

    /// **Test 8**: Test data corruption detection
    #[tokio::test]
    async fn test_corrupt_data_handling() {
        let mut mock = MockStorageService::new();

        // Simulate corrupt data (JSON parse error)
        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Err("JSON parse error: unexpected token".to_string()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);

        // App should fail to load
        let result = RustbotApp::new(storage).await;

        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert!(error_msg.contains("parse error"));
    }

    /// **Test 9**: Test workflow with save failure
    #[tokio::test]
    async fn test_workflow_handles_save_failure() {
        let mut mock = MockStorageService::new();

        mock.expect_load_token_stats()
            .times(1)
            .returning(|| Ok(TokenStats::default()));

        // Workflow includes save, which fails
        mock.expect_save_token_stats()
            .times(1)
            .returning(|_| Err("Write error".to_string()));

        let storage: Arc<dyn StorageService> = Arc::new(mock);
        let mut app = RustbotApp::new(storage).await.unwrap();

        // Workflow should return error
        let result = app.process_api_call(100, 50).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Write error");

        // But stats were still updated in memory
        assert_eq!(app.token_stats.daily_input, 100);
    }
}

fn main() {
    println!("=== MOCKALL TESTING - Error Condition Testing ===\n");
    println!("This example demonstrates advanced testing with mockall.");
    println!("\nRun tests with: cargo test --example mockall_testing\n");
    println!("Tests demonstrate:");
    println!("  ✅ Load errors (permission denied, corrupt data)");
    println!("  ✅ Save errors (disk full, network timeout)");
    println!("  ✅ Intermittent failures (retry logic)");
    println!("  ✅ Data validation (correct data saved)");
    println!("  ✅ Concurrent access (race conditions)");
    println!("  ✅ Read-only operations (no unnecessary saves)");
}
