// File-based storage service implementation
//
// Design Decision: JSON files in application data directory
//
// Rationale: For a desktop application, JSON files provide:
// 1. Human-readable storage (easy debugging and manual editing)
// 2. No database dependency (simpler deployment)
// 3. Cross-platform compatibility (works on all OS)
// 4. Git-friendly (can version control user data if needed)
//
// Trade-offs:
// - Simplicity: JSON files vs. SQLite database
// - Performance: O(1) file read vs. O(log n) database query (acceptable for small data)
// - Scalability: Single-user desktop app vs. multi-user server (JSON sufficient)
//
// Extension Points: Can switch to SQLite or cloud storage by implementing
// StorageService trait with a different adapter (no business logic changes).

use super::traits::{FileSystem, StorageService, SystemPrompts, TokenStats};
use crate::error::{Result, RustbotError};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// File-based storage service using JSON serialization
///
/// Stores application data as JSON files in a configured directory.
/// Each data type has its own file (token_stats.json, system_prompts.json, etc.).
///
/// Thread Safety: Uses Arc<dyn FileSystem> for shared filesystem access.
/// Multiple instances can safely read, but writes should be coordinated
/// at the application level (single writer).
///
/// Usage:
///     let fs = Arc::new(RealFileSystem);
///     let storage = FileStorageService::new(fs, PathBuf::from("data"));
///     let stats = storage.load_token_stats().await?;
pub struct FileStorageService {
    /// Filesystem abstraction for testing
    fs: Arc<dyn FileSystem>,

    /// Base directory for storing data files
    base_path: PathBuf,
}

impl FileStorageService {
    /// Create a new file storage service
    ///
    /// # Arguments
    /// * `fs` - Filesystem implementation (RealFileSystem for production)
    /// * `base_path` - Directory to store data files
    ///
    /// The base directory is created if it doesn't exist on first write.
    pub fn new(fs: Arc<dyn FileSystem>, base_path: PathBuf) -> Self {
        Self { fs, base_path }
    }

    /// Get path to token stats file
    fn token_stats_path(&self) -> PathBuf {
        self.base_path.join("token_stats.json")
    }

    /// Get path to system prompts file
    fn system_prompts_path(&self) -> PathBuf {
        self.base_path.join("system_prompts.json")
    }

    /// Ensure base directory exists
    async fn ensure_base_dir(&self) -> Result<()> {
        if !self.fs.exists(&self.base_path).await {
            self.fs.create_dir_all(&self.base_path).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl StorageService for FileStorageService {
    async fn load_token_stats(&self) -> Result<TokenStats> {
        let path = self.token_stats_path();

        if !self.fs.exists(&path).await {
            // Return default stats if file doesn't exist (first run)
            return Ok(TokenStats::default());
        }

        let content = self.fs.read_to_string(&path).await?;

        serde_json::from_str(&content).map_err(|e| {
            RustbotError::StorageError(format!("Failed to deserialize token stats: {}", e))
        })
    }

    async fn save_token_stats(&self, stats: &TokenStats) -> Result<()> {
        self.ensure_base_dir().await?;

        let path = self.token_stats_path();
        let content = serde_json::to_string_pretty(stats).map_err(|e| {
            RustbotError::StorageError(format!("Failed to serialize token stats: {}", e))
        })?;

        self.fs.write(&path, &content).await?;
        Ok(())
    }

    async fn load_system_prompts(&self) -> Result<SystemPrompts> {
        let path = self.system_prompts_path();

        if !self.fs.exists(&path).await {
            // Return default prompts if file doesn't exist
            return Ok(SystemPrompts::default());
        }

        let content = self.fs.read_to_string(&path).await?;

        serde_json::from_str(&content).map_err(|e| {
            RustbotError::StorageError(format!("Failed to deserialize system prompts: {}", e))
        })
    }

    async fn save_system_prompts(&self, prompts: &SystemPrompts) -> Result<()> {
        self.ensure_base_dir().await?;

        let path = self.system_prompts_path();
        let content = serde_json::to_string_pretty(prompts).map_err(|e| {
            RustbotError::StorageError(format!("Failed to serialize system prompts: {}", e))
        })?;

        self.fs.write(&path, &content).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mocks::test_helpers::*;
    use crate::services::traits::MockFileSystem;
    use crate::services::RealFileSystem;
    use mockall::predicate::*;
    use tempfile::TempDir;

    // ===== INTEGRATION TESTS (using real filesystem) =====

    #[tokio::test]
    async fn test_load_token_stats_default() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs, temp_dir.path().to_path_buf());

        // Should return default stats when file doesn't exist
        let stats = storage.load_token_stats().await.unwrap();
        assert_eq!(stats.total_input_tokens, 0);
        assert_eq!(stats.total_output_tokens, 0);
    }

    #[tokio::test]
    async fn test_save_and_load_token_stats() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs, temp_dir.path().to_path_buf());

        // Create and save stats
        let mut stats = TokenStats::default();
        stats.total_input_tokens = 1000;
        stats.total_output_tokens = 500;
        stats.total_cost = 0.05;

        storage.save_token_stats(&stats).await.unwrap();

        // Load and verify
        let loaded_stats = storage.load_token_stats().await.unwrap();
        assert_eq!(loaded_stats.total_input_tokens, 1000);
        assert_eq!(loaded_stats.total_output_tokens, 500);
        assert_eq!(loaded_stats.total_cost, 0.05);
    }

    #[tokio::test]
    async fn test_load_system_prompts_default() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs, temp_dir.path().to_path_buf());

        // Should return default prompts when file doesn't exist
        let prompts = storage.load_system_prompts().await.unwrap();
        assert_eq!(prompts.base_prompt, "");
        assert_eq!(prompts.context, None);
    }

    #[tokio::test]
    async fn test_save_and_load_system_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs, temp_dir.path().to_path_buf());

        // Create and save prompts
        let prompts = SystemPrompts {
            base_prompt: "You are a helpful assistant.".to_string(),
            context: Some("Additional context here.".to_string()),
        };

        storage.save_system_prompts(&prompts).await.unwrap();

        // Load and verify
        let loaded_prompts = storage.load_system_prompts().await.unwrap();
        assert_eq!(loaded_prompts.base_prompt, "You are a helpful assistant.");
        assert_eq!(
            loaded_prompts.context,
            Some("Additional context here.".to_string())
        );
    }

    #[tokio::test]
    async fn test_ensure_base_dir_created() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("data");

        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs.clone(), nested_path.clone());

        // Directory doesn't exist yet
        assert!(!fs.exists(&nested_path).await);

        // Save should create directory
        let stats = TokenStats::default();
        storage.save_token_stats(&stats).await.unwrap();

        // Directory should now exist
        assert!(fs.exists(&nested_path).await);
    }

    // ===== UNIT TESTS (using mocks) =====

    #[tokio::test]
    async fn test_mock_load_token_stats_success() {
        let mut mock_fs = MockFileSystem::new();

        // Setup: file exists with valid JSON
        let test_path = PathBuf::from("data/token_stats.json");
        mock_fs
            .expect_exists()
            .with(eq(test_path.clone()))
            .times(1)
            .returning(|_| true);

        mock_fs
            .expect_read_to_string()
            .with(eq(test_path))
            .times(1)
            .returning(|_| {
                Ok(r#"{
                    "total_input_tokens": 1000,
                    "total_output_tokens": 500,
                    "total_cost": 0.05,
                    "last_updated": "2024-01-01T00:00:00Z"
                }"#
                .to_string())
            });

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let stats = storage.load_token_stats().await.unwrap();
        assert_eq!(stats.total_input_tokens, 1000);
        assert_eq!(stats.total_output_tokens, 500);
        assert_eq!(stats.total_cost, 0.05);
    }

    #[tokio::test]
    async fn test_mock_load_token_stats_file_not_found() {
        let mut mock_fs = MockFileSystem::new();

        // Setup: file doesn't exist
        let test_path = PathBuf::from("data/token_stats.json");
        mock_fs
            .expect_exists()
            .with(eq(test_path))
            .times(1)
            .returning(|_| false);

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        // Should return default stats
        let stats = storage.load_token_stats().await.unwrap();
        assert_eq!(stats.total_input_tokens, 0);
        assert_eq!(stats.total_output_tokens, 0);
        assert_eq!(stats.total_cost, 0.0);
    }

    #[tokio::test]
    async fn test_mock_load_token_stats_invalid_json() {
        let mut mock_fs = MockFileSystem::new();

        // Setup: file exists but contains invalid JSON
        let test_path = PathBuf::from("data/token_stats.json");
        mock_fs
            .expect_exists()
            .with(eq(test_path.clone()))
            .times(1)
            .returning(|_| true);

        mock_fs
            .expect_read_to_string()
            .with(eq(test_path))
            .times(1)
            .returning(|_| Ok("invalid json {{{".to_string()));

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let result = storage.load_token_stats().await;
        assert!(result.is_err());

        match result {
            Err(RustbotError::StorageError(msg)) => {
                assert!(msg.contains("Failed to deserialize token stats"));
            }
            _ => panic!("Expected StorageError"),
        }
    }

    #[tokio::test]
    async fn test_mock_save_token_stats_success() {
        let mut mock_fs = MockFileSystem::new();

        let base_path = PathBuf::from("data");
        let test_path = base_path.join("token_stats.json");

        // Setup: directory doesn't exist, needs creation
        mock_fs
            .expect_exists()
            .with(eq(base_path.clone()))
            .times(1)
            .returning(|_| false);

        mock_fs
            .expect_create_dir_all()
            .with(eq(base_path))
            .times(1)
            .returning(|_| Ok(()));

        // Expect write with JSON containing token stats
        mock_fs
            .expect_write()
            .with(
                eq(test_path),
                function(|s: &str| {
                    s.contains("total_input_tokens")
                        && s.contains("500")
                        && s.contains("total_output_tokens")
                        && s.contains("250")
                }),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let stats = create_test_token_stats(500, 250, 0.025);
        assert!(storage.save_token_stats(&stats).await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_save_token_stats_write_error() {
        let mut mock_fs = MockFileSystem::new();

        let base_path = PathBuf::from("data");

        // Setup: directory exists
        mock_fs
            .expect_exists()
            .with(eq(base_path.clone()))
            .times(1)
            .returning(|_| true);

        // Write fails (e.g., disk full, permission denied)
        mock_fs.expect_write().times(1).returning(|_, _| {
            Err(RustbotError::IoError(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Permission denied",
            )))
        });

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let stats = TokenStats::default();
        let result = storage.save_token_stats(&stats).await;

        assert!(result.is_err());
        match result {
            Err(RustbotError::IoError(_)) => {} // Expected
            _ => panic!("Expected IoError"),
        }
    }

    #[tokio::test]
    async fn test_mock_load_system_prompts_success() {
        let mut mock_fs = MockFileSystem::new();

        let test_path = PathBuf::from("data/system_prompts.json");
        mock_fs
            .expect_exists()
            .with(eq(test_path.clone()))
            .times(1)
            .returning(|_| true);

        mock_fs
            .expect_read_to_string()
            .with(eq(test_path))
            .times(1)
            .returning(|_| {
                Ok(r#"{
                    "base_prompt": "You are a helpful assistant.",
                    "context": "Additional context here."
                }"#
                .to_string())
            });

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let prompts = storage.load_system_prompts().await.unwrap();
        assert_eq!(prompts.base_prompt, "You are a helpful assistant.");
        assert_eq!(
            prompts.context,
            Some("Additional context here.".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_load_system_prompts_default() {
        let mut mock_fs = MockFileSystem::new();

        // File doesn't exist
        let test_path = PathBuf::from("data/system_prompts.json");
        mock_fs
            .expect_exists()
            .with(eq(test_path))
            .times(1)
            .returning(|_| false);

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let prompts = storage.load_system_prompts().await.unwrap();
        assert_eq!(prompts.base_prompt, "");
        assert_eq!(prompts.context, None);
    }

    #[tokio::test]
    async fn test_mock_save_system_prompts_success() {
        let mut mock_fs = MockFileSystem::new();

        let base_path = PathBuf::from("data");
        let test_path = base_path.join("system_prompts.json");

        mock_fs
            .expect_exists()
            .with(eq(base_path.clone()))
            .times(1)
            .returning(|_| true);

        mock_fs
            .expect_write()
            .with(
                eq(test_path),
                function(|s: &str| s.contains("base_prompt") && s.contains("Test base prompt")),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        let prompts = create_test_system_prompts("Test base prompt", Some("Test context"));
        assert!(storage.save_system_prompts(&prompts).await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_concurrent_reads() {
        // Test that service is Send + Sync
        let mut mock_fs = MockFileSystem::new();

        mock_fs.expect_exists().returning(|_| false);

        let storage = Arc::new(FileStorageService::new(
            Arc::new(mock_fs),
            PathBuf::from("data"),
        ));

        let storage1 = storage.clone();
        let storage2 = storage.clone();

        // Spawn concurrent reads
        let handle1 = tokio::spawn(async move { storage1.load_token_stats().await });

        let handle2 = tokio::spawn(async move { storage2.load_system_prompts().await });

        // Both should complete without deadlock
        let (result1, result2) = tokio::try_join!(handle1, handle2).unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_mock_directory_creation_once() {
        let mut mock_fs = MockFileSystem::new();

        let base_path = PathBuf::from("data");

        // First call: directory doesn't exist
        mock_fs
            .expect_exists()
            .with(eq(base_path.clone()))
            .times(2) // Called twice (for two saves)
            .returning(|_| false);

        // Directory creation should be called twice (once per save)
        mock_fs
            .expect_create_dir_all()
            .with(eq(base_path.clone()))
            .times(2)
            .returning(|_| Ok(()));

        // Write should be called twice
        mock_fs.expect_write().times(2).returning(|_, _| Ok(()));

        let storage = FileStorageService::new(Arc::new(mock_fs), PathBuf::from("data"));

        // Save twice
        let stats = TokenStats::default();
        storage.save_token_stats(&stats).await.unwrap();
        storage.save_token_stats(&stats).await.unwrap();
    }
}
