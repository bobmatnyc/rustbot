# Mermaid Rendering - Quick Reference

## Current Implementation

**Endpoint**: `https://mermaid.ink/img/{base64_diagram}`
**Format**: JPEG
**Status**: ✅ Working with labels

## Run Tests Before Changes

```bash
# One-line regression check
./tests/e2e_mermaid_labels_test.sh && echo "✅ Safe to proceed"
```

## Common Issues & Solutions

### Issue: Diagrams not rendering
**Check**:
```bash
tail -20 /tmp/rustbot_mermaid_system.log | grep -E "(ERROR|mermaid)"
```
**Common Causes**:
- API endpoint down (check with `curl https://mermaid.ink/img/...`)
- Invalid mermaid syntax
- Network issues

### Issue: Labels missing
**Check**:
```bash
cargo test --test debug_mermaid_png -- --nocapture
open /tmp/debug_mermaid.jpg
```
**If labels missing**:
- Verify using `/img/` endpoint (NOT `/svg/`)
- Check JPEG format (NOT PNG/SVG)
- Run E2E tests: `./tests/e2e_mermaid_labels_test.sh`

### Issue: White background instead of transparent
**This is expected** - JPEG doesn't support transparency.
**Don't change** to `/svg/` - that will break labels.

## Key Files

| File | Purpose |
|------|---------|
| `src/mermaid.rs` | Core rendering logic |
| `src/main.rs` | Preprocessing & data URL generation |
| `tests/e2e_mermaid_labels_test.sh` | Primary regression test |
| `tests/debug_mermaid_png.rs` | Visual verification test |
| `docs/MERMAID_FINAL_SOLUTION.md` | Full technical details |

## Critical Constants

```rust
// src/mermaid.rs
const ENDPOINT: &str = "https://mermaid.ink/img/";  // DO NOT change to /svg/
const FORMAT: &str = "JPEG";                         // DO NOT change to PNG
const THEME_CONFIG: Option<&str> = None;            // DO NOT add theme config
```

## Testing Checklist

Before deploying Mermaid changes:

- [ ] E2E tests pass: `./tests/e2e_mermaid_labels_test.sh`
- [ ] Unit tests pass: `cargo test --lib mermaid::`
- [ ] Debug test generates valid JPEG: `cargo test --test debug_mermaid_png`
- [ ] Visual verification: `open /tmp/debug_mermaid.jpg` (labels visible?)
- [ ] No errors in logs: `tail -50 /tmp/rustbot_mermaid_system.log`

## Don't Break This!

❌ **Never change** endpoint from `/img/` to `/svg/` (breaks labels)
❌ **Never add** theme configuration (breaks API)
❌ **Never change** format from JPEG to PNG (breaks rendering)
❌ **Never remove** E2E tests (breaks regression detection)

✅ **Always run** E2E tests before committing changes
✅ **Always verify** labels are visible after changes
✅ **Always check** logs for errors

## Emergency Rollback

If mermaid rendering breaks:

```bash
# Check git history for last working version
git log --oneline -- src/mermaid.rs | head -5

# Rollback to last working commit
git checkout <commit-hash> -- src/mermaid.rs src/main.rs

# Rebuild and verify
cargo build
./tests/e2e_mermaid_labels_test.sh
```

## Contact/References

- **Issue History**: `docs/MERMAID_REGRESSION_LOG.md`
- **Full Solution**: `docs/MERMAID_FINAL_SOLUTION.md`
- **API Docs**: https://mermaid.ink (unofficial)
- **Mermaid Syntax**: https://mermaid.js.org/
