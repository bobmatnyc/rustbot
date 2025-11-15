# Web Search Fix: Correct OpenRouter API Format

**Date**: 2025-11-13
**Status**: ✅ FIXED
**Issue**: Web search agent returned placeholder text instead of real search results

## Root Cause

The OpenRouter web search API was being called with the **wrong field name**. Our implementation was sending `web_search_options` but OpenRouter's actual API expects a `plugins` array.

### Incorrect Implementation (Before)
```rust
// ❌ WRONG - OpenRouter doesn't recognize this field
#[derive(Debug, Serialize)]
struct WebSearchOptions {
    engine: Option<String>,
    max_results: Option<u32>,
}

struct ApiRequest {
    // ... other fields
    web_search_options: Option<WebSearchOptions>,  // ❌ Wrong field name
}
```

### Correct Implementation (After)
```rust
// ✅ CORRECT - OpenRouter expects plugins array
#[derive(Debug, Serialize)]
struct WebPlugin {
    id: String,  // Must be "web" for web search
    max_results: Option<u32>,
}

struct ApiRequest {
    // ... other fields
    plugins: Option<Vec<WebPlugin>>,  // ✅ Correct field name
}
```

## Discovery Process

1. **Initial Investigation**: Observed that web_search tool was being called correctly but returning placeholder text
2. **API Documentation Search**: Used WebSearch to find OpenRouter's official documentation
3. **Format Verification**: Fetched https://openrouter.ai/docs/features/web-search and confirmed the correct format
4. **Critical Finding**: OpenRouter expects `plugins: [{"id": "web", "max_results": 5}]` NOT `web_search_options`

## Changes Made

### File: `src/llm/openrouter.rs`

**1. Renamed Struct and Updated Fields** (Lines 633-640)
```rust
// Old:
struct WebSearchOptions {
    engine: Option<String>,
    max_results: Option<u32>,
}

// New:
struct WebPlugin {
    id: String,              // Required: "web" for web search
    max_results: Option<u32>,  // Optional: defaults to 5
}
```

**2. Updated ApiRequest Struct** (Lines 294-297)
```rust
// Old:
web_search_options: Option<WebSearchOptions>,

// New:
plugins: Option<Vec<WebPlugin>>,
```

**3. Updated stream_chat() Method** (Lines 60-80)
```rust
// Old:
let web_search_options = if request.web_search == Some(true) {
    Some(WebSearchOptions {
        engine: None,
        max_results: Some(5),
    })
} else {
    None
};

// New:
let plugins = if request.web_search == Some(true) {
    Some(vec![WebPlugin {
        id: "web".to_string(),
        max_results: Some(5),
    }])
} else {
    None
};
```

**4. Updated complete_chat() Method** (Lines 186-206)
- Same changes as stream_chat()

**5. Fixed Test ApiRequest Instantiations** (Lines 751-1095)
- Added `plugins: None` to all test ApiRequest structs
- Updated tests to use `serialize_messages_for_anthropic_value()` for proper message formatting

## OpenRouter API Format

According to OpenRouter documentation (https://openrouter.ai/docs/features/web-search):

### Method 1: Plugins Array (Implemented)
```json
{
  "model": "anthropic/claude-3.5-haiku",
  "messages": [...],
  "plugins": [
    {
      "id": "web",
      "max_results": 5
    }
  ]
}
```

### Method 2: Model Suffix (Alternative)
```json
{
  "model": "anthropic/claude-3.5-haiku:online",
  "messages": [...]
}
```

We implemented Method 1 (plugins array) as it provides more control over search parameters.

## Testing

### Build Status
- ✅ `cargo build` - Successful
- ✅ All existing tests pass
- ✅ Application runs without errors

### Manual Testing Required
Since this requires actual OpenRouter API calls with web search:
1. Start application: `cargo run`
2. Switch to web_search agent in UI
3. Send query: "What's the latest news in AI?"
4. Verify response contains real search results (not placeholder text)

### Expected Behavior
**Before Fix**:
```
Tool execution result: Just a moment while I search for the latest news updates.
```

**After Fix**:
```
Tool execution result: Based on my web search, here are the latest AI developments:
1. [Actual news item with URL]
2. [Another news item with URL]
...
```

## Files Modified

1. `/Users/masa/Projects/rustbot/src/llm/openrouter.rs` - Primary fix
   - Renamed `WebSearchOptions` → `WebPlugin`
   - Changed `web_search_options` → `plugins`
   - Updated both `stream_chat()` and `complete_chat()` methods
   - Fixed all test instantiations

## Related Documentation

- OpenRouter Web Search Docs: https://openrouter.ai/docs/features/web-search
- OpenRouter API Reference: https://openrouter.ai/docs/api-reference/responses-api/web-search

## Impact

- **Scope**: All web_search agent functionality
- **Breaking Changes**: None (internal implementation only)
- **API Compatibility**: Now correctly implements OpenRouter's web search API

## Next Steps

- [x] Fix implemented
- [x] Code compiles successfully
- [x] Documentation updated
- [ ] Manual testing with live API to confirm real search results
- [ ] Consider adding integration test (requires live API key)

## Lessons Learned

1. **Always verify API documentation**: Assumed field names can be wrong
2. **Use WebSearch for current docs**: Official documentation is more reliable than assumptions
3. **Test with DEBUG logs**: Would have caught this earlier by inspecting actual API requests
4. **API format matters**: Small differences (plugins vs web_search_options) cause silent failures

## Related Issues

- Original issue: docs/fix-empty-content-bug.md
- Tool execution: docs/progress/2025-11-13-tool-execution-fix.md
- Format debugging: docs/progress/2025-11-13-tool-calling-format-fix.md
