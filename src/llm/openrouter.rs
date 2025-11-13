use super::types::*;
use super::LlmAdapter;
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";

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
        let api_request = ApiRequest {
            model: request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages: request.messages,
            stream: true,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self.send_request(&api_request).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
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
                                        if let Some(content) = &delta.content {
                                            if tx.send(content.clone()).is_err() {
                                                return Ok(()); // Receiver dropped
                                            }
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
        let api_request = ApiRequest {
            model: request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages: request.messages,
            stream: false,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self.send_request(&api_request).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error {}: {}", status, error_text);
        }

        let completion: CompletionResponse = response.json().await?;

        let choice = completion
            .choices
            .first()
            .context("No choices in response")?;

        Ok(LlmResponse {
            content: choice.message.content.clone(),
            tool_calls: None, // OpenRouter doesn't support tool calls yet
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
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    message: Message,
    finish_reason: Option<String>,
}
