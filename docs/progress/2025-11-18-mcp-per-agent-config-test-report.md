# Per-Agent MCP Configuration Testing Report
**Date**: 2025-11-18
**Tester**: QA Agent
**Test Target**: Per-agent MCP configuration system
**Status**: ❌ **CRITICAL FAILURE - Application Launch Blocked**

---

## Executive Summary

The newly implemented per-agent MCP configuration architecture **CANNOT BE TESTED** due to a critical JSON deserialization bug that prevents the application from launching. The root cause is a field naming convention mismatch between agent configuration JSON files and the Rust deserialization code.

**Impact**: Severity 1 (Critical) - Application completely non-functional

---

## Test Environment

- **Binary**: `/Users/masa/Projects/rustbot/target/release/rustbot`
- **Build Date**: 2025-11-18 04:41
- **Test Extension**: Gmail MCP (`com.mintmcp/gmail`)
- **Config Locations**:
  - Global: `~/.rustbot/mcp_config.json`
  - Per-Agent: `~/.rustbot/mcp_configs/{agent_id}_mcp.json` (directory not created - never reached)

---

## Test Results

### ❌ Test 1: Application Launch
**Status**: **FAILED**
**Severity**: Critical

**Command**:
```bash
/Users/masa/Projects/rustbot/target/release/rustbot
```

**Error Output**:
```
[ERROR] Failed to load agent from "agents/presets/assistant.json": Failed to parse agent JSON
[ERROR] Failed to load agent from "agents/presets/web_search.json": Failed to parse agent JSON

thread 'main' panicked at tokio-1.48.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```

**Root Cause Analysis**:

The application crashes during agent loading due to JSON deserialization failures. Investigation reveals a **field naming convention mismatch**:

| Field Purpose | Agent JSON File Uses | Rust Deserializer Expects | Result |
|---------------|---------------------|---------------------------|--------|
| MCP Extensions | `mcp_extensions` (snake_case) | `mcpExtensions` (camelCase) | ❌ Deserialization fails |
| Primary Agent Flag | `is_primary` (snake_case) | `isPrimary` (camelCase) | ❌ Deserialization fails |
| Web Search Flag | `web_search_enabled` (snake_case) | `capabilities.webSearch` (camelCase, nested) | ❌ Deserialization fails |

**Evidence from Code**:

**File**: `src/agent/config.rs`
```rust
pub struct JsonAgentConfig {
    // ... other fields ...

    #[serde(rename = "mcpExtensions")]  // ← Expects "mcpExtensions" in JSON
    pub mcp_extensions: Vec<String>,

    #[serde(rename = "isPrimary")]      // ← Expects "isPrimary" in JSON
    pub is_primary: bool,

    pub capabilities: AgentCapabilities, // ← Nested structure
}

pub struct AgentCapabilities {
    #[serde(rename = "webSearch")]      // ← Expects capabilities.webSearch
    pub web_search: bool,
}
```

**File**: `agents/presets/assistant.json`
```json
{
  "id": "assistant",
  "name": "assistant",
  "mcp_extensions": ["com.mintmcp/gmail"],  // ← Wrong: should be "mcpExtensions"
  "is_primary": true,                        // ← Wrong: should be "isPrimary"
  "web_search_enabled": true                 // ← Wrong: should be in capabilities object
}
```

**Impact**:
- Application cannot start
- No agent functionality available
- All per-agent MCP configuration tests blocked

---

### ⏸️ Test 2-7: All Remaining Tests
**Status**: **BLOCKED**
**Reason**: Application cannot launch due to Test 1 failure

The following tests could not be executed:

- ⏸️ **Test 2**: Verify Directory Structure - Blocked
- ⏸️ **Test 3**: Check Installed Extensions - Blocked
- ⏸️ **Test 4**: Manual Test - Install Gmail for Specific Agent - Blocked
- ⏸️ **Test 5**: Verify Per-Agent Config Created - Blocked
- ⏸️ **Test 6**: Verify Global Config Unchanged - Blocked
- ⏸️ **Test 7**: Test Agent Config Schema - Blocked
- ⏸️ **Test 8**: Restart and Verify Tool Loading - Blocked

---

## Bug Analysis

### Bug #1: JSON Field Naming Convention Mismatch
**Type**: Data Serialization / Configuration Error
**Severity**: Critical (Blocks all functionality)
**Component**: Agent Configuration Loader

**Detailed Analysis**:

1. **The Problem**: The Rust code's serde attributes use camelCase JSON field names, but the actual JSON configuration files use snake_case field names.

2. **Why It Fails**:
   - Serde's `#[serde(rename = "...")]` attribute tells the deserializer to look for a specific JSON field name
   - When that field name doesn't exist in the JSON, deserialization fails
   - The `mcp_extensions` field in JSON is ignored because serde is looking for `mcpExtensions`

3. **Inconsistency Sources**:
   - The schema file (`agents/schema/agent.schema.json`) uses camelCase: `mcpExtensions`, `isPrimary`
   - The Rust struct uses camelCase renames to match the schema
   - BUT the actual agent JSON files use snake_case: `mcp_extensions`, `is_primary`

4. **Why This Wasn't Caught Earlier**:
   - Recent code changes added new fields (`mcp_extensions`, `mcp_config_file`, `web_search_enabled`)
   - These fields were added to agent JSON files in snake_case
   - The serde rename attributes expect camelCase
   - No integration tests validate actual agent JSON file deserialization

---

## Fix Recommendations

### Priority 1: Critical - Make Application Functional

**Option A: Update Agent JSON Files (Recommended - Quick Fix)**

Change all agent JSON files to use camelCase field names matching the schema:

**File**: `agents/presets/assistant.json`
```json
{
  "id": "assistant",
  "name": "assistant",
  "instructions": "...",
  "personality": "...",
  "model": "anthropic/claude-sonnet-4.5",
  "enabled": true,
  "isPrimary": true,                      // ← Changed from is_primary
  "capabilities": {                       // ← Changed from flat web_search_enabled
    "webSearch": true
  },
  "mcpExtensions": ["com.mintmcp/gmail"], // ← Changed from mcp_extensions
  "mcpConfigFile": null                   // ← Changed from mcp_config_file (if present)
}
```

**Pros**:
- Fastest fix (just edit JSON files)
- Matches the official schema
- No code changes required

**Cons**:
- Breaks backward compatibility if users have custom agent configs

**Files to Update**:
- `agents/presets/assistant.json`
- `agents/presets/web_search.json`
- Any other agent preset files

---

**Option B: Update Rust Serde Attributes (More Breaking)**

Remove the `#[serde(rename = "...")]` attributes to accept snake_case:

```rust
pub struct JsonAgentConfig {
    // Remove rename attribute, accept snake_case from JSON
    #[serde(default)]
    pub mcp_extensions: Vec<String>,  // Accepts "mcp_extensions" from JSON

    #[serde(default)]
    pub is_primary: bool,             // Accepts "is_primary" from JSON
}
```

**Pros**:
- More Rust-idiomatic (snake_case is Rust convention)
- Matches current JSON files

**Cons**:
- Breaks schema compliance
- Requires updating the JSON schema file
- More invasive code change

---

**Option C: Support Both Naming Conventions (Best Long-term)**

Use serde's alias feature to accept both:

```rust
#[serde(rename = "mcpExtensions", alias = "mcp_extensions")]
pub mcp_extensions: Vec<String>,

#[serde(rename = "isPrimary", alias = "is_primary")]
pub is_primary: bool,
```

**Pros**:
- Backward compatible
- Accepts both naming conventions
- Graceful migration path

**Cons**:
- Slightly more complex
- Might hide inconsistencies

---

### Priority 2: Prevent Future Issues

1. **Add Integration Tests**:
   ```rust
   #[test]
   fn test_load_actual_agent_files() {
       // Test that all agent preset files can be loaded
       let assistant = JsonAgentConfig::from_file("agents/presets/assistant.json");
       assert!(assistant.is_ok());

       let web_search = JsonAgentConfig::from_file("agents/presets/web_search.json");
       assert!(web_search.is_ok());
   }
   ```

2. **Add Schema Validation**:
   - Use a JSON schema validator in CI/CD
   - Validate agent preset files against `agents/schema/agent.schema.json`
   - Catch mismatches before code runs

3. **Document Naming Convention**:
   - Add clear documentation in `DEVELOPMENT.md` about field naming
   - Explain why camelCase is used (JSON schema standard)
   - Provide examples of correct agent config files

---

## Acceptance Criteria Status

### ✅ Build & Launch
- [x] Application builds successfully
- [ ] ❌ Application launches without errors (FAILED)
- [ ] ❌ No crashes or segfaults (FAILED - panic on startup)

### ⏸️ Directory Creation (BLOCKED)
- [ ] `~/.rustbot/mcp_configs/` directory created on first agent-specific install (NOT TESTED)

### ⏸️ Agent-Specific Installation (BLOCKED)
- [ ] Can install extension for specific agent (NOT TESTED)
- [ ] Creates `{agent_id}_mcp.json` file (NOT TESTED)
- [ ] Config contains only that extension (NOT TESTED)

### ⏸️ Global Installation (BLOCKED)
- [ ] Can install extension globally (NOT TESTED)
- [ ] Updates `~/.rustbot/mcp_config.json` (NOT TESTED)
- [ ] All agents have access (NOT TESTED)

### ⏸️ No Regressions (BLOCKED)
- [ ] Existing functionality still works (CANNOT TEST - app won't start)
- [ ] No new errors or warnings (FAILED - critical errors present)
- [ ] UI is responsive (CANNOT TEST - app won't start)

---

## Evidence

### Command Outputs

**Build Status**:
```bash
$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.11s
```
✅ Build succeeds

**Launch Attempt**:
```bash
$ ./target/release/rustbot
[ERROR] Failed to load agent from "agents/presets/assistant.json": Failed to parse agent JSON
[ERROR] Failed to load agent from "agents/presets/web_search.json": Failed to parse agent JSON
thread 'main' panicked at tokio-1.48.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```
❌ Launch fails

**Process Status**:
```bash
$ ps aux | grep rustbot
# No process found - application crashed
```
❌ Application not running

---

### Configuration Files

**Extension Registry** (`~/.rustbot/extensions/registry.json`):
```json
{
    "extensions": {
        "com.mintmcp/gmail": {
            "id": "com.mintmcp/gmail",
            "name": "com.mintmcp/gmail",
            "description": "A MCP server for Gmail...",
            "install_type": "remote",
            "mcp_config": {
                "cloud_service": {
                    "id": "com.mintmcp/gmail",
                    "url": "https://gmail.mintmcp.com/mcp",
                    "enabled": false
                }
            },
            "metadata": {
                "version": "1.0.5",
                "installed_at": "2025-11-18T07:46:58.048456+00:00"
            }
        }
    }
}
```
✅ Gmail extension available for testing (when app works)

**Directory Structure**:
```bash
$ ls -la ~/.rustbot/
drwxr-xr-x  extensions/
drwxr-xr-x  instructions/
# mcp_configs/ directory NOT created - never reached that code path
```

---

## Conclusions

1. **Critical Blocker**: The application cannot launch due to JSON deserialization failures in agent configuration loading.

2. **Root Cause**: Field naming convention mismatch between:
   - Agent JSON files (snake_case: `mcp_extensions`, `is_primary`)
   - Rust deserializer (camelCase: `mcpExtensions`, `isPrimary`)

3. **Testing Blocked**: Cannot test any per-agent MCP configuration features until this bug is fixed.

4. **Impact**: Complete application failure - no functionality available.

---

## Recommended Next Steps

### Immediate (Critical):
1. **Fix JSON Field Names**: Update `agents/presets/*.json` files to use camelCase field names
2. **Test Application Launch**: Verify app starts after JSON fixes
3. **Re-run This Test Suite**: Execute all blocked tests once app launches

### Short-term (High Priority):
1. **Add Integration Tests**: Test actual agent file loading in CI
2. **Schema Validation**: Add JSON schema validation to prevent future mismatches
3. **Documentation**: Document field naming conventions clearly

### Medium-term (Recommended):
1. **Support Both Conventions**: Use serde aliases for backward compatibility
2. **Migration Guide**: If changing conventions, provide migration guide for users
3. **Better Error Messages**: Improve deserialization error messages to identify field mismatches

---

## Test Report Summary

| Test Category | Tests Planned | Tests Passed | Tests Failed | Tests Blocked |
|---------------|---------------|--------------|--------------|---------------|
| Application Launch | 1 | 0 | 1 | 0 |
| Configuration System | 6 | 0 | 0 | 6 |
| **Total** | **7** | **0** | **1** | **6** |

**Overall Status**: ❌ **CRITICAL FAILURE**

**Recommendation**: **DO NOT MERGE** until JSON field naming issue is resolved.

---

## Appendix: Field Naming Reference

### Current Agent JSON Files (INCORRECT)
```json
{
  "mcp_extensions": [...],
  "is_primary": true,
  "web_search_enabled": true
}
```

### Expected by Rust Code (CORRECT per Schema)
```json
{
  "mcpExtensions": [...],
  "isPrimary": true,
  "capabilities": {
    "webSearch": true
  }
}
```

### JSON Schema Definition
```json
{
  "properties": {
    "mcpExtensions": {
      "type": "array",
      "items": {"type": "string"}
    },
    "isPrimary": {
      "type": "boolean"
    },
    "capabilities": {
      "type": "object",
      "properties": {
        "webSearch": {"type": "boolean"}
      }
    }
  }
}
```

---

**Report Generated**: 2025-11-18
**Tester**: QA Agent
**Status**: Ready for Engineer Review
