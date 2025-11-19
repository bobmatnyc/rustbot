# 1Password CLI Integration for Rustbot

## Overview

Rustbot now supports pulling secrets from 1Password using the `op://` reference format. This provides a secure way to manage API keys and other sensitive configuration without storing them in plain text files or environment variables.

## Features

- **1Password Secret References**: Use `op://vault/item/field` format in agent and MCP configs
- **Backward Compatibility**: Existing `${VAR}` and `${VAR:-default}` syntax still works
- **Helpful Error Messages**: Clear guidance when `op` CLI is missing or not authenticated
- **Automatic Secret Resolution**: Secrets are fetched transparently at config load time

## Prerequisites

1. **Install 1Password CLI**:
   ```bash
   brew install 1password-cli
   ```

2. **Sign in to 1Password**:
   ```bash
   op signin
   ```

3. **Store your secrets in 1Password** (see "Setting Up Secrets" below)

## Usage

### Main Application API Key (`.env.local`)

**NEW**: The main `OPENROUTER_API_KEY` in `.env.local` now supports 1Password references!

In your `.env.local` file:

```bash
# Instead of plain text:
# OPENROUTER_API_KEY=sk-or-v1-abc123...

# Use 1Password reference:
OPENROUTER_API_KEY=op://Private/Rustbot/api_key
```

This is the recommended approach for securing your primary OpenRouter API key.

**Setup**:
1. Store your OpenRouter API key in 1Password:
   - Vault: `Private` (or your choice)
   - Item: `Rustbot`
   - Field: `api_key`
   - Value: Your `sk-or-v1-...` key

2. Update `.env.local`:
   ```bash
   echo "OPENROUTER_API_KEY=op://Private/Rustbot/api_key" > .env.local
   ```

3. Test:
   ```bash
   # Verify 1Password can read it
   op read "op://Private/Rustbot/api_key"

   # Run Rustbot
   cargo run
   ```

**Error Handling**: If the 1Password reference fails to resolve (CLI not installed, not signed in, or secret not found), Rustbot will:
1. Display a helpful error message with troubleshooting steps
2. Fall back to the setup wizard to manually enter an API key

### Agent Configuration

In your agent JSON files (e.g., `agents/presets/my-agent.json`):

```json
{
  "name": "my-agent",
  "provider": "openrouter",
  "model": "anthropic/claude-sonnet-4",
  "apiKey": "op://Private/Rustbot/openrouter_api_key",
  "systemPrompt": "You are a helpful assistant"
}
```

### MCP Plugin Configuration

In your MCP config file (e.g., `.mcp.json`):

```json
{
  "mcp_plugins": {
    "local_servers": {
      "my-service": {
        "command": "npx",
        "args": ["-y", "my-mcp-server"],
        "env": {
          "API_KEY": "op://Private/MCP Services/my_service_key"
        }
      }
    }
  }
}
```

## Supported Formats

Rustbot's configuration system supports three secret resolution formats:

### 1. 1Password Secret Reference
```json
"apiKey": "op://Private/Rustbot/openrouter_api_key"
```
- Format: `op://vault/item/field`
- Fetches secret from 1Password at config load time
- Requires `op` CLI installed and signed in

### 2. Environment Variable (Required)
```json
"apiKey": "${OPENROUTER_API_KEY}"
```
- Format: `${VAR_NAME}`
- Must be set in environment (`.env` or shell)
- Error if variable not found

### 3. Environment Variable with Default
```json
"model": "${MODEL:-anthropic/claude-sonnet-4}"
```
- Format: `${VAR_NAME:-default_value}`
- Uses environment variable if set
- Falls back to default value if not set or empty

### 4. Plain Values
```json
"name": "assistant"
```
- Regular strings are used as-is
- No resolution or substitution

## Setting Up Secrets in 1Password

### Create a Vault (if needed)
1. Open 1Password app
2. Create a new vault named "Rustbot" or use existing vault

### Add API Key Items

#### For OpenRouter API Key:
1. Create new item in 1Password
2. Item type: "API Credential" or "Password"
3. Item name: "Rustbot OpenRouter"
4. Add field named "api_key" with your OpenRouter API key
5. Save item

#### For MCP Service Keys:
1. Create new item: "MCP Services"
2. Add custom fields for each service:
   - Field name: `github_token` → Value: `ghp_xxx`
   - Field name: `anthropic_key` → Value: `sk-ant-xxx`
   - etc.

### Get the Reference Path

To get the exact reference path:
```bash
op item get "Rustbot OpenRouter" --format=json | jq -r '.fields[] | "op://\(.section.id // "default")/\(.label)"'
```

Or manually construct it:
```
op://[Vault]/[Item Name]/[Field Name]
```

Example:
```
op://Private/Rustbot OpenRouter/api_key
```

## Error Handling

The integration provides helpful error messages:

### `op` CLI Not Installed
```
Failed to execute 1Password CLI. Is it installed?
Install: brew install 1password-cli
Reference: op://Private/Rustbot/api_key
```

### Not Signed In
```
Not signed in to 1Password. Run: op signin
Reference: op://Private/Rustbot/api_key
```

### Secret Not Found
```
1Password secret not found: op://Private/Rustbot/wrong_key
Error: "wrong_key" isn't an item in the "Rustbot" vault
```

### Empty Secret
```
1Password secret is empty: op://Private/Rustbot/api_key
```

## Implementation Details

### Modified Files

1. **`src/main.rs`** (NEW):
   - Added `read_1password_secret()` function (lines 50-114)
   - Added `resolve_api_key()` helper (lines 129-137)
   - Updated API key loading with 1Password support (lines 163-190)
   - Setup wizard saves references as-is (already compatible)

2. **`src/agent/config.rs`**:
   - Added `read_1password_secret()` function
   - Enhanced `resolve_env_var()` to check for `op://` prefix first
   - Maintains backward compatibility with `${VAR}` syntax

3. **`src/mcp/config.rs`**:
   - Added `read_1password_secret()` function
   - Enhanced `resolve_env_var()` with 1Password support
   - Added `${VAR:-default}` syntax support (previously missing)

### Architecture

**Application Startup (main.rs)**:
```
main() startup
        ↓
Load .env.local
        ↓
Read OPENROUTER_API_KEY
        ↓
resolve_api_key(value)
        ↓
    [Check prefix]
        ↓
    op:// ?  →  read_1password_secret()
        ↓                ↓
    Plain key       Execute `op read`
        ↓                ↓
    Return key      Parse + return secret
        ↓                ↓
        └────────────────┘
                ↓
        Use for API calls
```

**Agent/MCP Config Load**:
```
Config Load
        ↓
resolve_env_var(value)
        ↓
    [Check prefix]
        ↓
    op:// ?  →  read_1password_secret() → Execute `op read` command
        ↓                                        ↓
    ${ ?  →  Env var resolution            Parse output
        ↓                                        ↓
    Plain value  →  Return as-is            Return secret
```

### Security Considerations

1. **Secrets in Memory**: Secrets are loaded once at startup and kept in memory
   - Trade-off: Performance vs. secret rotation
   - Mitigation: Restart Rustbot to pick up new secrets

2. **`op` CLI Access**: Requires 1Password CLI to be signed in
   - User's 1Password session provides authentication
   - Session timeout requires re-authentication

3. **Process Output**: Secret values appear in process memory
   - Same risk as environment variables
   - Better than plain text in config files

## Migration Guide

### From Environment Variables

**Before** (`.env.local`):
```bash
OPENROUTER_API_KEY=sk-or-v1-xxxxx
```

**After** (agent config + 1Password):
```json
{
  "apiKey": "op://Private/Rustbot/openrouter_api_key"
}
```

**Steps**:
1. Store secret in 1Password
2. Update agent config with `op://` reference
3. Remove secret from `.env.local`
4. Restart Rustbot

### Hybrid Approach (Recommended)

You can mix formats based on your needs:

```json
{
  "apiKey": "op://Private/Rustbot/openrouter_api_key",
  "model": "${MODEL:-anthropic/claude-sonnet-4}",
  "name": "assistant"
}
```

- Use `op://` for sensitive secrets
- Use `${VAR:-default}` for optional configuration
- Use plain values for non-sensitive defaults

## Testing

### Test 1Password Resolution

Create a test secret:
```bash
# Store a test secret
op item create --category=login \
  --title="Rustbot Test" \
  --vault="Private" \
  password="test-secret-value"

# Verify it works
op read "op://Private/Rustbot Test/password"
```

Update your agent config:
```json
{
  "apiKey": "op://Private/Rustbot Test/password"
}
```

Restart Rustbot and verify it loads successfully.

### Test Backward Compatibility

Ensure existing configs still work:
```bash
# Test environment variable
export TEST_KEY="env-var-value"
# Use ${TEST_KEY} in config

# Test default syntax
# Use ${MISSING_VAR:-default-value} in config
```

## Troubleshooting

### "op: command not found"
**Solution**: Install 1Password CLI
```bash
brew install 1password-cli
```

### "not currently signed in"
**Solution**: Sign in to 1Password
```bash
op signin
```

### "isn't an item in the vault"
**Solution**: Check the reference path
1. Verify vault name (case-sensitive)
2. Verify item name (case-sensitive)
3. Verify field name (case-sensitive)

Use `op item list` to see available items.

### Config loads but secret seems wrong
**Solution**: Test the reference directly
```bash
op read "op://Private/Rustbot/api_key"
```

Verify it returns the expected value.

### Performance concerns
**Solution**: Secrets are cached after first load
- Config load time increases by ~500ms-1s per 1Password secret
- Subsequent accesses use cached value
- No ongoing 1Password CLI calls during runtime

## Best Practices

1. **Use Descriptive Item Names**: "Rustbot OpenRouter API" is clearer than "API Key"
2. **Organize by Service**: Group related secrets in same 1Password item
3. **Document References**: Add comments in config files (if format allows)
4. **Test References First**: Use `op read` to verify before adding to config
5. **Secure Your 1Password**: Use strong master password and 2FA

## Future Enhancements

Potential improvements (not yet implemented):

- **Secret Caching**: Cache resolved secrets to avoid repeated CLI calls
- **Hot Reload**: Detect secret changes and reload without restart
- **Secret Validation**: Validate secret format (e.g., API key patterns)
- **Audit Logging**: Log when secrets are accessed (with redaction)
- **Fallback Mechanisms**: Try multiple resolution strategies

## Related Documentation

- [1Password CLI Documentation](https://developer.1password.com/docs/cli/)
- [Rustbot Configuration Guide](./CONFIGURATION.md)
- [Agent Configuration Reference](./AGENT_CONFIG.md)
- [MCP Plugin Configuration](./MCP_CONFIG.md)
