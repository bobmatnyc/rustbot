# Context Management Design Document

## Overview

This document outlines the design for intelligent context window management in Rustbot, including tracking, visualization, dynamic content management, and automatic compaction when approaching token limits.

## Research Findings

### Context Compaction Techniques (2024-2025)

Based on recent research from Jason Liu's work on AI agent compaction:

**Key Insight**: Effective compaction requires specialized, task-aware prompting strategies rather than generic summarization. The goal is preserving "learned optimization paths" and momentum, not just storing facts.

**Compaction Strategies**:
1. Failure mode detection (loops, conflicts, errors)
2. Language switching analysis (framework transitions)
3. User feedback clustering (preference patterns)

### Local NLP Solutions for Rust

Research into extractive summarization revealed:

**Available Rust Crates**:
- `tfidf-text-summarizer`: TF-IDF based extractive summarization (pure Rust)
- `rust-bert`: Transformer models for summarization (2-4x faster than Python)
- `rsnltk`: Natural language toolkit (with Python bindings)

**Algorithm Options**:
- **TF-IDF**: Lightweight, fast, no external models needed
- **TextRank/LexRank**: Graph-based centrality scoring (mostly Python implementations)
- **Transformer models**: High quality but resource intensive

**Recommendation**: Start with TF-IDF for local summarization due to:
- Pure Rust implementation
- Low resource requirements
- Fast execution
- No model downloads required
- Deterministic results

## Architecture Design

### 1. Context Tracking System

```rust
struct ContextTracker {
    max_tokens: u32,              // Model's context window (e.g., 200k for Claude Sonnet 4)
    current_tokens: u32,          // Current estimated token usage
    system_tokens: u32,           // Tokens used by system context (dynamic)
    conversation_tokens: u32,     // Tokens used by conversation history
    compaction_threshold: f32,    // Trigger compaction (default: 0.50 = 50%)
    warning_threshold: f32,       // Show warning (default: 0.75 = 75%)
}
```

### 2. Dynamic Content Management

**Problem**: System context (date/time, hostname, etc.) should be fresh on each message but not accumulate in token count.

**Solution**: Tag and strip/reinject pattern

```rust
struct DynamicContent {
    id: String,                   // "system_context"
    content: String,              // Current generated content
    token_estimate: u32,          // Cached token count
}

// Before sending to API:
// 1. Strip old dynamic content from history
// 2. Generate fresh dynamic content
// 3. Inject at appropriate position
// 4. Send to LLM
```

**Implementation**:
- Tag dynamic content with special markers in conversation history
- Before each API call, strip all dynamic content
- Regenerate and reinject fresh content
- Token counting only includes static content + one instance of dynamic content

### 3. Context Window Progress Bar

Visual indicator in the UI showing context usage:

```
[████████░░] 80% (160k/200k tokens)
```

**Color Coding**:
- Green: 0-50% (safe)
- Yellow: 50-75% (approaching compaction)
- Orange: 75-90% (warning)
- Red: 90-100% (critical)

**Location**: Below token tracker in status bar

### 4. Compaction Strategy

**Trigger**: When `current_tokens / max_tokens >= compaction_threshold`

**Approach**: Hybrid TF-IDF + Semantic Chunking

```rust
fn compact_conversation(&mut self) -> Vec<ChatMessage> {
    // 1. Identify compactable messages (exclude last N recent messages)
    let recent_count = 10; // Keep last 10 messages uncompacted
    let compactable = &self.messages[..self.messages.len().saturating_sub(recent_count)];

    // 2. Use TF-IDF summarization to extract key sentences
    let summarizer = TfidfSummarizer::new();
    let summary = summarizer.summarize(compactable, compression_ratio: 0.3);

    // 3. Create compacted message
    let compacted_msg = ChatMessage {
        role: MessageRole::System,
        content: format!("[COMPACTED HISTORY]\n{}", summary),
        input_tokens: Some(estimate_tokens(&summary)),
        output_tokens: None,
    };

    // 4. Rebuild message list: [compacted] + [recent messages]
    let mut new_messages = vec![compacted_msg];
    new_messages.extend_from_slice(&self.messages[self.messages.len() - recent_count..]);

    new_messages
}
```

**Compression Strategy**:
- Keep last 10 messages untouched (recent context is critical)
- Summarize older messages using TF-IDF
- Compress to ~30% of original length
- Tag as `[COMPACTED HISTORY]` so LLM knows it's summarized

### 5. Token Estimation

Current method: `text.len() / 4` (rough estimate)

**Improvement**: Use tiktoken-compatible tokenizer

```rust
// Add dependency: tiktoken-rs or similar
fn estimate_tokens_accurate(&self, text: &str) -> u32 {
    // Use actual tokenizer for accurate counts
    tokenizer.encode(text).len() as u32
}
```

**Fallback**: Keep simple estimation for speed, use accurate for compaction decisions

## Implementation Plan

### Phase 1: Context Tracking (Priority: High)
1. Add `ContextTracker` struct to `RustbotApp`
2. Track token usage for each message
3. Calculate current context percentage
4. Add to stats persistence

### Phase 2: Progress Bar UI (Priority: High)
1. Add progress bar component below token tracker
2. Color-coded based on thresholds
3. Show percentage and token counts
4. Update in real-time as messages are sent

### Phase 3: Dynamic Content Stripping/Injection (Priority: High)
1. Tag system context with unique identifier
2. Strip dynamic content before building API messages
3. Regenerate fresh content on each request
4. Inject at correct position in message array

### Phase 4: Compaction System (Priority: Medium)
1. Add `tfidf-text-summarizer` dependency
2. Implement `compact_conversation()` function
3. Trigger at 50% threshold
4. Test with long conversations

### Phase 5: User Controls (Priority: Low)
1. Add compaction settings to Settings page
2. Allow threshold adjustment
3. Manual compaction trigger
4. View compaction history/logs

## Configuration Options

Add to Settings > AI Settings:

```rust
struct CompactionSettings {
    enabled: bool,                     // Enable auto-compaction
    threshold: f32,                    // 0.50 = compact at 50%
    keep_recent_messages: usize,       // Don't compact last N messages
    compression_ratio: f32,            // Target 0.3 = 30% of original
    show_progress_bar: bool,           // Display context usage bar
}
```

## Testing Strategy

1. **Unit Tests**: Token estimation accuracy
2. **Integration Tests**: Compaction preserves key information
3. **Manual Testing**: Long conversation flows
4. **Benchmark**: Compaction performance (should be <100ms)

## Dependencies to Add

```toml
[dependencies]
# Context compaction
tfidf-text-summarizer = "0.1"  # TF-IDF based summarization

# Accurate token counting (optional)
tiktoken-rs = "0.5"             # OpenAI tokenizer

# Or alternative Rust tokenizer
tokenizers = "0.15"             # HuggingFace tokenizers
```

## Future Enhancements

1. **Smart Compaction**: Use LLM for high-quality summarization (optional, user-configurable)
2. **Semantic Chunking**: Group related messages before compaction
3. **Topic Tracking**: Identify and preserve topic transitions
4. **Export/Archive**: Save full conversation before compaction
5. **Multi-level Compaction**: Progressive compression at 50%, 75%, 90%

## Success Metrics

- ✅ Users can have conversations >200k tokens without manual intervention
- ✅ Compaction completes in <100ms
- ✅ Key information preserved (tested via user queries about earlier context)
- ✅ Context usage always visible
- ✅ No unexpected context overflow errors

## References

- Jason Liu's Context Engineering & Compaction: https://jxnl.co/writing/2025/08/30/context-engineering-compaction/
- TF-IDF Text Summarizer (Rust): https://github.com/shubham0204/tfidf-summarizer.rs
- rust-bert NLP pipelines: https://github.com/guillaume-be/rust-bert
- Claude Sonnet 4 context window: 200k tokens
