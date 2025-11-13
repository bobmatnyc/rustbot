// Web Search Agent Configuration
// This agent specializes in using web search to find current, relevant information

use crate::agent::AgentConfig;

/// Create a web search specialist agent
///
/// This agent is configured to use web search capabilities to answer
/// questions requiring current information, recent events, or real-time data.
///
/// # Features
/// - Web search enabled by default
/// - Lightweight model (Haiku) for fast responses
/// - Specialized instructions for search result synthesis
/// - No personality (pure tool agent)
///
/// # Returns
/// An `AgentConfig` configured for web search operations
pub fn create_web_search_agent() -> AgentConfig {
    AgentConfig {
        id: "web_search".to_string(),
        name: "Web Search".to_string(),
        instructions: r#"You are a web search specialist agent.

Your job is to:
1. Understand the user's search intent
2. Use web search capabilities to find current, relevant information
3. Synthesize findings into a clear, concise response
4. Cite your sources with URLs

Always provide:
- Direct answers to the query
- Key findings from search results
- Source URLs for verification
- Publication dates when available

Guidelines:
- Be concise but thorough
- Focus on factual, current information
- Prioritize authoritative sources
- Acknowledge if information is conflicting or uncertain
- Distinguish between facts and opinions in sources

Format your responses clearly with:
- Summary answer first
- Supporting details from sources
- Source citations at the end"#.to_string(),
        personality: None,  // No personality for tool agents
        model: "anthropic/claude-3.5-haiku".to_string(),  // Lightweight model for speed
        enabled: true,
        is_primary: false,  // Specialist agent, not primary
        web_search_enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_agent_creation() {
        let agent = create_web_search_agent();

        assert_eq!(agent.id, "web_search");
        assert_eq!(agent.name, "Web Search");
        assert!(agent.web_search_enabled);
        assert!(agent.enabled);
        assert!(agent.personality.is_none());
        assert!(agent.instructions.contains("web search specialist"));
    }

    #[test]
    fn test_web_search_agent_uses_haiku() {
        let agent = create_web_search_agent();

        // Verify using lightweight model for web search
        assert_eq!(agent.model, "anthropic/claude-3.5-haiku");
    }
}
