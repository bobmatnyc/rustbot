// Integration tests for service layer
//
// Design Decision: Integration tests using real implementations
//
// Rationale: While unit tests use mocks for isolation, integration tests
// verify that real implementations work together correctly. These tests
// use temporary directories and real filesystem operations.
//
// Usage:
//     cargo test --lib services::integration_tests

#[cfg(test)]
mod integration {
    use crate::services::traits::*;
    use crate::services::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_storage_workflow() {
        // Use real implementations with temp directory
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);
        let storage = FileStorageService::new(fs, temp_dir.path().to_path_buf());

        // Save token stats
        let mut stats = TokenStats::default();
        stats.total_input_tokens = 500;
        stats.total_output_tokens = 250;
        stats.total_cost = 0.05;

        storage.save_token_stats(&stats).await.unwrap();

        // Load stats back
        let loaded = storage.load_token_stats().await.unwrap();
        assert_eq!(loaded.total_input_tokens, 500);
        assert_eq!(loaded.total_output_tokens, 250);
        assert_eq!(loaded.total_cost, 0.05);

        // Save system prompts
        let prompts = SystemPrompts {
            base_prompt: "You are helpful.".to_string(),
            context: Some("Extra context".to_string()),
        };

        storage.save_system_prompts(&prompts).await.unwrap();

        // Load prompts back
        let loaded_prompts = storage.load_system_prompts().await.unwrap();
        assert_eq!(loaded_prompts.base_prompt, "You are helpful.");
        assert_eq!(loaded_prompts.context, Some("Extra context".to_string()));
    }

    #[tokio::test]
    async fn test_storage_persistence_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // First instance saves data
        {
            let fs = Arc::new(RealFileSystem);
            let storage = FileStorageService::new(fs, base_path.clone());

            let mut stats = TokenStats::default();
            stats.total_input_tokens = 1000;
            storage.save_token_stats(&stats).await.unwrap();
        }

        // Second instance loads data
        {
            let fs = Arc::new(RealFileSystem);
            let storage = FileStorageService::new(fs, base_path);

            let loaded = storage.load_token_stats().await.unwrap();
            assert_eq!(loaded.total_input_tokens, 1000);
        }
    }

    #[tokio::test]
    async fn test_filesystem_operations_integration() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem;

        let test_file = temp_dir.path().join("test.txt");
        let test_dir = temp_dir.path().join("subdir");

        // Write file
        fs.write(&test_file, "Hello, world!").await.unwrap();

        // Check existence
        assert!(fs.exists(&test_file).await);
        assert!(!fs.exists(&test_dir).await);

        // Create directory
        fs.create_dir_all(&test_dir).await.unwrap();
        assert!(fs.exists(&test_dir).await);

        // Read file
        let content = fs.read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "Hello, world!");

        // Read directory
        let entries = fs.read_dir(temp_dir.path()).await.unwrap();
        assert_eq!(entries.len(), 2); // test.txt and subdir
    }

    #[tokio::test]
    async fn test_nested_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem;

        let nested = temp_dir.path().join("a").join("b").join("c").join("d");

        // Create deeply nested directory
        fs.create_dir_all(&nested).await.unwrap();

        assert!(fs.exists(&nested).await);

        // Write file in nested directory
        let file = nested.join("test.txt");
        fs.write(&file, "nested content").await.unwrap();

        let content = fs.read_to_string(&file).await.unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_concurrent_filesystem_operations() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(RealFileSystem);

        // Spawn multiple concurrent writes
        let mut handles = vec![];

        for i in 0..10 {
            let fs_clone = fs.clone();
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));

            let handle = tokio::spawn(async move {
                fs_clone
                    .write(&file_path, &format!("Content {}", i))
                    .await
                    .unwrap();
            });

            handles.push(handle);
        }

        // Wait for all writes to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all files were written
        let entries = fs.read_dir(temp_dir.path()).await.unwrap();
        assert_eq!(entries.len(), 10);

        // Read all files concurrently
        let mut read_handles = vec![];

        for i in 0..10 {
            let fs_clone = fs.clone();
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));

            let handle = tokio::spawn(async move {
                let content = fs_clone.read_to_string(&file_path).await.unwrap();
                assert_eq!(content, format!("Content {}", i));
            });

            read_handles.push(handle);
        }

        for handle in read_handles {
            handle.await.unwrap();
        }
    }
}
