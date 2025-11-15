# OpenRouter Web Search Integration Fix

**Date**: 2025-11-13
**Session**: Web Search API Error Resolution

## Problem Summary

The application was experiencing an API error when attempting to use OpenRouter's web search feature:
```
OpenRouter API error 400 Bad Request: {"error":{"message":"Unrecognized key: \"tools\"","code":400}
```

## Root Cause

The initial implementation incorrectly attempted to enable web search by adding a `tools` field to the `ProviderConfig` struct. This was based on a misunderstanding of the OpenRouter API structure.

**Incorrect approach:**
```rust
struct ProviderConfig {
    allow_fallbacks: Option<bool>,
    tools: Option<Vec<WebSearchTool>>,  // ❌ WRONG - causes API error
}
```

## Solution

After researching OpenRouter's official documentation, I discovered that web search is enabled using a top-level `web_search_options` field in the API request, NOT through provider configuration.

### Changes Made

#### 1. Created `WebSearchOptions` struct (openrouter.rs:632-637)
```rust
#[derive(Debug, Serialize)]
struct WebSearchOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    engine: Option<String>,  // "exa" or "native"
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,  // default is 5
}
```

#### 2. Simplified `ProviderConfig` struct (openrouter.rs:639-643)
```rust
#[derive(Debug, Serialize)]
struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_fallbacks: Option<bool>,
    // ✅ Removed incorrect 'tools' field
}
```

#### 3. Added `web_search_options` to `ApiRequest` (openrouter.rs:296-298)
```rust
#[derive(Debug, Serialize)]
struct ApiRequest {
    // ... existing fields ...

    /// Web search configuration (OpenRouter feature)
    #[serde(skip_serializing_if = "Option::is_none")]
    web_search_options: Option<WebSearchOptions>,

    /// Provider-specific configuration (e.g., for fallbacks)
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<ProviderConfig>,
}
```

#### 4. Updated `stream_chat()` method (openrouter.rs:60-68)
```rust
// Configure web search if enabled (OpenRouter-specific feature)
let web_search_options = if request.web_search == Some(true) {
    Some(WebSearchOptions {
        engine: None,  // Use default (native if available, otherwise Exa)
        max_results: Some(5),  // Default is 5
    })
} else {
    None
};

let api_request = ApiRequest {
    // ... other fields ...
    web_search_options,
    provider: None,  // Not using provider-specific config anymore
};
```

#### 5. Updated `complete_chat()` method (openrouter.rs:185-193)
Applied the same changes to the non-streaming chat completion method.

## OpenRouter Web Search API Documentation

Based on official OpenRouter documentation:

### Two Methods to Enable Web Search:

**Method 1: Using `:online` suffix**
```json
{
  "model": "anthropic/claude-3.5-sonnet:online"
}
```

**Method 2: Using `web_search_options` field** (our implementation)
```json
{
  "model": "anthropic/claude-3.5-sonnet",
  "web_search_options": {
    "engine": "exa",  // or "native"
    "max_results": 5
  }
}
```

### Configuration Options:
- **engine**:
  - `"exa"` - Uses Exa search engine ($4 per 1,000 results)
  - `"native"` - Uses model provider's built-in search if available
  - `null`/unspecified - Uses native if available, falls back to Exa
- **max_results**: Number of search results to return (default: 5)

## Files Modified

1. `/Users/masa/Projects/rustbot/src/llm/openrouter.rs`
   - Removed incorrect `WebSearchTool` struct
   - Created correct `WebSearchOptions` struct
   - Updated `ProviderConfig` to remove `tools` field
   - Added `web_search_options` field to `ApiRequest`
   - Updated both `stream_chat()` and `complete_chat()` methods

## Build Status

✅ **Build successful** - No compilation errors
⚠️ **Warnings**: Only unused code warnings (dead_code), no blocking issues

## Testing Required

The fix is now ready for testing:

1. **Test Case 1**: Simple web search query
   - Input: "What's the news today?"
   - Expected: Model should use web search tool and return real-time results

2. **Test Case 2**: Specific information query
   - Input: "What's news about GPT-5.1?"
   - Expected: Model should search for and return current information about GPT-5.1

3. **Test Case 3**: Non-web-search query
   - Input: "What is 2+2?"
   - Expected: Model should answer directly without web search

## Key Learnings

1. **Always consult official API documentation** - The OpenRouter API structure was different from initial assumptions
2. **OpenRouter web search uses `web_search_options`** - Not provider-level configuration
3. **Two approaches available** - Can use `:online` model suffix OR `web_search_options` field
4. **Default behavior is intelligent** - Leaving `engine` as `None` uses native search when available, falls back to Exa otherwise

## References

- OpenRouter Web Search Documentation: https://openrouter.ai/docs/features/web-search
- OpenRouter Responses API Beta: https://openrouter.ai/docs/api-reference/responses-api/web-search
- OpenRouter Web Search Announcement: https://openrouter.ai/announcements/introducing-web-search-via-the-api

## Next Steps

1. ✅ Build completed successfully
2. ✅ Application running
3. ⏳ User testing required to verify web search functionality
4. ⏳ Monitor for any additional API errors
5. ⏳ Document results in future session log

---

**Status**: Implementation complete, ready for user testing
