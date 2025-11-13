// Tool definitions for converting agents to OpenAI-compatible function calling format
//
// This module implements the industry-standard OpenAI function calling specification
// to enable the primary assistant agent to delegate to specialist agents.
//
// Reference: https://platform.openai.com/docs/guides/function-calling
// OpenRouter Docs: https://openrouter.ai/docs/features/tool-calling

use crate::agent::AgentConfig;
use serde::{Deserialize, Serialize};

/// Tool definition in OpenAI function calling format
///
/// This represents a tool (specialist agent) that can be called by the LLM.
/// The LLM will decide when to call tools based on the user's message and
/// the tool descriptions.
///
/// # Example
/// ```json
/// {
///   "type": "function",
///   "function": {
///     "name": "web_search",
///     "description": "Search the web for current information",
///     "parameters": {
///       "type": "object",
///       "properties": {
///         "query": {
///           "type": "string",
///           "description": "The search query"
///         }
///       },
///       "required": ["query"]
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Always "function" for function calling
    #[serde(rename = "type")]
    pub tool_type: String,

    /// The function definition
    pub function: FunctionDefinition,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name (matches agent ID)
    pub name: String,

    /// Clear description of what this function does and when to use it
    pub description: String,

    /// JSON schema for the function parameters
    pub parameters: FunctionParameters,
}

/// Parameters schema for a function (JSON Schema format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameters {
    /// Always "object" for parameter schemas
    #[serde(rename = "type")]
    pub param_type: String,

    /// Properties of the object (parameter definitions)
    pub properties: serde_json::Value,

    /// List of required parameter names
    pub required: Vec<String>,
}

impl ToolDefinition {
    /// Convert an enabled specialist agent to a tool definition
    ///
    /// Only specialist agents (is_primary = false) that are enabled should be
    /// converted to tools. Primary agents are never tools.
    ///
    /// # Arguments
    /// * `agent` - The agent configuration to convert
    ///
    /// # Returns
    /// A tool definition in OpenAI function calling format
    ///
    /// # Panics
    /// Panics if called with a primary agent or disabled agent
    pub fn from_agent(agent: &AgentConfig) -> Self {
        // Safety check - only enabled specialist agents should become tools
        assert!(
            !agent.is_primary && agent.enabled,
            "Only enabled specialist agents can be converted to tools"
        );

        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: agent.id.clone(),
                description: Self::extract_tool_description(agent),
                parameters: Self::build_parameters(agent),
            },
        }
    }

    /// Extract a clear, concise tool description from agent instructions
    ///
    /// The description helps the LLM decide when to use this tool.
    /// We extract the first meaningful paragraph from the instructions.
    ///
    /// # Arguments
    /// * `agent` - The agent configuration
    ///
    /// # Returns
    /// A clear description suitable for tool calling
    fn extract_tool_description(agent: &AgentConfig) -> String {
        // For web_search, use a specific description
        if agent.id == "web_search" {
            return "Search the web for current, real-time information. Use this when the user asks about recent events, current data, weather, news, or any information after your knowledge cutoff. Provide a clear, specific search query.".to_string();
        }

        // For other agents, extract first paragraph from instructions
        // This is usually a good summary of what the agent does
        let description = agent
            .instructions
            .lines()
            .take_while(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        // If we got a description, use it; otherwise fall back to agent name
        if !description.is_empty() {
            description
        } else {
            format!("{} agent", agent.name)
        }
    }

    /// Build the parameter schema for this agent
    ///
    /// Different agents may accept different parameters. For now, we have
    /// two patterns:
    /// - Web search: Takes a "query" parameter
    /// - Generic agents: Take a "message" parameter
    ///
    /// # Arguments
    /// * `agent` - The agent configuration
    ///
    /// # Returns
    /// JSON schema for the function parameters
    fn build_parameters(agent: &AgentConfig) -> FunctionParameters {
        if agent.id == "web_search" {
            // Web search takes a specific query parameter
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "query": {
                        "type": "string",
                        "description": "The search query to execute"
                    }
                }),
                required: vec!["query".to_string()],
            }
        } else {
            // Generic agents take a message parameter
            FunctionParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "message": {
                        "type": "string",
                        "description": "The message to send to the agent"
                    }
                }),
                required: vec!["message".to_string()],
            }
        }
    }

    /// Build a list of tool definitions from all enabled specialist agents
    ///
    /// This is a convenience method to convert multiple agents at once.
    ///
    /// # Arguments
    /// * `agents` - Iterator of agent configurations
    ///
    /// # Returns
    /// Vector of tool definitions for all enabled specialists
    pub fn from_agents<'a, I>(agents: I) -> Vec<Self>
    where
        I: IntoIterator<Item = &'a AgentConfig>,
    {
        agents
            .into_iter()
            .filter(|agent| !agent.is_primary && agent.enabled)
            .map(Self::from_agent)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_definition() {
        let agent = AgentConfig {
            id: "web_search".to_string(),
            name: "Web Search".to_string(),
            instructions: "You are a web search specialist.".to_string(),
            personality: None,
            model: "anthropic/claude-3.5-haiku".to_string(),
            enabled: true,
            is_primary: false,
            web_search_enabled: true,
        };

        let tool = ToolDefinition::from_agent(&agent);

        assert_eq!(tool.tool_type, "function");
        assert_eq!(tool.function.name, "web_search");
        assert!(tool.function.description.contains("Search the web"));
        assert_eq!(tool.function.parameters.param_type, "object");
        assert_eq!(tool.function.parameters.required, vec!["query"]);

        // Check that parameters are properly formatted
        let params = &tool.function.parameters.properties;
        assert!(params.get("query").is_some());
    }

    #[test]
    fn test_generic_agent_tool_definition() {
        let agent = AgentConfig {
            id: "code_helper".to_string(),
            name: "Code Helper".to_string(),
            instructions: "You help with coding tasks.".to_string(),
            personality: None,
            model: "anthropic/claude-sonnet-4.5".to_string(),
            enabled: true,
            is_primary: false,
            web_search_enabled: false,
        };

        let tool = ToolDefinition::from_agent(&agent);

        assert_eq!(tool.function.name, "code_helper");
        assert_eq!(tool.function.parameters.required, vec!["message"]);

        let params = &tool.function.parameters.properties;
        assert!(params.get("message").is_some());
    }

    #[test]
    #[should_panic(expected = "Only enabled specialist agents can be converted to tools")]
    fn test_primary_agent_cannot_be_tool() {
        let agent = AgentConfig {
            id: "assistant".to_string(),
            name: "Assistant".to_string(),
            instructions: "Main assistant".to_string(),
            personality: None,
            model: "anthropic/claude-sonnet-4.5".to_string(),
            enabled: true,
            is_primary: true, // Primary agent
            web_search_enabled: false,
        };

        // This should panic
        ToolDefinition::from_agent(&agent);
    }

    #[test]
    #[should_panic(expected = "Only enabled specialist agents can be converted to tools")]
    fn test_disabled_agent_cannot_be_tool() {
        let agent = AgentConfig {
            id: "web_search".to_string(),
            name: "Web Search".to_string(),
            instructions: "Search agent".to_string(),
            personality: None,
            model: "anthropic/claude-3.5-haiku".to_string(),
            enabled: false, // Disabled
            is_primary: false,
            web_search_enabled: true,
        };

        // This should panic
        ToolDefinition::from_agent(&agent);
    }

    #[test]
    fn test_from_agents_filters_correctly() {
        let agents = vec![
            AgentConfig {
                id: "assistant".to_string(),
                name: "Assistant".to_string(),
                instructions: "Main".to_string(),
                personality: None,
                model: "anthropic/claude-sonnet-4.5".to_string(),
                enabled: true,
                is_primary: true, // Should be filtered out
                web_search_enabled: false,
            },
            AgentConfig {
                id: "web_search".to_string(),
                name: "Web Search".to_string(),
                instructions: "Search".to_string(),
                personality: None,
                model: "anthropic/claude-3.5-haiku".to_string(),
                enabled: true,
                is_primary: false, // Should be included
                web_search_enabled: true,
            },
            AgentConfig {
                id: "code_helper".to_string(),
                name: "Code Helper".to_string(),
                instructions: "Code".to_string(),
                personality: None,
                model: "anthropic/claude-sonnet-4.5".to_string(),
                enabled: false, // Should be filtered out
                is_primary: false,
                web_search_enabled: false,
            },
        ];

        let tools = ToolDefinition::from_agents(&agents);

        // Should only have web_search (enabled specialist)
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "web_search");
    }
}
