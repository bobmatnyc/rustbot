// UI module for Rustbot
// Contains all UI-related types, utilities, and views

pub mod icon;
pub mod plugins;
pub mod types;
pub mod views;
pub mod marketplace;

// Re-export commonly used types for convenience
pub use types::{
    AppView, ChatMessage, ContextTracker, MessageRole,
    SettingsView, SystemPrompts, TokenStats, VisualEvent,
};

pub use plugins::PluginsView;
pub use marketplace::MarketplaceView;
