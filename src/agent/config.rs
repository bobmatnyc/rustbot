use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::llm::LlmProvider;
use std::path::Path;

/// JSON-based agent configuration
///
/// Design Decision: JSON-based configuration for agent definitions
///
/// Rationale: Selected JSON for agent configs to enable:
/// 1. Easy modification without recompilation
/// 2. User-customizable agent behaviors
/// 3. Version control for agent templates
/// 4. Environment variable interpolation for secure API key management
///
/// Trade-offs:
/// - Flexibility: Runtime configuration vs. compile-time validation
/// - Maintainability: Separate config files vs. all-in-code definitions
/// - Security: Environment variable references vs. hardcoded keys
///
/// Extension Points: Schema validation can be added via jsonschema crate
/// if strict validation becomes necessary (>10 agent templates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonAgentConfig {
    /// Schema version for future compatibility
    #[serde(default = "default_version")]
    pub version: String,

    /// Unique agent identifier (used as agent ID)
    pub name: String,

    /// Human-readable description of agent purpose
    #[serde(default)]
    pub description: String,

    /// LLM provider to use
    pub provider: LlmProvider,

    /// Model identifier (provider-specific format)
    pub model: String,

    /// API key (supports ${ENV_VAR} syntax)
    #[serde(rename = "apiKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Custom API endpoint URL
    #[serde(rename = "apiBase")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,

    /// System prompt defining agent behavior
    pub instruction: String,

    /// Optional personality/tone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,

    /// Model parameters (temperature, max_tokens, etc.)
    #[serde(default)]
    pub parameters: ModelParameters,

    /// Agent capabilities (web search, image input, etc.)
    #[serde(default)]
    pub capabilities: AgentCapabilities,

    /// Whether this agent is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Whether this is the primary agent (handles user messages directly)
    #[serde(rename = "isPrimary")]
    #[serde(default)]
    pub is_primary: bool,

    /// Optional metadata (author, tags, version, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AgentMetadata>,

    /// MCP extensions enabled for this agent
    ///
    /// List of extension IDs (e.g., "ai.exa/exa") that should be loaded
    /// and made available as tools for this agent.
    ///
    /// Extensions are agent-specific, not global. Different agents can
    /// have different toolsets based on their purpose.
    #[serde(rename = "mcpExtensions")]
    #[serde(default)]
    pub mcp_extensions: Vec<String>,
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_enabled() -> bool {
    true
}

/// Model generation parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelParameters {
    /// Sampling temperature (0.0 = deterministic, 2.0 = very random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(rename = "maxTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p nucleus sampling parameter
    #[serde(rename = "topP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Agent capability flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Web search capability enabled
    #[serde(rename = "webSearch")]
    #[serde(default)]
    pub web_search: bool,

    /// Image input capability enabled
    #[serde(rename = "imageInput")]
    #[serde(default)]
    pub image_input: bool,

    /// Streaming response capability
    #[serde(default = "default_streaming")]
    pub streaming: bool,
}

fn default_streaming() -> bool {
    true
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            web_search: false,
            image_input: false,
            streaming: true,
        }
    }
}

/// Agent metadata for documentation and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent author/creator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Creation/modification date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Agent version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Searchable tags
    #[serde(default)]
    pub tags: Vec<String>,
}

impl JsonAgentConfig {
    /// Load agent configuration from JSON file
    ///
    /// # Errors
    /// - File I/O errors (file not found, permission denied)
    /// - JSON parsing errors (invalid syntax, missing required fields)
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read agent config from {:?}", path))?;

        Self::from_json(&content)
            .with_context(|| format!("Failed to parse agent config from {:?}", path))
    }

    /// Parse agent configuration from JSON string
    ///
    /// # Errors
    /// - JSON parsing errors (invalid syntax, type mismatches)
    /// - Missing required fields (name, provider, model, instruction)
    pub fn from_json(json: &str) -> Result<Self> {
        let config: JsonAgentConfig = serde_json::from_str(json)
            .context("Failed to deserialize agent configuration")?;

        Ok(config)
    }

    /// Resolve environment variables in configuration strings
    ///
    /// Supports two syntaxes:
    /// - `${VAR_NAME}` - Required variable (error if not found)
    /// - `${VAR_NAME:-default}` - Optional variable with fallback
    ///
    /// # Errors
    /// - Environment variable not found (for required variables)
    pub fn resolve_env_vars(&mut self) -> Result<()> {
        if let Some(api_key) = &self.api_key {
            self.api_key = Some(resolve_env_var(api_key)?);
        }
        Ok(())
    }

    /// Get resolved API key from config or environment
    ///
    /// Resolution order:
    /// 1. Explicit apiKey in config (after env var resolution)
    /// 2. Provider's default environment variable
    /// 3. None (if provider doesn't require API key)
    ///
    /// # Errors
    /// - Environment variable syntax errors
    pub fn get_api_key(&self) -> Result<Option<String>> {
        // First check explicit API key in config
        if let Some(key) = &self.api_key {
            return Ok(Some(resolve_env_var(key)?));
        }

        // Fallback to provider's default environment variable
        let env_var = self.provider.default_env_var();
        if !env_var.is_empty() {
            if let Ok(key) = std::env::var(env_var) {
                return Ok(Some(key));
            }
        }

        // Some providers (Ollama) don't require API keys
        Ok(None)
    }

    /// Get resolved API base URL
    ///
    /// Returns custom apiBase if specified, otherwise provider's default
    pub fn get_api_base(&self) -> String {
        self.api_base
            .clone()
            .unwrap_or_else(|| self.provider.default_api_base().to_string())
    }

    /// Validate configuration for common issues
    ///
    /// Checks:
    /// - API key present when required by provider
    /// - Temperature within valid range
    /// - Model identifier not empty
    ///
    /// # Errors
    /// - Missing required API key
    /// - Invalid parameter values
    pub fn validate(&self) -> Result<()> {
        // Check API key requirement
        if self.provider.requires_api_key() {
            let api_key = self.get_api_key()?;
            if api_key.is_none() || api_key.as_ref().unwrap().is_empty() {
                anyhow::bail!(
                    "Provider '{}' requires an API key. Set {} or specify apiKey in config.",
                    format!("{:?}", self.provider).to_lowercase(),
                    self.provider.default_env_var()
                );
            }
        }

        // Validate temperature range
        if let Some(temp) = self.parameters.temperature {
            if !(0.0..=2.0).contains(&temp) {
                anyhow::bail!("Temperature must be between 0.0 and 2.0, got {}", temp);
            }
        }

        // Validate top_p range
        if let Some(top_p) = self.parameters.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                anyhow::bail!("top_p must be between 0.0 and 1.0, got {}", top_p);
            }
        }

        // Ensure model is specified
        if self.model.is_empty() {
            anyhow::bail!("Model identifier cannot be empty");
        }

        Ok(())
    }
}

/// Resolve environment variable syntax in a string
///
/// Supported formats:
/// - `${VAR_NAME}` - Required variable (error if not found)
/// - `${VAR_NAME:-default_value}` - Optional with default
/// - Plain strings are returned unchanged
///
/// # Errors
/// - Environment variable not found (for required variables)
fn resolve_env_var(value: &str) -> Result<String> {
    // Not an environment variable reference
    if !value.starts_with("${") || !value.ends_with('}') {
        return Ok(value.to_string());
    }

    // Extract variable expression
    let var_expr = &value[2..value.len() - 1];

    // Check for default value syntax: ${VAR:-default}
    if let Some(pos) = var_expr.find(":-") {
        let var_name = &var_expr[..pos];
        let default_value = &var_expr[pos + 2..];

        // Return env var if set, otherwise use default
        match std::env::var(var_name) {
            Ok(val) if !val.is_empty() => Ok(val),
            _ => Ok(default_value.to_string()),
        }
    } else {
        // No default, variable is required
        std::env::var(var_expr)
            .with_context(|| format!("Environment variable '{}' not found", var_expr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let json = r#"{
            "name": "test",
            "provider": "openrouter",
            "model": "anthropic/claude-3.5-sonnet",
            "instruction": "Test instruction"
        }"#;

        let config = JsonAgentConfig::from_json(json).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.provider, LlmProvider::OpenRouter);
        assert_eq!(config.model, "anthropic/claude-3.5-sonnet");
        assert_eq!(config.version, "1.0");
        assert!(config.enabled);
    }

    #[test]
    fn test_parse_full_config() {
        let json = r#"{
            "version": "1.0",
            "name": "assistant",
            "description": "Test assistant",
            "provider": "anthropic",
            "model": "claude-3-opus-20240229",
            "apiKey": "${ANTHROPIC_API_KEY}",
            "instruction": "Be helpful",
            "personality": "Friendly",
            "parameters": {
                "temperature": 0.7,
                "maxTokens": 2048
            },
            "capabilities": {
                "webSearch": true,
                "streaming": true
            },
            "enabled": true,
            "metadata": {
                "author": "Test",
                "tags": ["test", "assistant"]
            }
        }"#;

        let config = JsonAgentConfig::from_json(json).unwrap();
        assert_eq!(config.name, "assistant");
        assert_eq!(config.provider, LlmProvider::Anthropic);
        assert_eq!(config.parameters.temperature, Some(0.7));
        assert_eq!(config.parameters.max_tokens, Some(2048));
        assert!(config.capabilities.web_search);
        assert_eq!(config.metadata.as_ref().unwrap().tags, vec!["test", "assistant"]);
    }

    #[test]
    fn test_resolve_env_var_plain_string() {
        let result = resolve_env_var("plain_string").unwrap();
        assert_eq!(result, "plain_string");
    }

    #[test]
    fn test_resolve_env_var_with_value() {
        std::env::set_var("TEST_VAR", "test_value");

        let result = resolve_env_var("${TEST_VAR}").unwrap();
        assert_eq!(result, "test_value");

        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_resolve_env_var_with_default() {
        let result = resolve_env_var("${MISSING_VAR:-default_value}").unwrap();
        assert_eq!(result, "default_value");
    }

    #[test]
    fn test_resolve_env_var_missing_required() {
        let result = resolve_env_var("${DEFINITELY_MISSING_VAR}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_provider_default_api_base() {
        assert_eq!(
            LlmProvider::OpenRouter.default_api_base(),
            "https://openrouter.ai/api/v1"
        );
        assert_eq!(
            LlmProvider::Ollama.default_api_base(),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_provider_requires_api_key() {
        assert!(LlmProvider::OpenRouter.requires_api_key());
        assert!(LlmProvider::OpenAI.requires_api_key());
        assert!(LlmProvider::Anthropic.requires_api_key());
        assert!(!LlmProvider::Ollama.requires_api_key());
    }

    #[test]
    fn test_validate_missing_api_key() {
        // Clear any existing env vars
        std::env::remove_var("OPENROUTER_API_KEY");

        let config = JsonAgentConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: String::new(),
            provider: LlmProvider::OpenRouter,
            model: "test/model".to_string(),
            api_key: None,
            api_base: None,
            instruction: "test".to_string(),
            personality: None,
            parameters: ModelParameters::default(),
            capabilities: AgentCapabilities::default(),
            enabled: true,
            is_primary: false,
            metadata: None,
            mcp_extensions: Vec::new(),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires an API key"));
    }

    #[test]
    fn test_validate_invalid_temperature() {
        std::env::set_var("OPENROUTER_API_KEY", "test_key");

        let config = JsonAgentConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: String::new(),
            provider: LlmProvider::OpenRouter,
            model: "test/model".to_string(),
            api_key: None,
            api_base: None,
            instruction: "test".to_string(),
            personality: None,
            parameters: ModelParameters {
                temperature: Some(3.0), // Invalid: > 2.0
                max_tokens: None,
                top_p: None,
            },
            capabilities: AgentCapabilities::default(),
            enabled: true,
            is_primary: false,
            metadata: None,
            mcp_extensions: Vec::new(),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Temperature"));

        std::env::remove_var("OPENROUTER_API_KEY");
    }

    #[test]
    fn test_ollama_no_api_key_required() {
        let config = JsonAgentConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: String::new(),
            provider: LlmProvider::Ollama,
            model: "llama2".to_string(),
            api_key: None,
            api_base: None,
            instruction: "test".to_string(),
            personality: None,
            parameters: ModelParameters::default(),
            capabilities: AgentCapabilities::default(),
            enabled: true,
            is_primary: false,
            metadata: None,
            mcp_extensions: Vec::new(),
        };

        // Ollama doesn't require API key, validation should pass
        assert!(config.validate().is_ok());
    }
}
