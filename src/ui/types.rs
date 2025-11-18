// UI type definitions for Rustbot
// Contains data structures used throughout the UI

use serde::{Deserialize, Serialize};

/// Event visualization structure
pub struct VisualEvent {
    pub source: String,
    pub destination: String,
    pub kind: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

/// Main application view
#[derive(PartialEq)]
pub enum AppView {
    Chat,
    Settings,
    Events,
    Extensions,
}

/// Settings sub-view
#[derive(PartialEq, Clone)]
pub enum SettingsView {
    SystemPrompts,
    Agents,
}

/// Extensions sub-view (Marketplace, Installed)
#[derive(PartialEq, Clone)]
pub enum ExtensionsView {
    Marketplace, // Browse available MCP servers
    Installed,   // View and manage installed extensions (with filtering)
}

impl Default for ExtensionsView {
    fn default() -> Self {
        Self::Marketplace
    }
}

/// Filter for installed extensions by installation type
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum InstallTypeFilter {
    All,    // Show all installed extensions
    Remote, // Show only remote/cloud extensions
    Local,  // Show only local extensions
}

impl Default for InstallTypeFilter {
    fn default() -> Self {
        Self::All
    }
}

impl InstallTypeFilter {
    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::Remote => "Remote",
            Self::Local => "Local",
        }
    }
}

/// System prompts configuration
#[derive(Serialize, Deserialize, Clone)]
pub struct SystemPrompts {
    pub system_instructions: String,
}

impl Default for SystemPrompts {
    fn default() -> Self {
        Self {
            system_instructions: "You are a helpful AI assistant.".to_string(),
        }
    }
}

/// Chat message with role and content
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    /// Embedded image data URLs (extracted from markdown for easy access)
    pub embedded_images: Vec<String>,
}

/// Token usage statistics
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct TokenStats {
    pub daily_input: u32,
    pub daily_output: u32,
    pub total_input: u32,
    pub total_output: u32,
    #[serde(default)]
    pub last_reset_date: String, // Track when daily stats were last reset
}

/// Context window tracker
#[derive(Clone)]
pub struct ContextTracker {
    pub max_tokens: u32,     // Model's context window (200k for Claude Sonnet 4.5)
    pub current_tokens: u32, // Current estimated token usage
    pub system_tokens: u32,  // Tokens used by system context (dynamic)
    pub conversation_tokens: u32, // Tokens used by conversation history
    pub compaction_threshold: f32, // Trigger compaction (default: 0.50 = 50%)
    pub warning_threshold: f32, // Show warning (default: 0.75 = 75%)
}

impl Default for ContextTracker {
    fn default() -> Self {
        Self {
            max_tokens: 200_000, // Claude Sonnet 4.5 context window
            current_tokens: 0,
            system_tokens: 0,
            conversation_tokens: 0,
            compaction_threshold: 0.50,
            warning_threshold: 0.75,
        }
    }
}

impl ContextTracker {
    pub fn usage_percentage(&self) -> f32 {
        if self.max_tokens == 0 {
            0.0
        } else {
            (self.current_tokens as f32 / self.max_tokens as f32) * 100.0
        }
    }

    pub fn get_color(&self) -> egui::Color32 {
        let percentage = self.usage_percentage();
        if percentage < 50.0 {
            egui::Color32::from_rgb(60, 150, 60) // Green
        } else if percentage < 75.0 {
            egui::Color32::from_rgb(200, 180, 50) // Yellow
        } else if percentage < 90.0 {
            egui::Color32::from_rgb(220, 120, 40) // Orange
        } else {
            egui::Color32::from_rgb(200, 60, 60) // Red
        }
    }

    pub fn update_counts(&mut self, system_tokens: u32, conversation_tokens: u32) {
        self.system_tokens = system_tokens;
        self.conversation_tokens = conversation_tokens;
        self.current_tokens = system_tokens + conversation_tokens;
    }

    pub fn should_compact(&self) -> bool {
        self.usage_percentage() >= (self.compaction_threshold * 100.0)
    }

    pub fn should_warn(&self) -> bool {
        self.usage_percentage() >= (self.warning_threshold * 100.0)
    }
}

/// Message role (User or Assistant)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}
