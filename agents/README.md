# Rustbot Agent Configuration

This directory contains JSON-based agent configuration files for Rustbot's multi-agent system.

## Directory Structure

```
agents/
├── schema/
│   └── agent.schema.json      # JSON schema definition
├── presets/                   # Built-in agent templates
│   ├── assistant.json
│   └── web_search.json
└── custom/                    # User-defined agents (create your own!)
```

## Quick Start

### Loading Agents from JSON

```rust
use rustbot::{AgentLoader, AgentConfig};

// Create loader with default search paths
let loader = AgentLoader::new();

// Load all agents from presets/ and custom/
let agents: Vec<AgentConfig> = loader.load_all()?;

// Load specific agent
let agent = loader.load_agent(Path::new("agents/presets/assistant.json"))?;
```

### Creating Custom Agents

1. Create a JSON file in `agents/custom/`
2. Follow the schema defined in `agents/schema/agent.schema.json`
3. Restart Rustbot or reload agents

**Example: `agents/custom/code_reviewer.json`**

```json
{
  "version": "1.0",
  "name": "code_reviewer",
  "description": "Specialized code review assistant",
  "provider": "openrouter",
  "model": "anthropic/claude-3.5-sonnet",
  "apiKey": "${OPENROUTER_API_KEY}",
  "instruction": "You are an expert code reviewer. Analyze code for bugs, performance issues, security vulnerabilities, and best practices. Provide constructive feedback with specific suggestions.",
  "personality": "Professional, thorough, and constructive. Focus on teaching and improvement.",
  "parameters": {
    "temperature": 0.5,
    "maxTokens": 4096
  },
  "capabilities": {
    "webSearch": false,
    "streaming": true
  },
  "enabled": true,
  "metadata": {
    "author": "Your Name",
    "version": "1.0.0",
    "tags": ["code", "review", "development"]
  }
}
```

## Configuration Schema

### Required Fields

- **name**: Unique identifier for the agent (alphanumeric, underscores)
- **provider**: LLM provider (`openrouter`, `openai`, `anthropic`, `ollama`)
- **model**: Model identifier (provider-specific format)
- **instruction**: System prompt defining agent behavior

### Optional Fields

- **version**: Schema version (default: "1.0")
- **description**: Human-readable agent purpose
- **apiKey**: API key or environment variable reference (e.g., `${API_KEY}`)
- **apiBase**: Custom API endpoint URL
- **personality**: Agent tone and communication style
- **parameters**: Model generation parameters
  - **temperature**: 0.0-2.0 (default: 0.7)
  - **maxTokens**: Maximum response length
  - **topP**: Nucleus sampling parameter (0.0-1.0)
- **capabilities**: Feature flags
  - **webSearch**: Enable web search (default: false)
  - **imageInput**: Enable image input (default: false)
  - **streaming**: Enable streaming responses (default: true)
- **enabled**: Whether agent is active (default: true)
- **metadata**: Optional documentation
  - **author**: Creator name
  - **created**: Creation date
  - **version**: Agent version
  - **tags**: Searchable keywords

## Supported Providers

### OpenRouter

```json
{
  "provider": "openrouter",
  "model": "anthropic/claude-3.5-sonnet",
  "apiKey": "${OPENROUTER_API_KEY}"
}
```

**Models**: See [OpenRouter documentation](https://openrouter.ai/docs#models)

### OpenAI

```json
{
  "provider": "openai",
  "model": "gpt-4-turbo-preview",
  "apiKey": "${OPENAI_API_KEY}"
}
```

**Models**: `gpt-4-turbo-preview`, `gpt-4`, `gpt-3.5-turbo`, etc.

### Anthropic

```json
{
  "provider": "anthropic",
  "model": "claude-3-opus-20240229",
  "apiKey": "${ANTHROPIC_API_KEY}"
}
```

**Models**: `claude-3-opus-20240229`, `claude-3-sonnet-20240229`, etc.

### Ollama (Local)

```json
{
  "provider": "ollama",
  "model": "llama2",
  "apiBase": "http://localhost:11434"
}
```

**No API key required**. Ollama must be running locally.

**Models**: Any model installed in Ollama (`llama2`, `mistral`, `codellama`, etc.)

## Environment Variables

### Variable Substitution

Use `${VAR_NAME}` syntax in JSON to reference environment variables:

```json
{
  "apiKey": "${OPENROUTER_API_KEY}"
}
```

### Default Values

Provide fallback values with `:-` syntax:

```json
{
  "apiBase": "${API_BASE_URL:-https://api.openrouter.ai/v1}"
}
```

### Setting Environment Variables

**macOS/Linux:**
```bash
export OPENROUTER_API_KEY="your-key-here"
export ANTHROPIC_API_KEY="your-key-here"
```

**Windows:**
```cmd
set OPENROUTER_API_KEY=your-key-here
set ANTHROPIC_API_KEY=your-key-here
```

**`.env` file:**
```
OPENROUTER_API_KEY=your-key-here
ANTHROPIC_API_KEY=your-key-here
```

## Agent Design Patterns

### General Purpose Assistant

Use for: Conversational AI, general Q&A, multi-domain tasks

```json
{
  "name": "assistant",
  "instruction": "You are a helpful AI assistant...",
  "personality": "Friendly, professional, and helpful",
  "parameters": {
    "temperature": 0.7
  }
}
```

### Web Search Specialist

Use for: Real-time information, current events, fact-checking

```json
{
  "name": "web_search",
  "model": "anthropic/claude-3.5-haiku",  // Lightweight model
  "instruction": "You are a web search specialist...",
  "parameters": {
    "temperature": 0.3,  // More deterministic
    "maxTokens": 2048     // Shorter responses
  },
  "capabilities": {
    "webSearch": true
  }
}
```

### Code Assistant

Use for: Code generation, debugging, refactoring

```json
{
  "name": "code_assistant",
  "instruction": "You are an expert programmer...",
  "parameters": {
    "temperature": 0.2,  // Very deterministic for code
    "maxTokens": 4096
  }
}
```

### Research Assistant

Use for: In-depth analysis, academic research, fact synthesis

```json
{
  "name": "researcher",
  "model": "anthropic/claude-3-opus-20240229",  // Most capable model
  "instruction": "You are a research assistant specializing in thorough analysis...",
  "parameters": {
    "temperature": 0.5,
    "maxTokens": 8192  // Long-form responses
  },
  "capabilities": {
    "webSearch": true
  }
}
```

## Best Practices

### Instruction Writing

1. **Be Specific**: Define exact behavior, not vague guidelines
2. **Provide Examples**: Show desired output format
3. **Set Boundaries**: Explicitly state what agent should/shouldn't do
4. **Use Sections**: Organize instructions with markdown headers

### Model Selection

- **Sonnet**: Balanced cost/performance for most tasks
- **Haiku**: Fast, cost-effective for simple tasks
- **Opus**: Maximum capability for complex reasoning

### Temperature Tuning

- **0.0-0.3**: Code, math, deterministic tasks
- **0.4-0.7**: General conversation, balanced creativity
- **0.8-1.0**: Creative writing, brainstorming
- **1.0+**: Experimental, highly creative

### Token Limits

- **2048**: Quick responses, simple queries
- **4096**: Standard conversations
- **8192+**: Long-form content, detailed analysis

## Troubleshooting

### Agent Not Loading

**Problem**: Agent doesn't appear in Rustbot

**Solutions**:
1. Check JSON syntax (use `jsonlint` or VS Code)
2. Verify required fields are present
3. Check file extension is `.json`
4. Review logs for parsing errors

### API Key Errors

**Problem**: "API key required" or authentication failures

**Solutions**:
1. Set environment variable: `export PROVIDER_API_KEY=your-key`
2. Verify variable name matches provider's default
3. Use explicit `apiKey` field in JSON
4. Check API key validity with provider

### Model Not Found

**Problem**: "Model not available" errors

**Solutions**:
1. Verify model name with provider documentation
2. Check provider supports the model
3. For Ollama: Ensure model is pulled (`ollama pull llama2`)

### Performance Issues

**Problem**: Slow responses or timeouts

**Solutions**:
1. Reduce `maxTokens` for faster responses
2. Use lighter model (Haiku instead of Opus)
3. Lower temperature for more deterministic output
4. For Ollama: Check local resources (CPU/RAM)

## Validation

Validate your agent configuration against the schema:

```bash
# Using jsonschema (Python)
pip install jsonschema
jsonschema -i agents/custom/my_agent.json agents/schema/agent.schema.json

# Using ajv-cli (Node.js)
npm install -g ajv-cli
ajv validate -s agents/schema/agent.schema.json -d agents/custom/my_agent.json
```

## Migration from Hardcoded Agents

If you have hardcoded agent configurations in Rust code:

**Before:**
```rust
let config = AgentConfig {
    id: "assistant".to_string(),
    name: "Assistant".to_string(),
    instructions: "You are a helpful AI assistant...".to_string(),
    personality: Some("Friendly".to_string()),
    model: "anthropic/claude-3.5-sonnet".to_string(),
    enabled: true,
    web_search_enabled: false,
};
```

**After:**
1. Create `agents/custom/assistant.json` with equivalent configuration
2. Load via `AgentLoader`
3. Remove hardcoded config

This enables runtime configuration without recompilation.

## Contributing

To add built-in agent templates:

1. Create JSON in `agents/presets/`
2. Follow naming convention: `{purpose}.json`
3. Document use case in this README
4. Test with `cargo test`
5. Submit pull request

## License

Agent configurations are MIT licensed like the rest of Rustbot.
