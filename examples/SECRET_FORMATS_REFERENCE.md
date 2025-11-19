# Secret Format Reference Guide

Quick reference for all supported secret resolution formats in Rustbot.

## Agent Configuration (`agents/presets/*.json`)

### 1Password Secret Reference
```json
{
  "apiKey": "op://Private/Rustbot/openrouter_api_key"
}
```
✅ **Use for**: Sensitive secrets stored in 1Password
⚙️ **Requires**: `op` CLI installed and signed in
🔒 **Security**: Secrets not in git, managed by 1Password

### Environment Variable (Required)
```json
{
  "apiKey": "${OPENROUTER_API_KEY}"
}
```
✅ **Use for**: Secrets from environment or .env file
⚠️ **Errors**: If variable not set
📝 **Set in**: `.env.local` or shell environment

### Environment Variable with Default
```json
{
  "model": "${MODEL:-anthropic/claude-sonnet-4}"
}
```
✅ **Use for**: Optional configuration with sensible defaults
💡 **Fallback**: Uses default if variable not set or empty
🎯 **Best for**: Non-sensitive configuration

### Plain Value
```json
{
  "name": "assistant",
  "temperature": 0.7
}
```
✅ **Use for**: Non-sensitive, static values
📊 **Best for**: Names, numbers, boolean flags

## MCP Configuration (`.mcp.json`)

### Environment Variables in Commands
```json
{
  "local_servers": {
    "my-service": {
      "command": "npx",
      "args": ["-y", "my-mcp-server"],
      "env": {
        "API_KEY": "op://Private/Services/my_service_key",
        "HOST": "${MCP_HOST:-localhost}",
        "PORT": "${MCP_PORT}"
      }
    }
  }
}
```

### Cloud Service Authentication
```json
{
  "cloud_services": {
    "my-api": {
      "url": "https://api.example.com",
      "auth": {
        "type": "bearer",
        "token": "op://Private/API Services/bearer_token"
      }
    }
  }
}
```

## Format Comparison

| Format | Syntax | Security | Flexibility | Best For |
|--------|--------|----------|-------------|----------|
| 1Password | `op://vault/item/field` | 🔒🔒🔒 High | Medium | Production secrets |
| Env Var Required | `${VAR}` | 🔒🔒 Medium | High | Dev environments |
| Env Var Default | `${VAR:-default}` | 🔒 Low-Medium | High | Optional config |
| Plain Value | `"value"` | ❌ None | Low | Non-sensitive data |

## Setup Checklist

### For 1Password Integration
- [ ] Install 1Password CLI: `brew install 1password-cli`
- [ ] Sign in: `op signin`
- [ ] Create vault and items in 1Password app
- [ ] Test reference: `op read "op://Private/Rustbot/api_key"`
- [ ] Update config with `op://` reference
- [ ] Restart Rustbot

### For Environment Variables
- [ ] Create `.env.local` file (not in git)
- [ ] Add variable: `OPENROUTER_API_KEY=sk-xxx`
- [ ] Load environment: `source .env.local` (or let app load it)
- [ ] Update config with `${VAR}` reference
- [ ] Restart Rustbot

## Common Patterns

### Hybrid Approach (Recommended)
```json
{
  "apiKey": "op://Private/Rustbot/openrouter_api_key",
  "model": "${MODEL:-anthropic/claude-sonnet-4}",
  "temperature": 0.7,
  "name": "assistant"
}
```
🎯 Secure secrets + Flexible config + Static values

### Development Environment
```json
{
  "apiKey": "${OPENROUTER_API_KEY}",
  "model": "${MODEL:-anthropic/claude-sonnet-4}",
  "apiBase": "${API_BASE:-https://openrouter.ai/api/v1}"
}
```
💻 All from environment, easy to override

### Production Environment
```json
{
  "apiKey": "op://Private/Rustbot Production/openrouter_api_key",
  "model": "anthropic/claude-sonnet-4",
  "apiBase": "https://openrouter.ai/api/v1"
}
```
🏢 Secrets in 1Password, config locked down

## Resolution Order

Rustbot resolves secrets in this order:

1. **Check prefix**: `op://` → 1Password
2. **Check prefix**: `${` → Environment variable
3. **No prefix**: Plain value

```
resolve_env_var("op://Private/Rustbot/key")
  → Execute: op read "op://Private/Rustbot/key"
  → Return: "sk-or-v1-xxx"

resolve_env_var("${API_KEY}")
  → Check: std::env::var("API_KEY")
  → Return: "sk-or-v1-xxx"

resolve_env_var("${MODEL:-claude-4}")
  → Check: std::env::var("MODEL")
  → Not set → Return: "claude-4"

resolve_env_var("assistant")
  → Return: "assistant"
```

## Error Messages

### 1Password Errors
```
Failed to execute 1Password CLI. Is it installed?
Install: brew install 1password-cli
Reference: op://Private/Rustbot/api_key
```
**Fix**: `brew install 1password-cli`

```
Not signed in to 1Password. Run: op signin
Reference: op://Private/Rustbot/api_key
```
**Fix**: `op signin`

```
1Password secret not found: op://Private/Rustbot/wrong_key
Error: "wrong_key" isn't an item in the "Rustbot" vault
```
**Fix**: Check vault, item, and field names (case-sensitive)

### Environment Variable Errors
```
Environment variable 'OPENROUTER_API_KEY' not found
```
**Fix**: Set in `.env.local` or environment

## Quick Commands

```bash
# Test 1Password reference
op read "op://Private/Rustbot/api_key"

# List all 1Password items
op item list --vault="Private"

# Get item details
op item get "Rustbot" --format=json

# Test environment variable
echo ${OPENROUTER_API_KEY}

# Check .env file
cat .env.local

# Validate JSON config
jq empty agents/presets/my-agent.json
```

## Security Best Practices

1. ✅ **DO**: Use 1Password for production secrets
2. ✅ **DO**: Use environment variables for development
3. ✅ **DO**: Use defaults for non-sensitive config
4. ✅ **DO**: Add `.env.local` to `.gitignore`
5. ❌ **DON'T**: Commit secrets to git
6. ❌ **DON'T**: Use plain values for API keys
7. ❌ **DON'T**: Share secrets in chat or email
8. ❌ **DON'T**: Store secrets in documentation

## Next Steps

1. See full documentation: `docs/1PASSWORD_INTEGRATION.md`
2. Review examples: `examples/agent-with-1password.json`
3. Test configuration: `cargo run`
4. Troubleshoot: Check error messages for guidance
