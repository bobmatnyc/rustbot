// UI module for Rustbot
// Contains all UI-related types, utilities, and views

pub mod icon;
pub mod types;
pub mod views;

// Re-export commonly used types for convenience
pub use types::{
    AppView, ChatMessage, ContextTracker, MessageRole,
    SettingsView, SystemPrompts, TokenStats, VisualEvent,
};
