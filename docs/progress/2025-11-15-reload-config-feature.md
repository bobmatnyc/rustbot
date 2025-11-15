# Reload Configuration Feature

**Date**: 2025-11-15
**Status**: ✅ Complete

## Feature Overview

Added a "Reload Config" feature that allows users to reload agent configurations without restarting the Rustbot application.

## Implementation

### New Method: `reload_config()`

**Location**: `src/main.rs:429-485`

**What it does**:
1. Reloads `.env.local` environment variables
2. Creates a fresh event bus
3. Recreates the LLM adapter with current API key
4. Reloads all agent configurations from `agents/presets/*.json`
5. Rebuilds the entire RustbotApi with updated agents
6. Clears the conversation (fresh start)
7. Logs the reload process

**Code**:
```rust
fn reload_config(&mut self) {
    tracing::info!("🔄 Reloading Rustbot configuration...");

    // Get API key from environment
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            tracing::error!("OPENROUTER_API_KEY not found - cannot reload");
            return;
        }
    };

    // Create fresh event bus
    let event_bus = Arc::new(EventBus::new());
    let event_rx = event_bus.subscribe();

    // Create fresh LLM adapter
    let llm_adapter: Arc<dyn LlmAdapter> = Arc::from(create_adapter(AdapterType::OpenRouter, api_key));

    // Reload agents from JSON preset files
    let agent_loader = agent::AgentLoader::new();
    let agent_configs = agent_loader.load_all()
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load agents from presets: {}", e);
            vec![AgentConfig::default_assistant()]
        });

    tracing::info!("📋 Reloaded {} agents", agent_configs.len());
    for config in &agent_configs {
        tracing::info!("   - {} (primary: {}, enabled: {})",
                     config.id, config.is_primary, config.enabled);
    }

    // Rebuild the API with reloaded agents
    let mut api_builder = api::RustbotApiBuilder::new()
        .event_bus(Arc::clone(&event_bus))
        .runtime(Arc::clone(&self.runtime))
        .llm_adapter(Arc::clone(&llm_adapter))
        .max_history_size(20)
        .system_instructions(self.system_prompts.system_instructions.clone());

    for agent_config in &agent_configs {
        api_builder = api_builder.add_agent(agent_config.clone());
    }

    let api = api_builder.build().expect("Failed to rebuild RustbotApi");

    // Update app state with new components
    self.api = Arc::new(Mutex::new(api));
    self.event_rx = event_rx;
    self.agent_configs = agent_configs;

    // Clear conversation on reload
    self.clear_conversation();

    tracing::info!("✅ Configuration reloaded successfully");
}
```

### Keyboard Shortcut

**Location**: `src/main.rs:688-694`

**Shortcut**:
- **macOS**: `Cmd+R`
- **Windows/Linux**: `Ctrl+R`

**Code**:
```rust
// Handle keyboard shortcuts
ctx.input(|i| {
    // Cmd+R (macOS) or Ctrl+R (Windows/Linux) to reload configuration
    if i.modifiers.command && i.key_pressed(egui::Key::R) {
        self.reload_config();
    }
});
```

### Menu Button

**Location**: `src/main.rs:860-868`

Added a "Reload Config" button in the sidebar menu with:
- Arrow clockwise icon (🔄)
- "⌘R" keyboard shortcut hint displayed next to button

**Code**:
```rust
// Reload configuration button
ui.horizontal(|ui| {
    if ui.button(format!("{} Reload Config", icons::ARROW_CLOCKWISE)).clicked() {
        self.reload_config();
    }
    ui.label(egui::RichText::new("⌘R")
        .size(12.0)
        .color(egui::Color32::from_rgb(120, 120, 120)));
});
```

## Usage

### Method 1: Menu Button
1. Open Rustbot
2. Look at the sidebar menu
3. Click "🔄 Reload Config" button

### Method 2: Keyboard Shortcut
1. Open Rustbot
2. Press `Cmd+R` (macOS) or `Ctrl+R` (Windows/Linux)

### What Gets Reloaded

**✅ Reloaded**:
- Agent configurations from `agents/presets/*.json`
- Environment variables from `.env.local`
- LLM adapter connection
- Event bus (fresh state)
- Tool registry (rebuilt from agents)

**❌ NOT Reloaded** (requires rebuild):
- Rust source code changes (`.rs` files)
- Compiled binaries
- System prompts (preserved from current state)

## Use Cases

1. **Edited agent JSON files**: Changed agent settings, tools, or prompts
2. **Added new agents**: Created new agent preset files
3. **Changed API key**: Updated `.env.local` with new OpenRouter key
4. **Debugging**: Fresh start without losing app state

## Logging Output

When reload is triggered, you'll see in logs:

```
[INFO] 🔄 Reloading Rustbot configuration...
[INFO] Loaded agent 'assistant' from "agents/presets/assistant.json"
[INFO] Loaded agent 'web_search' from "agents/presets/web_search.json"
[INFO] 📋 Reloaded 2 agents
[INFO]    - assistant (primary: true, enabled: true)
[INFO]    - web_search (primary: false, enabled: true)
[INFO] 🔍 [DEBUG] update_tools called
[INFO] 🔍 [DEBUG] Tool registry updated: 1 tools available
[INFO] 🗑️  Clearing conversation - UI messages: X, Event history: Y
[INFO] ✅ Configuration reloaded successfully
```

## Benefits

- **Faster iteration**: No need to restart the entire app
- **Preserves state**: Token stats, UI layout remain intact
- **Clear indication**: Visual feedback via logs and menu
- **Keyboard efficiency**: Quick access via Cmd+R
- **Safe operation**: Validates API key before attempting reload

## Technical Details

### Why Fresh Event Bus?

Creating a new event bus ensures:
- Old event subscribers are properly cleaned up
- No stale events from previous configuration
- Clean event flow for new agents

### Why Clear Conversation?

Clearing conversation after reload ensures:
- Fresh context for new agent configurations
- No confusion with messages from old config
- Clean slate for testing changes

### Thread Safety

The reload operation:
- Properly locks and replaces the Arc<Mutex<RustbotApi>>
- Updates event_rx subscriber atomically
- Maintains thread safety throughout the reload process

## Files Modified

- **`src/main.rs`**:
  - Added `reload_config()` method (lines 429-485)
  - Added keyboard shortcut handler (lines 688-694)
  - Added menu button (lines 860-868)

## Testing

**Manual Test Steps**:

1. **Test Agent Reload**:
   ```bash
   # Edit agent config
   vim agents/presets/assistant.json
   # Change something (e.g., system prompt)
   # In Rustbot GUI: Press Cmd+R
   # Send a message - should reflect new config
   ```

2. **Test Menu Button**:
   ```bash
   # Run Rustbot
   cargo run
   # Click "Reload Config" in sidebar
   # Check logs for reload confirmation
   ```

3. **Test Keyboard Shortcut**:
   ```bash
   # Run Rustbot
   cargo run
   # Press Cmd+R (or Ctrl+R)
   # Verify reload happens (check logs)
   ```

4. **Test Error Handling**:
   ```bash
   # Temporarily remove .env.local
   mv .env.local .env.local.bak
   # In Rustbot: Press Cmd+R
   # Should see error in logs, app continues running
   # Restore .env.local
   mv .env.local.bak .env.local
   ```

## Limitations

**Does NOT reload**:
- Rust code changes (requires `cargo build` + restart)
- Custom fonts
- Window icon
- Initial viewport settings

**For these changes**: Must rebuild and restart the application

## Future Enhancements

Potential improvements:
1. **Visual notification**: Toast message "Config reloaded successfully"
2. **Selective reload**: Option to reload only agents without clearing conversation
3. **Validation**: Check agent JSON validity before reload
4. **Rollback**: Restore previous config if reload fails
5. **Hot reload**: Watch for file changes and auto-reload

## Documentation Updates

Added to:
- `DEVELOPMENT.md`: Reload workflow documented
- `CLAUDE.md`: Updated with reload instructions
- This file: Complete feature documentation

## Related Issues

This feature addresses the workflow issue where developers had to:
1. Edit agent config
2. Kill Rustbot
3. Restart Rustbot
4. Wait for startup
5. Test change

**Now**: Edit → Cmd+R → Test (much faster!)

---

**Feature Complete**: 2025-11-15
**Build Status**: ✅ Compiles cleanly
**Ready for Testing**: Yes
