//! **BEFORE REFACTORING - Current Pattern (Anti-Pattern)**
//!
//! This example demonstrates the CURRENT implementation pattern used in Rustbot
//! where business logic is tightly coupled to filesystem I/O operations.
//!
//! **Problems with this approach:**
//! - ❌ Cannot unit test without touching filesystem
//! - ❌ Hard to mock for testing
//! - ❌ Tight coupling between business logic and I/O
//! - ❌ No dependency injection
//! - ❌ Difficult to swap implementations (e.g., JSON → Database)
//! - ❌ Hard to test error conditions
//!
//! **Run this example:**
//! ```bash
//! cargo run --example before_refactoring
//! ```
//!
//! This will create a real file `token_stats.json` in your current directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Token usage statistics
/// This is copied from src/ui/types.rs to show realistic usage
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// **PROBLEM**: This struct directly handles file I/O in its methods
/// Making it impossible to test without filesystem access
pub struct RustbotApp {
    pub token_stats: TokenStats,
    // In real app, there are many more fields here...
    // message_input: String,
    // messages: Vec<ChatMessage>,
    // etc.
}

impl RustbotApp {
    /// Constructor loads data directly from filesystem
    /// **PROBLEM**: Cannot test this without real file I/O
    pub fn new() -> Result<Self, String> {
        // Load token stats from file
        let token_stats = Self::load_token_stats()?;

        Ok(Self { token_stats })
    }

    /// **PROBLEM**: Static method directly accesses filesystem
    /// - Hard to mock
    /// - Hard to test error conditions
    /// - Tightly coupled to file format
    fn load_token_stats() -> Result<TokenStats, String> {
        let path = Self::get_stats_file_path();

        // If file doesn't exist, return defaults
        if !path.exists() {
            println!("No token_stats.json found, using defaults");
            return Ok(TokenStats::default());
        }

        // Read from filesystem
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read token stats from {:?}: {}", path, e))?;

        // Deserialize
        let stats: TokenStats = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse token stats JSON: {}", e))?;

        println!("✓ Loaded token stats from {:?}", path);
        Ok(stats)
    }

    /// **PROBLEM**: Instance method directly writes to filesystem
    /// - Cannot test without creating real files
    /// - No way to inject test doubles
    pub fn save_token_stats(&self) -> Result<(), String> {
        let path = Self::get_stats_file_path();

        // Serialize
        let content = serde_json::to_string_pretty(&self.token_stats)
            .map_err(|e| format!("Failed to serialize token stats: {}", e))?;

        // Write to filesystem
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write token stats to {:?}: {}", path, e))?;

        println!("✓ Saved token stats to {:?}", path);
        Ok(())
    }

    /// **PROBLEM**: Hard-coded file path
    /// - No flexibility to change storage location
    /// - Cannot redirect to test directory
    fn get_stats_file_path() -> PathBuf {
        PathBuf::from("token_stats.json")
    }

    /// Business logic: Update token counts after API call
    /// This is the logic we actually want to test!
    pub fn update_token_usage(&mut self, input_tokens: u32, output_tokens: u32) {
        self.token_stats.daily_input += input_tokens;
        self.token_stats.daily_output += output_tokens;
        self.token_stats.total_input += input_tokens;
        self.token_stats.total_output += output_tokens;
    }

    /// Calculate cost based on token usage
    /// More business logic we want to test
    pub fn calculate_cost(&self) -> f64 {
        // Claude Sonnet 4 pricing (example)
        let input_cost_per_1k = 0.003; // $0.003 per 1K input tokens
        let output_cost_per_1k = 0.015; // $0.015 per 1K output tokens

        let input_cost = (self.token_stats.total_input as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (self.token_stats.total_output as f64 / 1000.0) * output_cost_per_1k;

        input_cost + output_cost
    }
}

fn main() {
    println!("=== BEFORE REFACTORING - Current Pattern ===\n");

    // Create new app (loads from filesystem)
    let mut app = RustbotApp::new().expect("Failed to create app");

    println!("\nInitial stats:");
    println!("  Daily input:  {}", app.token_stats.daily_input);
    println!("  Daily output: {}", app.token_stats.daily_output);
    println!("  Total input:  {}", app.token_stats.total_input);
    println!("  Total output: {}", app.token_stats.total_output);

    // Simulate an API call
    println!("\nSimulating API call with 1000 input tokens, 500 output tokens...");
    app.update_token_usage(1000, 500);

    println!("\nUpdated stats:");
    println!("  Daily input:  {}", app.token_stats.daily_input);
    println!("  Daily output: {}", app.token_stats.daily_output);
    println!("  Total input:  {}", app.token_stats.total_input);
    println!("  Total output: {}", app.token_stats.total_output);
    println!("  Total cost:   ${:.4}", app.calculate_cost());

    // Save to filesystem
    app.save_token_stats().expect("Failed to save token stats");

    println!("\n=== Problems with this approach ===");
    println!("❌ Cannot test update_token_usage() without filesystem");
    println!("❌ Cannot test calculate_cost() independently");
    println!("❌ Cannot test error conditions (disk full, permissions)");
    println!("❌ Cannot swap storage backend (e.g., to database)");
    println!("❌ Tests pollute filesystem with test data");
    println!("❌ No dependency injection");
    println!("\n💡 See examples/after_refactoring.rs for the solution!");
}

// **PROBLEM**: Cannot write unit tests without filesystem I/O
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_token_usage() {
        // ❌ This test requires creating a real file!
        let mut app = RustbotApp::new().unwrap();

        app.update_token_usage(100, 50);

        assert_eq!(app.token_stats.daily_input, 100);
        assert_eq!(app.token_stats.daily_output, 50);

        // ❌ Test pollutes filesystem with token_stats.json
    }

    #[test]
    fn test_calculate_cost() {
        // ❌ Still requires filesystem access to create app
        let mut app = RustbotApp::new().unwrap();

        app.token_stats.total_input = 10000;
        app.token_stats.total_output = 5000;

        let cost = app.calculate_cost();

        // Expected: (10000/1000 * 0.003) + (5000/1000 * 0.015)
        // = (10 * 0.003) + (5 * 0.015)
        // = 0.03 + 0.075 = 0.105
        assert!((cost - 0.105).abs() < 0.001);

        // ❌ Cannot test in isolation from file I/O
    }

    // ❌ IMPOSSIBLE: Cannot test error conditions
    // How do we test what happens when:
    // - Disk is full?
    // - File is corrupted?
    // - No write permissions?
    // - JSON is invalid?
    //
    // Answer: We can't, without complex filesystem mocking
}
