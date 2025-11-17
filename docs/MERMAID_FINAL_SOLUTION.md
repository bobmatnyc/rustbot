# Mermaid Rendering - Final Solution

## Summary

After extensive troubleshooting and testing, Mermaid diagram rendering now works correctly with **visible labels** using the mermaid.ink `/img/` endpoint.

## Solution Overview

**Endpoint**: `https://mermaid.ink/img/{base64_encoded_diagram}`
**Format**: JPEG (not PNG/SVG)
**Labels**: ✅ Visible (pre-rendered in JPEG)
**Transparency**: ❌ Not supported (white background)

## Technical Details

### Why JPEG?

1. **foreignObject Issue**: mermaid.ink's `/svg/` endpoint uses `<foreignObject>` elements for text labels
2. **usvg Limitation**: The usvg library (used by resvg) doesn't support `<foreignObject>`
3. **Pre-rendered Solution**: The `/img/` endpoint returns pre-rendered JPEG images with labels already embedded

### Tradeoffs Accepted

| Feature | Status | Notes |
|---------|--------|-------|
| **Labels Visible** | ✅ Works | Primary requirement - ACHIEVED |
| **Transparent Background** | ❌ Not supported | JPEG format limitation |
| **Theme Customization** | ❌ Not supported | `/img/` endpoint uses default theme only |
| **Image Quality** | ✅ Good | JPEG compression acceptable for diagrams |
| **Performance** | ✅ Excellent | Direct API response, no SVG processing |

## Implementation

### Files Modified

**`src/mermaid.rs`**:
- Changed endpoint from `/svg/` to `/img/`
- Removed theme configuration (not supported by `/img/`)
- Changed validation from SVG to JPEG (magic bytes: `FF D8 FF`)
- Removed SVG-to-PNG conversion (no longer needed)

**`src/main.rs`**:
- Changed data URL format from `data:image/png` to `data:image/jpeg`
- Updated logging to reflect JPEG format

**`tests/debug_mermaid_png.rs`**:
- Updated to expect JPEG output
- Changed output file to `.jpg` extension
- Updated signature validation

### Code Example

```rust
// mermaid.rs - Key changes
let encoded = BASE64.encode(mermaid_code.as_bytes()); // No theme config
let url = format!("https://mermaid.ink/img/{}", encoded);

// Validate JPEG signature
if &jpeg_bytes[0..3] != &[0xFF, 0xD8, 0xFF] {
    return Err(MermaidError::InvalidSyntax(...));
}
```

```rust
// main.rs - Data URL format
let data_url = format!("data:image/jpeg;base64,{}", img_base64);
```

## Testing

### E2E Test Suite

Created comprehensive E2E test: `tests/e2e_mermaid_labels_test.sh`

**Tests Covered**:
1. ✅ API endpoint accessibility
2. ✅ JPEG format validation
3. ✅ Image size verification (labels add content)
4. ✅ Complex diagram rendering
5. ✅ Theme config rejection (expected behavior)
6. ✅ File integrity validation
7. ✅ Unit test suite execution
8. ✅ Debug test output validation

**Run Tests**:
```bash
# Full E2E test suite
./tests/e2e_mermaid_labels_test.sh

# Unit tests
cargo test --lib mermaid::

# Debug test with visual output
cargo test --test debug_mermaid_png -- --nocapture
open /tmp/debug_mermaid.jpg
```

### Regression Prevention

To prevent regressions, run before any mermaid-related changes:

```bash
# Quick check
./tests/e2e_mermaid_labels_test.sh

# Full verification
cargo test --lib mermaid::
cargo test --test integration_mermaid_test
./tests/e2e_mermaid_labels_test.sh
```

## Known Limitations

1. **No Transparency**: JPEG format doesn't support alpha channel
   - **Impact**: Diagrams have white background
   - **Mitigation**: Acceptable for most use cases

2. **No Theme Customization**: `/img/` endpoint uses default theme
   - **Impact**: Cannot customize colors, fonts, or styling
   - **Mitigation**: Default theme is clean and professional

3. **Image Compression**: JPEG uses lossy compression
   - **Impact**: Minor artifacts possible on complex diagrams
   - **Mitigation**: Quality is good enough for typical diagrams

## Future Improvements

### Option 1: Use Different SVG Renderer (High Effort)

Replace `usvg` with a library that supports `<foreignObject>`:
- **Candidate**: librsvg (Rust bindings)
- **Pros**: Full SVG support, transparency works
- **Cons**: Larger dependency, platform-specific builds

### Option 2: Server-Side Rendering (Medium Effort)

Run mermaid-cli on server to generate PNGs:
- **Pros**: Full control, transparency support
- **Cons**: Requires Node.js, Puppeteer, server infrastructure

### Option 3: Accept Current Solution (Recommended)

Keep JPEG approach:
- **Pros**: Simple, reliable, no dependencies
- **Cons**: White background
- **Verdict**: ✅ Good enough for now

## Comparison: Before vs After

| Aspect | Before (Broken) | After (Working) |
|--------|----------------|-----------------|
| Labels | ❌ Invisible | ✅ Visible |
| Format | SVG → PNG | JPEG (direct) |
| Transparency | ✅ (but broken) | ❌ (but working) |
| Endpoint | `/svg/` | `/img/` |
| Processing | usvg + resvg | None (direct use) |
| Dependencies | tiny-skia, usvg | None (base64 only) |
| Theme | Custom | Default |

## Decision Log

### Why Not Fix usvg?

usvg's lack of `<foreignObject>` support is intentional - it's not a bug:
- foreignObject embeds HTML, not SVG primitives
- Would require HTML rendering engine (like browser)
- Out of scope for lightweight SVG parser

### Why Not Use librsvg?

- Platform-specific builds (needs system librsvg)
- Large dependency footprint
- Complexity not justified for this use case

### Why JPEG is Acceptable

1. **Primary Goal**: Labels visible ← **ACHIEVED**
2. **Secondary Goal**: Transparency ← **Nice to have, not critical**
3. **Simplicity**: No SVG processing, fewer dependencies
4. **Reliability**: Direct API response, no conversion errors
5. **Performance**: Faster than SVG parsing + rasterization

## Conclusion

The JPEG solution is a **pragmatic tradeoff** that prioritizes:
- ✅ **Functionality** (labels work)
- ✅ **Reliability** (simple, tested)
- ✅ **Maintainability** (fewer dependencies)

Over:
- ❌ **Aesthetics** (transparent background)
- ❌ **Customization** (theme support)

This is the right choice for production use.
