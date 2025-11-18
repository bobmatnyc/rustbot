// Real filesystem implementation for production use
//
// Design Decision: Thin wrapper around tokio::fs
//
// Rationale: Keep implementation simple and focused on the trait contract.
// tokio::fs provides async file I/O that works with tokio runtime.
//
// This is the "real" adapter in hexagonal architecture - connects business
// logic to actual OS filesystem. Test code uses MockFileSystem instead.

use super::traits::FileSystem;
use crate::error::{Result, RustbotError};
use async_trait::async_trait;
use std::path::Path;

/// Real filesystem implementation using tokio::fs
///
/// Zero-cost wrapper around tokio filesystem operations.
/// All operations are async and work with tokio runtime.
///
/// Thread Safety: All operations are safe to call from multiple threads.
/// tokio::fs handles synchronization internally.
///
/// Usage:
///     let fs = RealFileSystem;
///     let content = fs.read_to_string(Path::new("config.json")).await?;
pub struct RealFileSystem;

#[async_trait]
impl FileSystem for RealFileSystem {
    async fn read_to_string(&self, path: &Path) -> Result<String> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| RustbotError::IoError(e))
    }

    async fn write(&self, path: &Path, content: &str) -> Result<()> {
        tokio::fs::write(path, content)
            .await
            .map_err(|e| RustbotError::IoError(e))
    }

    async fn exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| RustbotError::IoError(e))
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path)
            .await
            .map_err(|e| RustbotError::IoError(e))?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| RustbotError::IoError(e))? {
            entries.push(entry.path());
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_real_filesystem_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let fs = RealFileSystem;

        // Test write
        fs.write(&test_file, "Hello, world!")
            .await
            .expect("Failed to write file");

        // Test read
        let content = fs.read_to_string(&test_file)
            .await
            .expect("Failed to read file");

        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_real_filesystem_exists() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let fs = RealFileSystem;

        // File doesn't exist yet
        assert!(!fs.exists(&test_file).await);

        // Create file
        fs.write(&test_file, "test").await.unwrap();

        // Now it exists
        assert!(fs.exists(&test_file).await);
    }

    #[tokio::test]
    async fn test_real_filesystem_create_dir_all() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("a").join("b").join("c");

        let fs = RealFileSystem;

        // Create nested directories
        fs.create_dir_all(&nested_dir)
            .await
            .expect("Failed to create directories");

        // Verify they exist
        assert!(fs.exists(&nested_dir).await);
    }

    #[tokio::test]
    async fn test_real_filesystem_read_dir() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem;

        // Create some files
        fs.write(&temp_dir.path().join("file1.txt"), "content1").await.unwrap();
        fs.write(&temp_dir.path().join("file2.txt"), "content2").await.unwrap();

        // Read directory
        let entries = fs.read_dir(temp_dir.path()).await.unwrap();

        // Should have 2 entries
        assert_eq!(entries.len(), 2);

        // Check that both files are listed
        let file_names: Vec<_> = entries.iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(file_names.contains(&"file1.txt"));
        assert!(file_names.contains(&"file2.txt"));
    }

    #[tokio::test]
    async fn test_real_filesystem_read_nonexistent_file() {
        let fs = RealFileSystem;
        let result = fs.read_to_string(Path::new("/nonexistent/file.txt")).await;

        assert!(result.is_err());
        match result {
            Err(RustbotError::IoError(_)) => {}, // Expected
            _ => panic!("Expected IoError"),
        }
    }
}
