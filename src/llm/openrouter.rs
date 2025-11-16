use super::types::*;
use super::LlmAdapter;
use crate::agent::ToolDefinition;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODEL: &str = "openai/gpt-4o";

pub struct OpenRouterAdapter {
    client: Client,
    api_key: String,
}

impl OpenRouterAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    async fn send_request(&self, request: &ApiRequest) -> Result<reqwest::Response> {
        self.client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")
    }
}

#[async_trait]
impl LlmAdapter for OpenRouterAdapter {
    async fn stream_chat(
        &self,
        request: LlmRequest,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        tracing::debug!("⏱️  [LLM] stream_chat starting");

        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let is_anthropic = model.starts_with("anthropic/claude");

        // Convert messages to appropriate format based on model
        let messages = if is_anthropic {
            serialize_messages_for_anthropic_value(&request.messages)?
        } else {
            // OpenAI models - use standard format with tool_calls
            serialize_messages_for_openai_value(&request.messages)?
        };

        // Configure web search if enabled (OpenRouter-specific feature)
        // OpenRouter expects plugins array: [{"id": "web", "max_results": 5}]
        let plugins = if request.web_search == Some(true) {
            Some(vec![WebPlugin {
                id: "web".to_string(),  // Required value for web search
                max_results: Some(5),   // Default is 5 results per search
            }])
        } else {
            None
        };

        let api_request = ApiRequest {
            model,
            messages,
            stream: true,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools,  // Pass custom tools from request
            tool_choice: request.tool_choice,  // Pass tool_choice from request
            plugins,
            provider: None,  // Not using provider-specific config anymore
        };

        // DEBUG: Log the serialized request to see what's actually being sent
        tracing::debug!("🔍 [API] Sending request to model: {} (is_anthropic: {})", api_request.model, is_anthropic);
        tracing::debug!("🔍 [API] Sending request with {} messages", api_request.messages.len());
        for (idx, msg) in api_request.messages.iter().enumerate() {
            if let Ok(json) = serde_json::to_string(msg) {
                tracing::debug!("🔍 [API] Message[{}]: {}", idx, json);
            }
        }
        if let Ok(json) = serde_json::to_string_pretty(&api_request) {
            tracing::debug!("🔍 [API] Full request JSON:\n{}", json);
        }

        tracing::debug!("⏱️  [LLM] Sending stream request at {:?}", start_time.elapsed());
        let response = self.send_request(&api_request).await?;
        tracing::debug!("⏱️  [LLM] Stream response headers received at {:?}", start_time.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut first_chunk = true;

        while let Some(chunk) = stream.next().await {
            if first_chunk {
                tracing::debug!("⏱️  [LLM] First chunk received at {:?}", start_time.elapsed());
                first_chunk = false;
            }
            let chunk = chunk.context("Failed to read chunk from stream")?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE messages
            while let Some(pos) = buffer.find("\n\n") {
                let message = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                if message.trim().is_empty() {
                    continue;
                }

                // Parse SSE format: "data: {json}"
                for line in message.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];

                        if data == "[DONE]" {
                            return Ok(());
                        }

                        match serde_json::from_str::<StreamResponse>(data) {
                            Ok(response) => {
                                if let Some(choice) = response.choices.first() {
                                    if let Some(delta) = &choice.delta {
                                        // Handle content streaming
                                        if let Some(content) = &delta.content {
                                            if first_chunk {
                                                tracing::debug!("⏱️  [LLM] First content sent to channel at {:?}", start_time.elapsed());
                                                first_chunk = false;
                                            }
                                            if tx.send(content.clone()).is_err() {
                                                return Ok(()); // Receiver dropped
                                            }
                                        }

                                        // Detect tool calls
                                        if let Some(tool_calls) = &delta.tool_calls {
                                            tracing::info!("Tool call detected: {:?}", tool_calls);
                                            // TODO: Handle tool call routing to specialist agents
                                            // For now, just log it
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse chunk: {} - data: {}", e, data);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn complete_chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = std::time::Instant::now();
        tracing::debug!("⏱️  [LLM] complete_chat starting");

        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let is_anthropic = model.starts_with("anthropic/claude");

        // Convert messages to appropriate format based on model
        let messages = if is_anthropic {
            serialize_messages_for_anthropic_value(&request.messages)?
        } else {
            serialize_messages_for_openai_value(&request.messages)?
        };

        // Configure web search if enabled (OpenRouter-specific feature)
        // OpenRouter expects plugins array: [{"id": "web", "max_results": 5}]
        let plugins = if request.web_search == Some(true) {
            Some(vec![WebPlugin {
                id: "web".to_string(),  // Required value for web search
                max_results: Some(5),   // Default is 5 results per search
            }])
        } else {
            None
        };

        let api_request = ApiRequest {
            model,
            messages,
            stream: false,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools,  // Pass custom tools from request
            tool_choice: request.tool_choice,  // Pass tool_choice from request
            plugins,
            provider: None,  // Not using provider-specific config anymore
        };

        // Log detailed tool information for debugging
        if let Some(ref tools) = api_request.tools {
            tracing::info!("🔧 [LLM] Sending {} tools to API:", tools.len());
            for tool in tools {
                tracing::info!("   - Tool: {}", tool.function.name);
                tracing::info!("     Description: {}", tool.function.description);
            }
        } else {
            tracing::info!("🔧 [LLM] No tools in request");
        }

        if let Some(ref choice) = api_request.tool_choice {
            tracing::info!("🎯 [LLM] tool_choice: {:?}", choice);
        } else {
            tracing::info!("🎯 [LLM] tool_choice: auto (default)");
        }

        tracing::debug!("⏱️  [LLM] Sending request at {:?}", start_time.elapsed());
        let response = self.send_request(&api_request).await?;
        tracing::debug!("⏱️  [LLM] Response received at {:?}", start_time.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        // Get response text for debugging
        let response_text = response.text().await?;
        tracing::debug!("⏱️  [LLM] Response body read at {:?}", start_time.elapsed());
        tracing::debug!("OpenRouter raw response: {}", response_text);

        // Deserialize with detailed error reporting
        let completion: CompletionResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                tracing::error!("Failed to deserialize OpenRouter response: {}", e);
                tracing::error!("Raw response was: {}", response_text);
                anyhow::anyhow!("error decoding response body: {}", e)
            })?;

        let choice = completion
            .choices
            .first()
            .context("No choices in response")?;

        // Convert OpenRouter API format to our internal format
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            tracing::info!("📞 [LLM] Response contains {} tool call(s)", calls.len());
            calls
                .iter()
                .filter_map(|api_call| {
                    tracing::info!("   - Tool call: {} (id: {})", api_call.function.name, api_call.id);
                    // Parse the JSON arguments string into a Value
                    match serde_json::from_str(&api_call.function.arguments) {
                        Ok(args) => Some(ToolCall {
                            id: api_call.id.clone(),
                            name: api_call.function.name.clone(),
                            arguments: args,
                        }),
                        Err(e) => {
                            tracing::error!(
                                "Failed to parse tool arguments for {}: {}",
                                api_call.function.name,
                                e
                            );
                            None
                        }
                    }
                })
                .collect()
        });

        if tool_calls.is_none() {
            tracing::info!("📞 [LLM] Response contains NO tool calls - LLM responded directly");
        }

        Ok(LlmResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            tool_calls,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    fn name(&self) -> &str {
        "OpenRouter"
    }
}

// Internal API types
#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    // Messages are now serialized based on model type
    // - Anthropic models (claude-*): Use content blocks format
    // - OpenAI models (gpt-*, o1-*): Use standard OpenAI format with tool_calls
    messages: Vec<serde_json::Value>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,

    /// Custom tool definitions (OpenAI function calling format)
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,

    /// Tool choice parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,

    /// Web search plugin (OpenRouter feature)
    /// Format: [{"id": "web", "max_results": 5}]
    #[serde(skip_serializing_if = "Option::is_none")]
    plugins: Option<Vec<WebPlugin>>,

    /// Provider-specific configuration (e.g., for fallbacks)
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<ProviderConfig>,
}

/// Serialize messages for OpenAI models (GPT-4o, o1, etc.)
///
/// OpenAI format uses standard message structure with:
/// - Assistant messages with tool calls: `tool_calls` array with type="function"
/// - Tool result messages: role="tool" with `tool_call_id`
fn serialize_messages_for_openai_value(messages: &[Message]) -> Result<Vec<serde_json::Value>> {
    messages.iter().map(|msg| {
        // Convert our internal Message format to OpenAI API format
        let mut json = serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        });

        // Add tool_calls if present (for assistant messages)
        // OpenAI requires each tool call to have type="function"
        if let Some(tool_calls) = &msg.tool_calls {
            let openai_tool_calls: Vec<serde_json::Value> = tool_calls.iter().map(|tc| {
                serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                    }
                })
            }).collect();
            json["tool_calls"] = serde_json::Value::Array(openai_tool_calls);
        }

        // Add tool_call_id if present (for tool result messages)
        if let Some(tool_call_id) = &msg.tool_call_id {
            json["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
        }

        Ok(json)
    }).collect()
}

/// Serialize messages for Anthropic models (Claude)
///
/// Returns a Result with Vec<serde_json::Value> for consistency with OpenAI serializer
fn serialize_messages_for_anthropic_value(messages: &[Message]) -> Result<Vec<serde_json::Value>> {
    let mut result = Vec::new();

    for (idx, message) in messages.iter().enumerate() {
        let value = match message.role.as_str() {
            "tool" => {
                // Tool result message - convert to Anthropic's format
                let tool_use_id = message.tool_call_id.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("tool message missing tool_call_id"))?;

                if message.content.is_empty() {
                    anyhow::bail!("Tool result message {} has empty content (tool_use_id: {})", idx, tool_use_id);
                }

                serde_json::json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": message.content
                    }]
                })
            }
            "assistant" if message.tool_calls.is_some() => {
                // Assistant message with tool calls - convert to content blocks
                let mut content_blocks = Vec::new();

                // Add text content if present
                if !message.content.is_empty() {
                    content_blocks.push(serde_json::json!({
                        "type": "text",
                        "text": message.content
                    }));
                }

                // Add tool_use blocks
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        content_blocks.push(serde_json::json!({
                            "type": "tool_use",
                            "id": tool_call.id,
                            "name": tool_call.name,
                            "input": tool_call.arguments
                        }));
                    }
                }

                if content_blocks.is_empty() {
                    anyhow::bail!("Assistant message {} with tool_calls generated no content blocks", idx);
                }

                serde_json::json!({
                    "role": "assistant",
                    "content": content_blocks
                })
            }
            "assistant" => {
                // Regular assistant message
                if message.content.is_empty() {
                    anyhow::bail!("Assistant message {} has empty content", idx);
                }
                serde_json::json!({
                    "role": "assistant",
                    "content": message.content
                })
            }
            _ => {
                // User, system, or other messages
                if message.content.is_empty() {
                    anyhow::bail!("Message {} (role: {}) has empty content", idx, message.role);
                }
                serde_json::json!({
                    "role": message.role,
                    "content": message.content
                })
            }
        };

        result.push(value);
    }

    Ok(result)
}

/// Custom serializer for messages to handle Anthropic's format (DEPRECATED - kept for reference)
///
/// **Problem**: Anthropic API has a different message format than OpenAI:
/// - OpenAI: Uses `tool_calls` array on assistant messages, `tool_call_id` on tool messages
/// - Anthropic: Uses content blocks with `tool_use` type, `tool_use_id` on tool_result blocks
///
/// **Solution**: Transform our internal OpenAI-compatible format to Anthropic's format:
///
/// 1. **Tool Result Messages** (role="tool"):
///    - Convert to role="user" with content block type "tool_result"
///    - Use `tool_use_id` instead of `tool_call_id`
///
/// 2. **Assistant Messages with Tool Calls**:
///    - Convert to content blocks array
///    - Add text block for content (if present)
///    - Add tool_use blocks for each tool call with `id`, `name`, and `input`
///
/// 3. **Regular Messages**: Keep simple string content format
///
/// This ensures the message sequence is:
/// - User message
/// - Assistant message with tool_use blocks (containing tool IDs)
/// - User message with tool_result blocks (referencing those tool IDs)
fn serialize_messages_for_anthropic<S>(
    messages: &[Message],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;

    let mut seq = serializer.serialize_seq(Some(messages.len()))?;

    for (idx, message) in messages.iter().enumerate() {
        // Validate: Anthropic requires all messages (except final assistant) to have non-empty content
        if message.content.is_empty() && message.role != "tool" && message.tool_calls.is_none() {
            tracing::warn!("⚠️  Message {} has empty content (role: {})", idx, message.role);
        }

        // Convert each message to Anthropic format
        // See: https://docs.anthropic.com/en/docs/build-with-claude/tool-use
        let anthropic_msg = match message.role.as_str() {
            "tool" => {
                // Tool result message - use Anthropic's format with content blocks
                #[derive(Serialize)]
                struct ToolResultMessage<'a> {
                    role: &'a str,
                    content: Vec<ToolResultBlock>,
                }

                #[derive(Serialize)]
                struct ToolResultBlock {
                    #[serde(rename = "type")]
                    block_type: String,
                    tool_use_id: String,
                    content: String,
                }

                let tool_use_id = message.tool_call_id.as_ref()
                    .ok_or_else(|| serde::ser::Error::custom("tool message missing tool_call_id"))?;

                // DEBUG: Log tool result content
                tracing::debug!("Tool result message {}: tool_use_id={}, content_len={}",
                    idx, tool_use_id, message.content.len());

                // CRITICAL: Anthropic requires tool_result content to be non-empty
                if message.content.is_empty() {
                    tracing::error!("❌ Tool result message {} has EMPTY content! tool_use_id={}", idx, tool_use_id);
                    return Err(serde::ser::Error::custom(format!(
                        "Tool result message {} has empty content (tool_use_id: {})",
                        idx, tool_use_id
                    )));
                }

                serde_json::to_value(ToolResultMessage {
                    role: "user",  // Anthropic requires tool results to have role "user"
                    content: vec![ToolResultBlock {
                        block_type: "tool_result".to_string(),
                        tool_use_id: tool_use_id.clone(),
                        content: message.content.clone(),
                    }],
                }).map_err(serde::ser::Error::custom)?
            }
            "assistant" if message.tool_calls.is_some() => {
                // Assistant message with tool calls - convert to Anthropic's tool_use blocks
                //
                // CRITICAL BUG FIX: Anthropic's API rejects messages.2 as having "empty content"
                // even when our debug logs show content is present. The issue is that we're
                // creating serde_json::Value objects and then re-serializing them through the
                // custom serializer, which causes the content array to become empty.
                //
                // SOLUTION: Use a struct that implements Serialize directly, avoiding the
                // double-serialization issue with serde_json::Value.

                #[derive(Serialize)]
                struct AssistantMessage<'a> {
                    role: &'static str,
                    content: Vec<ContentBlock<'a>>,
                }

                #[derive(Serialize)]
                #[serde(tag = "type", rename_all = "snake_case")]
                enum ContentBlock<'a> {
                    Text {
                        text: &'a str,
                    },
                    #[serde(rename = "tool_use")]
                    ToolUse {
                        id: &'a str,
                        name: &'a str,
                        input: &'a serde_json::Value,
                    },
                }

                let mut content_blocks = Vec::new();

                // Add text content first (if present)
                if !message.content.is_empty() {
                    content_blocks.push(ContentBlock::Text {
                        text: &message.content,
                    });
                }

                // Add tool_use blocks for each tool call
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        content_blocks.push(ContentBlock::ToolUse {
                            id: &tool_call.id,
                            name: &tool_call.name,
                            input: &tool_call.arguments,
                        });
                    }
                }

                // CRITICAL: Anthropic requires content to be non-empty
                if content_blocks.is_empty() {
                    return Err(serde::ser::Error::custom(format!(
                        "Assistant message {} has tool_calls but generated no content blocks! tool_calls: {:?}, content: {:?}",
                        idx, message.tool_calls, message.content
                    )));
                }

                // Serialize the struct directly (no serde_json::to_value!)
                // This is passed directly to the outer serializer
                let assistant_msg = AssistantMessage {
                    role: "assistant",
                    content: content_blocks,
                };

                // Convert to Value only for logging and final serialization
                serde_json::to_value(&assistant_msg).map_err(serde::ser::Error::custom)?
            }
            "assistant" => {
                // Assistant message without tool calls
                // DEFENSIVE: This should not happen due to upstream checks, but handle it gracefully
                if message.content.is_empty() {
                    tracing::error!(
                        "❌ Assistant message {} has EMPTY content and NO tool_calls! \
                         This indicates a bug in message creation. \
                         Anthropic requires non-empty content for all messages except final assistant.",
                        idx
                    );
                    return Err(serde::ser::Error::custom(format!(
                        "Assistant message {} has empty content and no tool_calls - \
                         this should have been prevented during message creation",
                        idx
                    )));
                }

                // Normal assistant message with text content
                serde_json::json!({
                    "role": "assistant",
                    "content": message.content
                })
            }
            _ => {
                // User, system, or other messages - simple string content
                // CRITICAL: Anthropic requires non-empty content
                if message.content.is_empty() {
                    tracing::error!("❌ Message {} (role: {}) has EMPTY content!", idx, message.role);
                    return Err(serde::ser::Error::custom(format!(
                        "Message {} (role: {}) has empty content - Anthropic requires non-empty content",
                        idx, message.role
                    )));
                }

                serde_json::json!({
                    "role": message.role,
                    "content": message.content
                })
            }
        };

        // DEBUG: Log serialized message to diagnose empty content errors
        if idx <= 5 {  // Only log first 6 messages to avoid spam
            tracing::debug!("  Serialized message[{}]: {}", idx, serde_json::to_string_pretty(&anthropic_msg).unwrap_or_else(|_| "<failed to serialize>".to_string()));
        }

        seq.serialize_element(&anthropic_msg)?;
    }

    seq.end()
}

/// Web search plugin configuration (OpenRouter feature)
/// OpenRouter uses a plugins array format: [{"id": "web", "max_results": 5}]
#[derive(Debug, Serialize)]
struct WebPlugin {
    id: String,  // Must be "web" for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,  // default is 5
}

#[derive(Debug, Serialize)]
struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_fallbacks: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct ToolCallDelta {
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

/// Internal representation of a message from OpenRouter API
/// This differs from our Message type because OpenRouter uses OpenAI's format
#[derive(Debug, Deserialize)]
struct ApiMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
}

/// OpenRouter/OpenAI format for tool calls
#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiFunctionCall,
}

#[derive(Debug, Deserialize)]
struct ApiFunctionCall {
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_tool_execution_sequence() {
        // Test the exact sequence from the bug report:
        // 1. User message
        // 2. Assistant message with tool_use blocks
        // 3. User message with tool_result block (referencing the tool_use_id)

        let mut messages = Vec::new();

        // Message 1: User asks for something requiring a tool
        messages.push(Message::new("user", "What is 2+2?"));

        // Message 2: Assistant responds with tool call
        let tool_id = "toolu_vrtx_011f4dtw2pHcnc92YobnXXeN";
        messages.push(Message::with_tool_calls(
            "Let me calculate that for you.".to_string(),
            vec![ToolCall {
                id: tool_id.to_string(),
                name: "calculator".to_string(),
                arguments: serde_json::json!({"expression": "2+2"}),
            }],
        ));

        // Message 3: Tool result (should have role "user" with tool_result block)
        messages.push(Message::tool_result(
            tool_id.to_string(),
            "4".to_string(),
        ));

        // Serialize messages to Anthropic format
        let messages_json = serialize_messages_for_anthropic_value(&messages).unwrap();

        // Serialize to JSON
        let api_request = ApiRequest {
            model: "anthropic/claude-sonnet-4.5".to_string(),
            messages: messages_json,
            stream: false,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            plugins: None,
            provider: None,
        };

        let json = serde_json::to_value(&api_request).unwrap();
        let msgs = json["messages"].as_array().unwrap();

        // Verify the sequence matches Anthropic's expectations
        assert_eq!(msgs.len(), 3);

        // Message 0: User
        assert_eq!(msgs[0]["role"], "user");

        // Message 1: Assistant with tool_use block
        assert_eq!(msgs[1]["role"], "assistant");
        let content = msgs[1]["content"].as_array().unwrap();
        let tool_use_block = content.iter()
            .find(|b| b["type"] == "tool_use")
            .expect("Should have tool_use block");
        assert_eq!(tool_use_block["id"], tool_id);

        // Message 2: User with tool_result block (NOT role "tool")
        assert_eq!(msgs[2]["role"], "user", "Tool results must have role 'user' for Anthropic");
        let result_content = msgs[2]["content"].as_array().unwrap();
        assert_eq!(result_content[0]["type"], "tool_result");
        assert_eq!(result_content[0]["tool_use_id"], tool_id, "tool_use_id must match the id from tool_use block");

        // CRITICAL: Verify that content is not empty!
        assert_eq!(result_content[0]["content"], "4", "Tool result content must not be empty!");
    }

    #[test]
    fn test_serialize_messages_for_anthropic_format() {
        // Test that our custom serializer produces Anthropic-compatible format
        let mut messages = Vec::new();

        // User message
        messages.push(Message::new("user", "What's the weather?"));

        // Assistant message with tool call
        messages.push(Message::with_tool_calls(
            "I'll check the weather for you.".to_string(),
            vec![ToolCall {
                id: "toolu_vrtx_011f4dtw2pHcnc92YobnXXeN".to_string(),
                name: "get_weather".to_string(),
                arguments: serde_json::json!({"location": "NYC"}),
            }],
        ));

        // Tool result
        messages.push(Message::tool_result(
            "toolu_vrtx_011f4dtw2pHcnc92YobnXXeN".to_string(),
            "Weather in NYC: 72°F, sunny".to_string(),
        ));

        // Serialize messages to Anthropic format
        let messages_json = serialize_messages_for_anthropic_value(&messages).unwrap();

        // Create API request
        let api_request = ApiRequest {
            model: "anthropic/claude-sonnet-4.5".to_string(),
            messages: messages_json,
            stream: false,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            plugins: None,
            provider: None,
        };

        // Serialize to JSON
        let json = serde_json::to_value(&api_request).unwrap();
        let messages_json = &json["messages"];

        // Verify structure
        assert!(messages_json.is_array());
        let msgs = messages_json.as_array().unwrap();
        assert_eq!(msgs.len(), 3);

        // Check user message (simple format)
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "What's the weather?");

        // Check assistant message with tool_use blocks
        assert_eq!(msgs[1]["role"], "assistant");
        let assistant_content = msgs[1]["content"].as_array().unwrap();
        assert_eq!(assistant_content.len(), 2); // text block + tool_use block

        // Text block
        assert_eq!(assistant_content[0]["type"], "text");
        assert_eq!(assistant_content[0]["text"], "I'll check the weather for you.");

        // Tool use block
        assert_eq!(assistant_content[1]["type"], "tool_use");
        assert_eq!(assistant_content[1]["id"], "toolu_vrtx_011f4dtw2pHcnc92YobnXXeN");
        assert_eq!(assistant_content[1]["name"], "get_weather");
        assert_eq!(assistant_content[1]["input"]["location"], "NYC");

        // Check tool result message (user role with tool_result block)
        assert_eq!(msgs[2]["role"], "user");
        let tool_result_content = msgs[2]["content"].as_array().unwrap();
        assert_eq!(tool_result_content.len(), 1);

        // Tool result block
        assert_eq!(tool_result_content[0]["type"], "tool_result");
        assert_eq!(tool_result_content[0]["tool_use_id"], "toolu_vrtx_011f4dtw2pHcnc92YobnXXeN");
        assert_eq!(tool_result_content[0]["content"], "Weather in NYC: 72°F, sunny");
    }

    #[test]
    fn test_deserialize_openrouter_response_with_tools() {
        // Simulated OpenRouter response with tool calls
        let response_json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"NYC\",\"units\":\"metric\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;

        // Should deserialize without error
        let result: Result<CompletionResponse, _> = serde_json::from_str(response_json);
        assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

        let completion = result.unwrap();
        assert_eq!(completion.choices.len(), 1);

        let choice = &completion.choices[0];
        assert!(choice.message.tool_calls.is_some());

        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);

        let tool_call = &tool_calls[0];
        assert_eq!(tool_call.id, "call_abc123");
        assert_eq!(tool_call.function.name, "get_weather");

        // Verify arguments can be parsed as JSON
        let args: Result<serde_json::Value, _> =
            serde_json::from_str(&tool_call.function.arguments);
        assert!(args.is_ok());
    }

    #[test]
    fn test_deserialize_openrouter_response_without_tools() {
        // Normal response without tool calls
        let response_json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }]
        }"#;

        let result: Result<CompletionResponse, _> = serde_json::from_str(response_json);
        assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

        let completion = result.unwrap();
        let choice = &completion.choices[0];
        assert_eq!(
            choice.message.content,
            Some("Hello! How can I help you today?".to_string())
        );
        assert!(choice.message.tool_calls.is_none());
    }

    #[test]
    fn test_convert_api_tool_calls_to_internal_format() {
        // Test the conversion from OpenRouter's format to our internal ToolCall format
        let response_json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "search_code",
                            "arguments": "{\"query\":\"rust async\",\"file_type\":\"rs\"}"
                        }
                    },
                    {
                        "id": "call_456",
                        "type": "function",
                        "function": {
                            "name": "get_status",
                            "arguments": "{}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;

        let completion: CompletionResponse = serde_json::from_str(response_json).unwrap();
        let choice = &completion.choices[0];

        // Convert to internal format (same logic as complete_chat)
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .filter_map(|api_call| {
                    match serde_json::from_str(&api_call.function.arguments) {
                        Ok(args) => Some(ToolCall {
                            id: api_call.id.clone(),
                            name: api_call.function.name.clone(),
                            arguments: args,
                        }),
                        Err(_) => None,
                    }
                })
                .collect::<Vec<_>>()
        });

        assert!(tool_calls.is_some());
        let calls = tool_calls.unwrap();
        assert_eq!(calls.len(), 2);

        // Verify first tool call
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].name, "search_code");
        assert_eq!(calls[0].arguments["query"], "rust async");
        assert_eq!(calls[0].arguments["file_type"], "rs");

        // Verify second tool call
        assert_eq!(calls[1].id, "call_456");
        assert_eq!(calls[1].name, "get_status");
        assert!(calls[1].arguments.is_object());
    }

    #[test]
    fn test_empty_tool_result_is_rejected() {
        // This test verifies that empty tool results are caught during serialization
        let mut messages = Vec::new();

        messages.push(Message::new("user", "Test"));
        messages.push(Message::with_tool_calls(
            "Using tool".to_string(),
            vec![ToolCall {
                id: "tool_123".to_string(),
                name: "test_tool".to_string(),
                arguments: serde_json::json!({}),
            }],
        ));

        // Tool result with EMPTY content - this should be rejected!
        messages.push(Message::tool_result(
            "tool_123".to_string(),
            String::new(),  // ← Empty content!
        ));

        // Serialization should fail when converting to Anthropic format
        let result = serialize_messages_for_anthropic_value(&messages);

        // Create API request (this will only be reached if serialization succeeds)
        let api_request = if let Ok(messages_json) = result {
            ApiRequest {
                model: "anthropic/claude-sonnet-4.5".to_string(),
                messages: messages_json,
                stream: false,
                temperature: None,
                max_tokens: None,
                tools: None,
                tool_choice: None,
                plugins: None,
                provider: None,
            }
        } else {
            // If serialization failed (as expected), test passes
            assert!(result.is_err(), "Empty tool result should be rejected during serialization");
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.contains("empty content"),
                "Error should mention empty content, got: {}", error_msg);
            return;
        };

        // If we got here, serialization didn't fail as expected - final JSON check
        let result = serde_json::to_value(&api_request);
        assert!(result.is_err(), "Empty tool result should be rejected");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("empty content"),
            "Error should mention empty content, got: {}", error_msg);
    }

    #[test]
    fn test_empty_regular_message_is_rejected() {
        // This test verifies that empty regular messages are caught
        let mut messages = Vec::new();

        messages.push(Message::new("user", ""));  // ← Empty user message!

        // Serialization should fail when converting to Anthropic format
        let result = serialize_messages_for_anthropic_value(&messages);

        // Create API request (this will only be reached if serialization succeeds)
        let api_request = if let Ok(messages_json) = result {
            ApiRequest {
                model: "anthropic/claude-sonnet-4.5".to_string(),
                messages: messages_json,
                stream: false,
                temperature: None,
                max_tokens: None,
                tools: None,
                tool_choice: None,
                plugins: None,
                provider: None,
            }
        } else {
            // If serialization failed (as expected), test passes
            assert!(result.is_err(), "Empty message should be rejected during serialization");
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.contains("empty content"),
                "Error should mention empty content, got: {}", error_msg);
            return;
        };

        // If we got here, serialization didn't fail as expected - final JSON check
        let result = serde_json::to_value(&api_request);
        assert!(result.is_err(), "Empty message should be rejected");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("empty content"),
            "Error should mention empty content, got: {}", error_msg);
    }

    #[test]
    fn test_handle_invalid_tool_arguments() {
        // Test that invalid JSON in arguments doesn't crash, just filters out
        let response_json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_good",
                        "type": "function",
                        "function": {
                            "name": "valid_tool",
                            "arguments": "{\"param\":\"value\"}"
                        }
                    },
                    {
                        "id": "call_bad",
                        "type": "function",
                        "function": {
                            "name": "invalid_tool",
                            "arguments": "not valid json at all"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;

        let completion: CompletionResponse = serde_json::from_str(response_json).unwrap();
        let choice = &completion.choices[0];

        // Convert with error handling
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .filter_map(|api_call| {
                    match serde_json::from_str(&api_call.function.arguments) {
                        Ok(args) => Some(ToolCall {
                            id: api_call.id.clone(),
                            name: api_call.function.name.clone(),
                            arguments: args,
                        }),
                        Err(_) => None, // Filter out invalid arguments
                    }
                })
                .collect::<Vec<_>>()
        });

        // Should only have the valid tool call
        assert!(tool_calls.is_some());
        let calls = tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_good");
        assert_eq!(calls[0].name, "valid_tool");
    }
}
