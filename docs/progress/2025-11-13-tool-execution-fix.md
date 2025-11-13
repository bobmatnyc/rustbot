# Tool Execution Decoding Fix - 2025-11-13

## Problem
OpenRouter API was returning "error decoding response body" when tool definitions were provided in `complete_chat` requests. The error occurred during JSON deserialization of the completion response.

## Root Cause Analysis

### Format Mismatch
OpenRouter returns tool calls in **OpenAI's API format**, which differs structurally from our internal `ToolCall` type:

**OpenRouter/OpenAI Response Format:**
```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "I'll help you with that...",
      "tool_calls": [{
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "get_weather",
          "arguments": "{\"location\":\"NYC\", \"units\":\"metric\"}"
        }
      }]
    },
    "finish_reason": "tool_calls"
  }]
}
```

**Our Internal ToolCall Type:**
```rust
struct ToolCall {
    id: String,
    name: String,              // NOT nested in "function"
    arguments: serde_json::Value,  // Parsed JSON, not string
}
```

### Deserialization Issues
1. **Structure mismatch**: OpenRouter wraps function info in a `function` object with `type: "function"` field
2. **Type mismatch**: `arguments` is a JSON string in OpenRouter's format, but `serde_json::Value` in ours
3. **Direct deserialization**: Code attempted to deserialize OpenRouter's response directly into our `Message` type

## Solution

### 1. Created Intermediate API Types
Defined types that exactly match OpenRouter's response format:

```rust
/// Internal representation of a message from OpenRouter API
#[derive(Debug, Deserialize)]
struct ApiMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCall>>,
}

/// OpenRouter/OpenAI format for tool calls
#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiFunctionCall,
}

#[derive(Debug, Deserialize)]
struct ApiFunctionCall {
    name: String,
    arguments: String,  // JSON string, not parsed
}
```

### 2. Added Conversion Logic
Transform OpenRouter format to our internal format in `complete_chat`:

```rust
// Convert OpenRouter API format to our internal format
let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
    calls
        .iter()
        .filter_map(|api_call| {
            // Parse the JSON arguments string into a Value
            match serde_json::from_str(&api_call.function.arguments) {
                Ok(args) => Some(ToolCall {
                    id: api_call.id.clone(),
                    name: api_call.function.name.clone(),
                    arguments: args,
                }),
                Err(e) => {
                    tracing::error!(
                        "Failed to parse tool arguments for {}: {}",
                        api_call.function.name,
                        e
                    );
                    None
                }
            }
        })
        .collect()
});
```

### 3. Enhanced Debug Logging
Added comprehensive logging for troubleshooting:

```rust
// Get response text for debugging
let response_text = response.text().await?;
tracing::debug!("OpenRouter raw response: {}", response_text);

// Deserialize with detailed error reporting
let completion: CompletionResponse = serde_json::from_str(&response_text)
    .map_err(|e| {
        tracing::error!("Failed to deserialize OpenRouter response: {}", e);
        tracing::error!("Raw response was: {}", response_text);
        anyhow::anyhow!("error decoding response body: {}", e)
    })?;
```

## Changes Made

### Files Modified
- **src/llm/openrouter.rs** (Lines 133-294):
  - Added `ApiMessage`, `ApiToolCall`, `ApiFunctionCall` types
  - Modified `complete_chat` to:
    - Read response as text first for logging
    - Deserialize into intermediate `ApiMessage` type
    - Convert to internal `ToolCall` format with argument parsing
    - Handle parse errors gracefully with filter_map
  - Added debug logging for raw responses and errors
  - Handle `content: Option<String>` (may be null when tool calls present)

### Technical Details

**Why the Conversion Layer?**
1. **Separation of Concerns**: API format vs. internal representation
2. **Error Handling**: Parse errors don't crash the entire response
3. **Flexibility**: Easy to support multiple LLM providers with different formats
4. **Debugging**: Log raw responses before transformation

**Error Recovery Strategy**:
- If argument parsing fails for one tool call, log the error but continue processing others
- Use `filter_map` to collect only successfully parsed tool calls
- Return empty content string if content is null (common with tool_calls responses)

## Testing Recommendations

To verify the fix:

1. **Enable debug logging**: `RUST_LOG=debug cargo run`
2. **Send tool-enabled request**: Use agent with tool definitions
3. **Check logs for**:
   - "OpenRouter raw response" showing full API response
   - No "error decoding response body" errors
   - Tool calls successfully parsed and logged
4. **Verify tool execution**: Ensure tool calls trigger agent actions

## Related Components

- **src/llm/types.rs**: Defines internal `ToolCall` and `Message` types
- **src/agent/mod.rs**: Consumes tool calls from LLM responses
- **src/agent/rustbot_api.rs**: Implements `ToolExecutor` trait

## Future Improvements

1. **Add unit tests**: Test ApiMessage deserialization with mock OpenRouter responses
2. **Validate arguments**: Add JSON schema validation for tool arguments
3. **Support streaming tool calls**: Currently only non-streaming complete_chat handles tools
4. **Error metrics**: Track tool call parsing failures for monitoring

## OpenRouter API Compatibility

This fix ensures compatibility with OpenRouter's implementation of the OpenAI-compatible API format. The same approach should work for:
- Direct OpenAI API integration
- Other OpenAI-compatible providers (Groq, Together, etc.)
- Any provider using the function calling standard

## Verification

Build Status: ✅ Compiles successfully
Warnings: Only unused code warnings (not related to this fix)

Next step: Manual testing with actual tool-enabled requests to confirm tool calls are properly decoded and executed.
