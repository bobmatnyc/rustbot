// UI module for Rustbot
// Contains all UI-related types, utilities, and views

pub mod icon;
pub mod marketplace;
pub mod plugins;
pub mod types;
pub mod views;

// Re-export commonly used types for convenience
pub use types::{
    AppView, ChatMessage, ContextTracker, ExtensionsView, MessageRole, SettingsView, SystemPrompts,
    TokenStats, VisualEvent,
};

pub use marketplace::MarketplaceView;
pub use plugins::PluginsView;
