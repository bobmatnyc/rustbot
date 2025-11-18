# Direct Testing Methods for Rustbot

This document contains methods for directly testing functionality without relying on the UI or full integration.

## Mermaid Diagram Rendering

### Test mermaid.ink API Endpoints

**Create test Mermaid code:**
```bash
cat > /tmp/test_mermaid.txt << 'EOF'
graph TD
    A[Start] --> B[Process]
    B --> C[End]
EOF
```

**Test SVG endpoint:**
```bash
ENCODED=$(base64 < /tmp/test_mermaid.txt | tr -d '\n')
echo "Base64: $ENCODED"
curl -s -o /tmp/test_output.svg "https://mermaid.ink/svg/$ENCODED"
file /tmp/test_output.svg
cat /tmp/test_output.svg | head -20
```

**Test IMG/PNG endpoint (actually returns JPEG):**
```bash
ENCODED=$(base64 < /tmp/test_mermaid.txt | tr -d '\n')
curl -s -o /tmp/test_output.png "https://mermaid.ink/img/$ENCODED"
file /tmp/test_output.png  # Will show: "JPEG image data"
ls -lh /tmp/test_output.png
```

**View the image:**
```bash
open /tmp/test_output.png  # macOS
```

### Test Image Format Detection

**Check image signatures:**
```bash
# PNG signature
hexdump -C /tmp/test.png | head -1
# Should show: 89 50 4E 47 0D 0A 1A 0A

# JPEG signature
hexdump -C /tmp/test.jpg | head -1
# Should show: FF D8 FF

# SVG (text)
head -1 /tmp/test.svg
# Should show: <?xml or <svg
```

## Agent Configuration Testing

### Validate Agent JSON Files

**Check JSON syntax:**
```bash
jq empty agents/presets/assistant.json
jq empty agents/presets/web_search.json
```

**View agent instructions:**
```bash
jq -r '.instruction' agents/presets/assistant.json
```

**Check enabled status:**
```bash
jq '.enabled' agents/presets/*.json
```

## System Prompt Testing

### View Active System Prompt

**Read current system instructions:**
```bash
cat ~/.rustbot/instructions/system/current
```

**Check for Mermaid capability:**
```bash
grep -i "mermaid\|diagram" ~/.rustbot/instructions/system/current
```

## Application Logs

### Monitor Real-time Logs

**Watch application logs:**
```bash
tail -f /tmp/rustbot_mermaid_system.log
```

**Filter for specific events:**
```bash
tail -f /tmp/rustbot_mermaid_system.log | grep -i "mermaid\|error\|warn"
```

**Search recent logs:**
```bash
tail -100 /tmp/rustbot_mermaid_system.log | grep -E "mermaid|PNG|JPEG|render"
```

## Build and Runtime Testing

### Quick Build Validation

**Build and check for errors:**
```bash
cargo build 2>&1 | tail -20
```

**Check for warnings:**
```bash
cargo build 2>&1 | grep -i "warning"
```

**Check build time:**
```bash
time cargo build
```

### Process Management

**Check if rustbot is running:**
```bash
ps aux | grep rustbot | grep -v grep
```

**Kill all rustbot processes:**
```bash
pkill -9 rustbot
```

**Start in background:**
```bash
./target/debug/rustbot > /tmp/rustbot_mermaid_system.log 2>&1 &
echo $!  # Shows PID
```

**Check startup:**
```bash
sleep 2 && tail -20 /tmp/rustbot_mermaid_system.log
```

## Network Testing

### Test API Connectivity

**Test OpenRouter API:**
```bash
curl -s -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  https://openrouter.ai/api/v1/models | jq '.data[0]'
```

**Test mermaid.ink availability:**
```bash
curl -I https://mermaid.ink/
```

## Dependency Verification

### Check Rust Dependencies

**List all dependencies:**
```bash
cargo tree | head -30
```

**Check specific dependency:**
```bash
cargo tree | grep -i "egui_commonmark\|resvg\|usvg"
```

**Verify features enabled:**
```bash
grep -A 2 "egui_commonmark\|egui_extras" Cargo.toml
```

## Code Quality Checks

### Run Clippy

```bash
cargo clippy 2>&1 | grep -v "warning:"
```

### Format Check

```bash
cargo fmt --check
```

### Test Compilation Only

```bash
cargo check
```

## Memory and Performance

### Check Binary Size

```bash
ls -lh target/debug/rustbot
```

### Monitor Memory Usage

```bash
# While app is running
ps aux | grep rustbot | grep -v grep | awk '{print $4"% memory, "$6" KB"}'
```

## MCP Configuration Testing

### Validate MCP Config

**Check MCP config exists:**
```bash
cat mcp_config.json | jq .
```

**List configured servers:**
```bash
jq -r '.mcpServers | keys[]' mcp_config.json
```

## Font and Resource Testing

### Verify Font Files

```bash
ls -lh assets/fonts/
file assets/fonts/Roboto-Regular.ttf
```

### Check Icon Resources

```bash
ls -lh assets/*.icns
file assets/rustbot.icns
```

## Quick Debugging Commands

### All-in-One Debug Session

```bash
# Kill old process
pkill -9 rustbot

# Clean build
cargo clean && cargo build 2>&1 | tail -20

# Start fresh
./target/debug/rustbot > /tmp/rustbot_debug.log 2>&1 &
PID=$!
echo "Started PID: $PID"

# Watch logs
tail -f /tmp/rustbot_debug.log
```

### Emergency Reset

```bash
# Stop everything
pkill -9 rustbot

# Clear caches
rm -rf target/
rm /tmp/rustbot*.log

# Fresh start
cargo build && ./target/debug/rustbot
```

## Tips

- Always check logs first: `tail /tmp/rustbot_mermaid_system.log`
- Use `jq` for JSON validation and pretty-printing
- Use `file` command to verify actual file formats (not just extensions)
- Monitor build output for warnings that might indicate issues
- Test external APIs (mermaid.ink, OpenRouter) independently before debugging app
- Use `grep -E` for multiple pattern matching in logs
- Remember: Rust code changes need rebuild, config changes only need restart
