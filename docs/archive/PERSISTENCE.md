# System Prompt Persistence Documentation

## Overview

Rustbot persists system and personality prompts between sessions using a file-based storage system. This document describes how the persistence mechanism works and how to verify it.

---

## Persistence Location

**Base Directory**: `~/.rustbot/instructions/`

This directory is automatically created on first run if it doesn't exist.

### Directory Structure

```
~/.rustbot/instructions/
├── system/
│   ├── current             ← Currently active system instructions
│   └── backup_YYYYMMDD_HHMMSS  ← Timestamped backups
└── personality/
    ├── current             ← Currently active personality instructions
    └── backup_YYYYMMDD_HHMMSS  ← Timestamped backups (if edited)
```

---

## How It Works

### Loading on Startup

When Rustbot starts, it automatically loads both prompts from:
- **System Instructions**: `~/.rustbot/instructions/system/current`
- **Personality Instructions**: `~/.rustbot/instructions/personality/current`

If the files don't exist, Rustbot uses default empty strings.

**Code Location**: `src/main.rs:191-218` (`load_system_prompts()`)

### Saving on Changes

When you click "Save Prompts" in the Settings → System Prompts view:

1. **Backup Creation**: If a `current` file exists, it's backed up with a timestamp
2. **Write New Content**: The new prompts are written to the `current` files
3. **Verification**: The system confirms the save operation

**Code Location**: `src/main.rs:220-258` (`save_system_prompts()`)

**UI Trigger**: `src/ui/views.rs:427-429` (Save button in System Prompts view)

---

## Verification

### Current Status (Verified 2025-11-13)

✅ **Persistence is working correctly**

**Evidence**:

```bash
$ cat ~/.rustbot/instructions/system/current
Test system instruction

$ cat ~/.rustbot/instructions/personality/current
Test personality instruction
```

**Backups**:
```bash
$ ls -la ~/.rustbot/instructions/system/
total 16
-rw-r--r--@ 1 masa  staff   27 Nov 12 22:12 backup_20251112_220000
-rw-r--r--@ 1 masa  staff   24 Nov 12 22:12 current
```

---

## How to Verify Persistence Yourself

### Test 1: Edit and Save Prompts

1. **Start Rustbot**:
   ```bash
   cargo run
   ```

2. **Navigate to Settings**:
   - Click the "Settings" button in the sidebar
   - Select "System Prompts" tab

3. **Enter Test Prompts**:
   - **System Instructions**: "This is a test system prompt"
   - **Personality Instructions**: "This is a test personality prompt"

4. **Click "Save Prompts"**

5. **Verify Files Created**:
   ```bash
   cat ~/.rustbot/instructions/system/current
   cat ~/.rustbot/instructions/personality/current
   ```

Expected output should match what you entered.

### Test 2: Verify Loading on Restart

1. **Close Rustbot** (quit the application)

2. **Start Rustbot Again**:
   ```bash
   cargo run
   ```

3. **Check Settings → System Prompts**:
   - The prompts you saved should still be there
   - This confirms they were loaded from disk on startup

### Test 3: Backup Verification

1. **Save prompts** (first time)
2. **Edit prompts again** and save
3. **Check for backup**:
   ```bash
   ls -la ~/.rustbot/instructions/system/
   ```

You should see both:
- `current` - Latest version
- `backup_YYYYMMDD_HHMMSS` - Previous version with timestamp

---

## Implementation Details

### Code Architecture

**Location**: All persistence code is in `src/main.rs`

#### Key Functions

1. **`get_instructions_dir()`** (lines 174-189)
   - Resolves `~/.rustbot/instructions/` path
   - Creates directory if it doesn't exist
   - Cross-platform (works on Windows via `USERPROFILE` env var)

2. **`load_system_prompts()`** (lines 191-218)
   - Called during app initialization
   - Reads both `system/current` and `personality/current`
   - Returns `SystemPrompts` struct
   - Handles missing files gracefully (returns empty strings)

3. **`save_system_prompts()`** (lines 220-258)
   - Creates backup of previous version with timestamp
   - Writes new content to `current` files
   - Creates directories if needed
   - Returns `Result` for error handling

### Data Structure

**`SystemPrompts` struct** (defined in `src/ui/types.rs:30-43`):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct SystemPrompts {
    pub system_instructions: String,
    pub personality_instructions: String,
}
```

- Implements `Serialize`/`Deserialize` (though currently using plain text files)
- Implements `Default` trait for fallback values
- Implements `Clone` for internal use

---

## Agent Configuration Persistence

### Current Status

**Agent configurations** are currently **NOT persisted** to disk. They exist only in memory during the application session.

### Planned Enhancement

Future versions may add agent configuration persistence to:
- `~/.rustbot/agents/` directory
- Store agent-specific settings
- Remember agent selections across sessions

**Tracking Issue**: To be created if prioritized

---

## Troubleshooting

### Prompts Not Saving

**Symptom**: Prompts don't persist between sessions

**Check**:
1. Verify directory exists and is writable:
   ```bash
   ls -la ~/.rustbot/instructions/
   ```

2. Check file permissions:
   ```bash
   ls -la ~/.rustbot/instructions/system/
   ```

3. Check application logs for errors

**Solution**: Ensure the directory has write permissions:
```bash
chmod 755 ~/.rustbot/instructions/
```

### Backup Files Accumulating

**Symptom**: Too many backup files

**Explanation**: Every save creates a new backup with timestamp

**Solution**: Manually clean old backups:
```bash
# Keep only last 5 backups
cd ~/.rustbot/instructions/system/
ls -t backup_* | tail -n +6 | xargs rm -f
```

**Future Enhancement**: Automatic backup rotation (limit to N backups)

---

## File Format

**Format**: Plain text files (UTF-8 encoded)

**No JSON/YAML**: Currently using simple text files for:
- Simplicity
- Easy manual editing
- No parsing overhead
- Human-readable

**Example `system/current`**:
```
You are a helpful AI assistant specialized in Rust programming.
Be concise and focus on practical solutions.
```

---

## Cross-Platform Support

### Supported Platforms

- **macOS**: ✅ Tested and working
- **Linux**: ✅ Should work (uses `$HOME`)
- **Windows**: ✅ Should work (uses `%USERPROFILE%`)

### Path Resolution

```rust
let home_dir = std::env::var("HOME")
    .or_else(|_| std::env::var("USERPROFILE"))
    .map_err(|e| format!("Could not determine home directory: {}", e))?;
```

Falls back from `$HOME` to `$USERPROFILE` for Windows compatibility.

---

## Security Considerations

### File Permissions

**Default permissions**: `rw-r--r--` (644)
- Owner can read/write
- Others can read

**Recommendation**: If storing sensitive prompts, restrict permissions:
```bash
chmod 600 ~/.rustbot/instructions/system/current
chmod 600 ~/.rustbot/instructions/personality/current
```

### Content Sensitivity

**Note**: System prompts may contain:
- API instructions
- Domain-specific knowledge
- User preferences

Treat these files as configuration data and back them up accordingly.

---

## Summary

✅ **System and Personality Prompts PERSIST correctly**
✅ **Automatic backup on save**
✅ **Cross-platform support**
✅ **Simple, reliable file-based storage**
✅ **Verified working (2025-11-13)**

Users don't need to re-enter their prompts on each application restart.
