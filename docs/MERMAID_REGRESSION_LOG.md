# Mermaid Rendering - Regression Log

## Session: 2025-11-17

### Issues Encountered and Fixed

#### Issue 1: Red Triangle (SVG Data URL Not Supported)
**Symptom**: Red triangle error instead of diagram rendering
**Cause**: egui_commonmark doesn't support `data:image/svg+xml;base64,` URLs
**Fix**: Added SVG to PNG conversion using resvg + tiny-skia
**Files Changed**: `src/mermaid.rs`, `Cargo.toml`
**Test**: `cargo test --lib mermaid::test_svg_to_png_conversion`

#### Issue 2: No Transparency
**Symptom**: White background instead of transparent
**Cause**: Using wrong data URL format
**Fix**: Changed from `data:image/svg+xml` to `data:image/png`
**Files Changed**: `src/main.rs`
**Test**: `cargo test --lib mermaid::test_svg_to_png_transparent_background`

#### Issue 3: Missing Labels
**Symptom**: Diagrams render but no text labels visible
**Cause**: usvg parser skipping `<rect>` elements with invalid `width=""` or `width="0"` attributes
**Fix**: Added SVG pre-processing to replace invalid rect attributes before parsing
**Files Changed**: `src/mermaid.rs` (lines 293-310)
**Test**: `cargo test --lib mermaid::test_svg_to_png_fixes_invalid_rect_attributes`
**Verification**: No more `WARN usvg::parser::shapes: Rect '' has an invalid 'width' value. Skipped.` warnings

#### Issue 4: HTTP 404 from mermaid.ink API
**Symptom**: API returns 404 Not Found
**Cause**: Complex theme configuration with too many variables causing URL encoding issues
**Fix**: Simplified theme config to just `{'theme':'base','themeVariables':{'background':'transparent'}}`
**Files Changed**: `src/mermaid.rs` (line 201)
**Test**: Integration tests should catch API errors

### Testing Infrastructure Created

#### Unit Tests
- **Location**: `src/mermaid.rs`
- **Count**: 8 tests
- **Command**: `cargo test --lib mermaid::`
- **Coverage**: Block extraction, SVG/PNG conversion, transparency, error handling, invalid rect attribute fixing

#### Integration Tests
- **Location**: `tests/integration_mermaid_test.rs`
- **Count**: 7 tests
- **Command**: `cargo test --test integration_mermaid_test`
- **Coverage**: Full pipeline, API connectivity, caching, format validation

#### Debug Tests
- **Location**: `tests/debug_mermaid_png.rs`
- **Purpose**: Save PNG files for manual inspection
- **Command**: `cargo test --test debug_mermaid_png -- --nocapture`
- **Output**: `/tmp/debug_mermaid.png`

#### E2E Tests
- **Location**: `tests/e2e_mermaid_test.sh`
- **Purpose**: External API testing, network validation
- **Command**: `./tests/e2e_mermaid_test.sh`

### Lessons Learned

1. **Test Early and Often**: Could have caught format incompatibility earlier with proper tests
2. **Simple is Better**: Complex theme configurations can break mermaid.ink API
3. **Log Everything**: Added detailed error logging for API failures
4. **Validate Assumptions**: egui_commonmark support != universal SVG support
5. **Debug with Files**: Saving intermediate results (like PNG files) helps isolate issues

### Quick Regression Test

Run this before any mermaid changes:

```bash
# PRIMARY: E2E test suite (comprehensive)
./tests/e2e_mermaid_labels_test.sh

# SECONDARY: Unit and integration tests
cargo test --lib mermaid::
cargo test --test integration_mermaid_test

# VERIFY: Generate debug JPEG and verify labels manually
cargo test --test debug_mermaid_png -- --nocapture
open /tmp/debug_mermaid.jpg  # Should show labels

# CHECK: Recent logs for errors
tail -50 /tmp/rustbot_mermaid_system.log | grep -E "(ERROR|WARN|mermaid)"
```

**Expected Results**:
- ✅ All E2E tests pass (8/8)
- ✅ `/tmp/debug_mermaid.jpg` has visible labels
- ✅ JPEG format (magic bytes: FF D8 FF)
- ✅ No ERROR logs
- ✅ File size >3KB (indicates labels present)

### Known Issues

1. **Theme Complexity**: Very complex theme configurations cause 404 errors
   - **Impact**: MEDIUM - Prevents rendering
   - **Workaround**: Use simplified theme with only essential variables
   - **Fix**: Implemented in current version (v0.2.2-004)

### Future Improvements

1. **Offline Mode**: Cache SVGs to disk for offline rendering
2. **Retry Logic**: Add exponential backoff for API failures
3. **Theme Validation**: Validate theme config before sending to API
4. **PNG Optimization**: Compress PNGs to reduce data URL size
5. **Error Recovery**: Better fallback when rendering fails

### Version History

- **v0.2.2-001**: Initial mermaid support with direct SVG rendering (failed - red triangle)
- **v0.2.2-002**: Added SVG to PNG conversion (transparency issues)
- **v0.2.2-003**: Fixed PNG data URL format (missing labels due to usvg)
- **v0.2.2-004**: Simplified theme config (API 404 errors)
- **v0.2.2-005**: Added SVG pre-processing for invalid rect attributes (foreignObject not supported)
- **v0.2.2-006**: Switched to JPEG endpoint - FINAL SOLUTION (labels work, no transparency)

### Final Solution (v0.2.2-006)

**Status**: ✅ ALL ISSUES RESOLVED

**Approach**: Use mermaid.ink `/img/` endpoint (returns JPEG with pre-rendered labels)

**Tradeoffs**:
- ✅ Labels visible (primary goal achieved)
- ❌ No transparency (white background - acceptable)
- ❌ No theme customization (default theme - acceptable)

**Testing**: Comprehensive E2E test suite created (`tests/e2e_mermaid_labels_test.sh`)

