mod openrouter;
mod types;

pub use openrouter::OpenRouterAdapter;
pub use types::*;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Unified LLM interface that all adapters must implement
/// Supports streaming, structured responses, and tool calls
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Stream a chat completion response
    /// Sends chunks of text through the provided channel as they arrive
    async fn stream_chat(
        &self,
        request: LlmRequest,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<()>;

    /// Get a complete chat response (non-streaming)
    /// Useful for tool calls and structured outputs
    async fn complete_chat(&self, request: LlmRequest) -> Result<LlmResponse>;

    /// Get the adapter name for logging/debugging
    fn name(&self) -> &str;
}

/// Factory function to create the appropriate LLM adapter
pub fn create_adapter(adapter_type: AdapterType, api_key: String) -> Box<dyn LlmAdapter> {
    match adapter_type {
        AdapterType::OpenRouter => Box::new(OpenRouterAdapter::new(api_key)),
        // Future adapters can be added here:
        // AdapterType::Anthropic => Box::new(AnthropicAdapter::new(api_key)),
        // AdapterType::OpenAI => Box::new(OpenAIAdapter::new(api_key)),
    }
}
