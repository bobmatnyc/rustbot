use serde::{Deserialize, Serialize};

/// Type of LLM adapter to use
#[derive(Debug, Clone, Copy)]
pub enum AdapterType {
    OpenRouter,
    // Future options:
    // Anthropic,
    // OpenAI,
}

/// Unified request format for all LLM adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>, // Override default model
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>, // For function calling
}

impl LlmRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
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
