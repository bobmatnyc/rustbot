# Rustbot Cleanup Plan

**Date**: 2025-11-15
**Purpose**: Organize test files, debug docs, and temporary files

## Files to Archive

### Debug Documentation (Root Directory → Archive)

**Move to `docs/archive/debug/`**:
- `DEBUGGING_EMPTY_MESSAGES.md` - Empty message debugging (FIXED)
- `DEBUGGING_TOOL_STATE.md` - Tool state debugging guide
- `TEST_TOOL_CALLING.md` - Tool calling test guide
- `TESTING_INSTRUCTIONS.md` - Testing instructions

### Progress Logs to Consolidate

**Keep most recent, archive duplicates**:

#### 2025-11-13 Files (Multiple similar topics)
- Tool calling: 6 files → Keep final diagnosis, archive rest
- Empty messages: 5 files → Keep root cause fix, archive rest
- Performance: 2 files → Keep complete, archive analysis
- Web search: 2 files → Keep final fix, archive debug

#### 2025-11-15 Files (Current)
- Keep all (active development)

## Files to Keep

### Root Directory
- `README.md` ✅
- `CLAUDE.md` ✅
- `DEVELOPMENT.md` ✅
- `VERSION_MANAGEMENT.md` ✅
- `Cargo.toml` ✅
- `.gitignore` ✅

### Examples (Keep All - Useful)
- `api_demo.rs` ✅
- `test_agent_config_loading.rs` ✅
- `test_gui_api.rs` ✅
- `test_tool_calling.rs` ✅

### Documentation
- `docs/progress/` - Keep recent, archive old
- `docs/archive/` - Archive location

## Cleanup Actions

### 1. Create Archive Structure
```bash
mkdir -p docs/archive/debug
mkdir -p docs/archive/progress/2025-11-12
mkdir -p docs/archive/progress/2025-11-13
```

### 2. Move Debug Docs
```bash
mv DEBUGGING_*.md docs/archive/debug/
mv TEST_TOOL_CALLING.md docs/archive/debug/
mv TESTING_INSTRUCTIONS.md docs/archive/debug/
```

### 3. Archive Old Progress (2025-11-12)
```bash
mv docs/progress/2025-11-12-*.md docs/archive/progress/2025-11-12/
```

### 4. Consolidate 2025-11-13 Progress

**Keep (Final/Complete)**:
- `2025-11-13-tool-execution-complete.md` - Final tool execution
- `2025-11-13-empty-content-bug-ROOT-CAUSE-FIX.md` - Root cause fix
- `2025-11-13-performance-optimization-complete.md` - Performance final
- `2025-11-13-web-search-plugins-fix.md` - Web search final

**Archive (Debug/Interim)**:
```bash
# Tool calling intermediates
mv docs/progress/2025-11-13-tool-calling-*.md docs/archive/progress/2025-11-13/
mv docs/progress/2025-11-13-tool-call-detection.md docs/archive/progress/2025-11-13/
mv docs/progress/2025-11-13-tool-execution-fix.md docs/archive/progress/2025-11-13/
mv docs/progress/2025-11-13-tool-execution-implementation.md docs/archive/progress/2025-11-13/

# Empty message intermediates
mv docs/progress/2025-11-13-empty-*.md docs/archive/progress/2025-11-13/
# Keep: 2025-11-13-empty-content-bug-ROOT-CAUSE-FIX.md (final)

# Other intermediates
mv docs/progress/2025-11-13-serialization-debug-logging.md docs/archive/progress/2025-11-13/
mv docs/progress/2025-11-13-conversation-history-fix.md docs/archive/progress/2025-11-13/
```

### 5. Keep Current (2025-11-15)

**Keep all 2025-11-15 files** (active development):
- `2025-11-15-session.md`
- `2025-11-15-tool-calling-*.md`
- `2025-11-15-reload-config-feature.md`
- `2025-11-15-agent-config-verification.md`

### 6. Clean /tmp/ Files

**Remove old logs**:
```bash
rm /tmp/rustbot_debug.log
rm /tmp/rustbot_test_output.log
rm /tmp/rustbot_test.log
rm /tmp/rustbot_tool_test_output.log
rm /tmp/rustbot_trace.log
rm /tmp/rustbot.log
rm /tmp/test_gui_api.rs
```

**Keep recent**:
- `/tmp/gui_api_test.log` (today's test)
- `/tmp/rustbot_reload_test.log` (today's reload test)

## Final Structure

```
rustbot/
├── README.md
├── CLAUDE.md
├── DEVELOPMENT.md
├── VERSION_MANAGEMENT.md
├── Cargo.toml
├── examples/
│   ├── api_demo.rs
│   ├── test_agent_config_loading.rs
│   ├── test_gui_api.rs
│   └── test_tool_calling.rs
├── docs/
│   ├── archive/
│   │   ├── debug/
│   │   │   ├── DEBUGGING_EMPTY_MESSAGES.md
│   │   │   ├── DEBUGGING_TOOL_STATE.md
│   │   │   ├── TEST_TOOL_CALLING.md
│   │   │   └── TESTING_INSTRUCTIONS.md
│   │   └── progress/
│   │       ├── 2025-11-12/
│   │       │   ├── 2025-11-12-session.md
│   │       │   └── 2025-11-12-session-2.md
│   │       └── 2025-11-13/
│   │           ├── (intermediate debug files)
│   │           └── ...
│   └── progress/
│       ├── 2025-11-13-tool-execution-complete.md (FINAL)
│       ├── 2025-11-13-empty-content-bug-ROOT-CAUSE-FIX.md (FINAL)
│       ├── 2025-11-13-performance-optimization-complete.md (FINAL)
│       ├── 2025-11-13-web-search-plugins-fix.md (FINAL)
│       ├── 2025-11-15-session.md
│       ├── 2025-11-15-tool-calling-diagnosis-complete.md
│       ├── 2025-11-15-reload-config-feature.md
│       └── 2025-11-15-agent-config-verification.md
└── src/
    └── ...
```

## Benefits

1. **Cleaner root directory** - No debug files
2. **Organized archives** - Easy to find historical context
3. **Clear progress** - Only final/important docs in progress/
4. **Preserved history** - Nothing deleted, just organized
5. **Easier navigation** - Less clutter, clearer structure

## Execute Cleanup

Run: `./scripts/cleanup_project.sh`

Or execute commands manually from this plan.
