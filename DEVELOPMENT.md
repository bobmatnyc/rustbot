# Rustbot Development Guide

Comprehensive guide for developing Rustbot with efficient workflows and best practices.

## Quick Reference

### What Requires Rebuild vs Restart

| Change Type | Action Required | Command |
|-------------|----------------|---------|
| Rust code (`.rs` files) | **Rebuild + Restart** | `cargo build && ./target/debug/rustbot` |
| Agent configs (`.json` files) | **Restart only** | Ctrl+C then rerun `./target/debug/rustbot` |
| Dependencies (`Cargo.toml`) | **Rebuild + Restart** | `cargo build && ./target/debug/rustbot` |
| Environment variables (`.env`) | **Restart only** | Ctrl+C then rerun `./target/debug/rustbot` |
| Static assets (`assets/`) | **Restart only** | Ctrl+C then rerun `./target/debug/rustbot` |

### Quick Commands

```bash
# Automated development (recommended)
cargo watch -x run                    # Auto-rebuild on .rs changes
cargo watch -x run -w agents         # Also watch agent configs

# Manual development
cargo build                          # Compile
./target/debug/rustbot               # Run
cargo run                            # Compile + run (slower)

# Validation
jq empty agents/presets/*.json       # Validate all JSON configs
cargo clippy                         # Lint code
cargo test                           # Run tests
```

## Development Workflow

### Automated Development Setup (Recommended)

The fastest way to develop is using `cargo-watch` which automatically rebuilds and restarts when files change.

#### Installation

```bash
# Install cargo-watch (one-time setup)
cargo install cargo-watch
```

#### Basic Usage

**For Code Development:**
```bash
# Watch Rust source files only
cargo watch -x run
```

Changes to `.rs` files trigger automatic rebuild and restart.

**For Code + Config Development:**
```bash
# Watch Rust source files AND agent configs
cargo watch -x run -w agents -w .env
```

Changes to `.rs`, `.json`, or `.env` files trigger rebuild/restart.

#### Custom Watch Patterns

```bash
# Watch specific directories
cargo watch -x run -w src -w agents

# Watch specific file extensions
cargo watch -x 'run --features debug'

# Clear console on each rebuild
cargo watch -c -x run
```

### Manual Development Workflow

If you prefer manual control:

#### Step 1: Code Changes
```bash
# Edit source files
vim src/main.rs
vim src/agent/mod.rs
```

#### Step 2: Build
```bash
# Standard build (debug mode)
cargo build

# Release build (optimized, slower compile)
cargo build --release

# With specific features
cargo build --features web-search
```

#### Step 3: Run
```bash
# Debug build
./target/debug/rustbot

# Release build
./target/release/rustbot
```

#### Step 4: Iterate
- Ctrl+C to stop
- Repeat steps 1-3

### Agent Configuration Workflow

Agent configurations in `agents/presets/*.json` are loaded at runtime.

#### Quick Iteration

1. **Edit agent config:**
   ```bash
   vim agents/presets/assistant.json
   ```

2. **Validate JSON syntax:**
   ```bash
   jq empty agents/presets/assistant.json
   ```

   No output = valid JSON. Errors shown if invalid.

3. **Restart application:**
   - If using `cargo-watch`: Changes auto-detected
   - If running manually: Ctrl+C and rerun `./target/debug/rustbot`

**No rebuild required** - configs are loaded at startup.

#### Config Validation

**Validate single file:**
```bash
jq empty agents/presets/assistant.json
```

**Validate all configs:**
```bash
jq empty agents/presets/*.json
```

**Pretty-print config:**
```bash
jq . agents/presets/assistant.json
```

**Check specific field:**
```bash
jq '.model' agents/presets/web_search.json
```

### Environment Variables

Changes to `.env` file require **restart only** (no rebuild).

#### Common Variables

```bash
# .env file
OPENROUTER_API_KEY=your-key-here
ANTHROPIC_API_KEY=your-key-here
RUST_LOG=debug                    # Enable debug logging
```

#### Reload After Changes

```bash
# If using cargo-watch with -w .env
# Changes auto-detected

# If running manually
# Ctrl+C and rerun
./target/debug/rustbot
```

## Troubleshooting

### Agent Configuration Issues

#### Problem: Agent config changes not reflected

**Cause**: Application still running with old config in memory.

**Solution**:
```bash
# Ensure application is stopped (Ctrl+C)
# Restart to load new config
./target/debug/rustbot
```

#### Problem: JSON syntax error

**Symptoms**: Agent not loading, error in console.

**Solution**:
```bash
# Validate JSON syntax
jq empty agents/presets/assistant.json

# Common issues:
# - Missing comma between fields
# - Trailing comma after last field
# - Unescaped quotes in strings
# - Missing closing brace/bracket
```

**Fix example:**
```json
// WRONG - trailing comma
{
  "name": "assistant",
  "model": "anthropic/claude-3.5-sonnet",
}

// CORRECT
{
  "name": "assistant",
  "model": "anthropic/claude-3.5-sonnet"
}
```

#### Problem: Environment variable not expanded

**Symptoms**: API key shows as `${OPENROUTER_API_KEY}` instead of actual key.

**Solution**:
```bash
# Check environment variable is set
echo $OPENROUTER_API_KEY

# Set if missing
export OPENROUTER_API_KEY="your-key-here"

# Or add to .env file
echo "OPENROUTER_API_KEY=your-key-here" >> .env

# Restart application
./target/debug/rustbot
```

### Build Issues

#### Problem: Compilation fails

**Solution**:
```bash
# Clean build artifacts and rebuild
cargo clean
cargo build

# Check for dependency conflicts
cargo update
cargo build
```

#### Problem: cargo-watch not found

**Solution**:
```bash
# Install cargo-watch
cargo install cargo-watch

# Verify installation
cargo watch --version
```

#### Problem: Slow compilation

**Solutions**:

1. **Use release mode for final testing only:**
   ```bash
   cargo build              # Fast debug builds
   cargo build --release    # Slow optimized builds
   ```

2. **Use incremental compilation (default in debug):**
   ```bash
   # Already enabled by default for debug builds
   # Speeds up subsequent builds
   ```

3. **Build dependencies in parallel:**
   ```bash
   # Set in ~/.cargo/config.toml
   [build]
   jobs = 4  # Number of parallel jobs
   ```

### Runtime Issues

#### Problem: Application crashes on startup

**Check logs:**
```bash
# Enable debug logging
RUST_LOG=debug ./target/debug/rustbot

# Check for:
# - Missing .env file
# - Invalid agent configs
# - Missing API keys
```

#### Problem: Changes not appearing

**Checklist:**
1. Did you save the file?
2. Did you rebuild (for `.rs` changes)?
3. Did you restart (for config/env changes)?
4. Is cargo-watch watching the right files?

**Verify cargo-watch is running:**
```bash
# Should show "Running" messages when files change
cargo watch -x run
```

## Advanced Topics

### Custom Agents

See `agents/README.md` for detailed agent configuration guide.

**Quick steps:**
1. Create `agents/custom/my_agent.json`
2. Follow schema in `agents/schema/agent.schema.json`
3. Validate: `jq empty agents/custom/my_agent.json`
4. Restart application

### Debugging

#### Enable Debug Logging

```bash
# Debug level
RUST_LOG=debug ./target/debug/rustbot

# Trace level (very verbose)
RUST_LOG=trace ./target/debug/rustbot

# Module-specific
RUST_LOG=rustbot::agent=debug ./target/debug/rustbot
```

#### Debug Agent Selection

```rust
// Add to src/agent/mod.rs
println!("Selected agent: {:?}", agent_config);
```

#### Debug API Requests

```rust
// Add to src/api.rs
println!("Request payload: {}", serde_json::to_string_pretty(&payload)?);
```

### Performance Optimization

#### Build Times

**Development (fast iteration):**
```bash
cargo build              # Debug mode
cargo watch -x run       # Auto-rebuild
```

**Production (optimized):**
```bash
cargo build --release    # Full optimizations
strip target/release/rustbot  # Reduce binary size
```

#### Agent Loading

Agent configs are loaded at startup. For faster startup:
- Keep configs small and focused
- Disable unused agents: `"enabled": false`
- Remove unnecessary metadata

### Development with Multiple Terminals

**Recommended setup:**

**Terminal 1 - Development:**
```bash
cargo watch -x run -w agents
```

**Terminal 2 - Testing:**
```bash
# Validate configs
jq empty agents/presets/*.json

# Run tests
cargo test

# Check code quality
cargo clippy
```

**Terminal 3 - Git:**
```bash
git status
git diff
git add -A && git commit -m "feat: Add new agent"
```

### Cargo Aliases

Add to `~/.cargo/config.toml` for project-wide convenience:

```toml
[alias]
dev = "watch -x run"
dev-all = "watch -x run -w agents -w .env"
validate = "clippy -- -D warnings"
```

Then use:
```bash
cargo dev           # Same as cargo watch -x run
cargo dev-all       # Watch code + configs
cargo validate      # Strict linting
```

## Best Practices

### Code Changes

1. **Test before commit:**
   ```bash
   cargo test
   cargo clippy
   ```

2. **Use meaningful commit messages:**
   ```bash
   git commit -m "feat: Add web search agent"
   git commit -m "fix: Resolve JSON parsing issue"
   git commit -m "docs: Update agent configuration guide"
   ```

3. **Keep sessions documented:**
   - Create session logs in `docs/progress/`
   - Follow template in `CLAUDE.md`

### Agent Configuration Changes

1. **Always validate JSON:**
   ```bash
   jq empty agents/presets/assistant.json
   ```

2. **Test with minimal changes:**
   - Change one field at a time
   - Restart and verify
   - Iterate

3. **Document significant changes:**
   - Update `agents/README.md` if needed
   - Add comments in JSON (if schema allows)

### Version Management

See `VERSION_MANAGEMENT.md` for complete guide.

**Quick reference:**
- **Build increment**: Edit `src/version.rs` BUILD constant
- **Patch release**: Increment VERSION patch, reset BUILD
- **Feature release**: Increment VERSION minor, reset BUILD

## Additional Resources

- **Agent Configuration**: `agents/README.md`
- **Version Management**: `VERSION_MANAGEMENT.md`
- **Project Memory**: `CLAUDE.md`
- **Architecture**: `docs/design/ARCHITECTURE.md`
- **Session Logs**: `docs/progress/`

## Getting Help

### Check Documentation

1. This guide for development workflows
2. `agents/README.md` for agent configuration
3. `docs/progress/` for recent changes and solutions

### Debug Checklist

- [ ] Saved all files?
- [ ] Rebuilt (for code changes)?
- [ ] Restarted (for config changes)?
- [ ] Validated JSON syntax?
- [ ] Checked environment variables?
- [ ] Reviewed error messages?
- [ ] Enabled debug logging?

### Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| "Failed to parse agent config" | Invalid JSON | Run `jq empty <file>` |
| "API key required" | Missing/invalid env var | Check `.env` file |
| "Model not found" | Invalid model name | Check provider docs |
| "Connection refused" | API endpoint down | Check network/API status |

## Quick Start Checklist

New to Rustbot development? Follow this:

- [ ] Install Rust 1.70+
- [ ] Clone repository
- [ ] Install cargo-watch: `cargo install cargo-watch`
- [ ] Create `.env` file with API keys
- [ ] Validate configs: `jq empty agents/presets/*.json`
- [ ] Start development: `cargo watch -x run -w agents`
- [ ] Make changes and test
- [ ] Review session logs for recent context

## Workflow Examples

### Example 1: Adding a New Feature

```bash
# Start development server
cargo watch -x run

# Edit code in another terminal
vim src/main.rs

# Save - cargo-watch auto-rebuilds and restarts
# Test feature in UI

# Validate and commit
cargo test
git add -A
git commit -m "feat: Add conversation export"
```

### Example 2: Modifying Agent Config

```bash
# Edit agent config
vim agents/presets/web_search.json

# Validate JSON
jq empty agents/presets/web_search.json

# If using cargo-watch -w agents:
# - Save triggers auto-restart
# - Test changes immediately

# If running manually:
# - Ctrl+C
# - Restart: ./target/debug/rustbot

# Commit changes
git add agents/presets/web_search.json
git commit -m "config: Update web search agent temperature"
```

### Example 3: Debugging an Issue

```bash
# Enable debug logging
RUST_LOG=debug cargo watch -x run

# Reproduce issue in UI
# Check console output for errors

# Add debug prints to code
vim src/agent/mod.rs
# Add: println!("Debug: {:?}", variable);

# Save - auto-rebuild with cargo-watch
# Check new debug output

# Fix issue
# Remove debug prints
# Commit fix
git commit -m "fix: Resolve agent selection issue"
```

---

**Remember**: Code changes need rebuild, config changes only need restart. Use `cargo-watch` for the fastest development experience.
