# Mermaid Rendering - Missing Labels Fix Summary

## Problem
Mermaid diagrams were rendering as images but all text labels were invisible.

## Root Cause
The mermaid.ink API returns SVG files with invalid `<rect>` attributes:
- `<rect width="0" height="0">`
- `<rect width="" height="">`

The usvg parser (used by resvg for SVG to PNG conversion) treats these as invalid and skips the elements entirely. Since these rectangles contain the text labels, skipping them resulted in diagrams with no labels.

## Evidence
Log warnings before fix:
```
WARN usvg::parser::shapes: Rect '' has an invalid 'width' value. Skipped.
```

## Solution
Added SVG pre-processing in `src/mermaid.rs` (lines 293-310) to fix invalid attributes before parsing:

```rust
fn svg_to_png(svg_bytes: &[u8]) -> Result<Vec<u8>> {
    // Pre-process SVG to fix mermaid.ink issues that cause usvg to skip elements
    let svg_str = String::from_utf8_lossy(svg_bytes);

    // Fix invalid rect attributes by replacing width="0" and height="0" with small values
    let fixed_svg = svg_str
        .replace(r#"width="0""#, r#"width="0.1""#)
        .replace(r#"height="0""#, r#"height="0.1""#)
        // Also fix empty width/height attributes
        .replace(r#"width="""#, r#"width="1""#)
        .replace(r#"height="""#, r#"height="1""#);

    // Parse SVG
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(fixed_svg.as_bytes(), &options)?;

    // ... rest of PNG conversion
}
```

## Verification

### Unit Test
Added test in `src/mermaid.rs` (lines 507-524):
```bash
cargo test --lib mermaid::test_svg_to_png_fixes_invalid_rect_attributes
```

### Integration Test
```bash
cargo test --test debug_mermaid_png -- --nocapture
```
Result: No warnings, PNG saved to `/tmp/debug_mermaid.png` with labels visible

### Log Verification
After fix, running the application shows:
```bash
tail -f /tmp/rustbot_mermaid_system.log | grep -E "(WARN|invalid|width)"
```
Result: No more usvg warnings about invalid rect width values

## Files Modified
- `src/mermaid.rs` - Added SVG pre-processing in `svg_to_png()` function
- `docs/MERMAID_REGRESSION_LOG.md` - Updated with fix details

## Test Coverage
- 8 unit tests in `src/mermaid.rs` (all passing)
- 7 integration tests in `tests/integration_mermaid_test.rs`
- 1 debug test in `tests/debug_mermaid_png.rs`
- E2E test script in `tests/e2e_mermaid_test.sh`

## Status
✅ **RESOLVED** - All Mermaid rendering issues now fixed:
1. ✅ Red triangle (SVG data URL not supported) - Fixed with PNG conversion
2. ✅ Transparent backgrounds - Fixed with theme config
3. ✅ API 404 errors - Fixed with simplified theme
4. ✅ Missing labels - Fixed with SVG pre-processing

## Quick Regression Test
Before any future mermaid changes, run:
```bash
cargo test --lib mermaid::
cargo test --test integration_mermaid_test
cargo test --test debug_mermaid_png -- --nocapture
tail -50 /tmp/rustbot_mermaid_system.log | grep -E "(ERROR|WARN.*Rect)"
```

All tests should pass and no usvg warnings should appear in logs.
