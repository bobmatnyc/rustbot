use serde::{Deserialize, Serialize};

/// Type of LLM adapter to use
#[derive(Debug, Clone, Copy)]
pub enum AdapterType {
    OpenRouter,
    // Future options:
    // Anthropic,
    // OpenAI,
}

/// LLM provider enumeration for JSON configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenRouter,
    OpenAI,
    Anthropic,
    Ollama,
}

impl LlmProvider {
    /// Get the default API base URL for this provider
    pub fn default_api_base(&self) -> &str {
        match self {
            LlmProvider::OpenRouter => "https://openrouter.ai/api/v1",
            LlmProvider::OpenAI => "https://api.openai.com/v1",
            LlmProvider::Anthropic => "https://api.anthropic.com/v1",
            LlmProvider::Ollama => "http://localhost:11434",
        }
    }

    /// Get the default environment variable name for this provider's API key
    pub fn default_env_var(&self) -> &str {
        match self {
            LlmProvider::OpenRouter => "OPENROUTER_API_KEY",
            LlmProvider::OpenAI => "OPENAI_API_KEY",
            LlmProvider::Anthropic => "ANTHROPIC_API_KEY",
            LlmProvider::Ollama => "", // No API key needed for local Ollama
        }
    }

    /// Check if this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, LlmProvider::Ollama)
    }
}

/// Unified request format for all LLM adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>, // Override default model
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>, // For function calling
    pub web_search: Option<bool>, // Enable web search capabilities
}

impl LlmRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            web_search: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_web_search(mut self, enabled: bool) -> Self {
        self.web_search = Some(enabled);
        self
    }
}

/// Unified response format from LLM adapters
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// A tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
