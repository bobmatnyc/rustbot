// Agent Configurations Module
// Contains pre-configured agent templates for specialized tasks

pub mod web_search;

// Re-export commonly used agent creators
pub use web_search::create_web_search_agent;
