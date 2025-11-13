# Compile-Time Icon Processing

## Quick Reference

Rustbot uses **compile-time icon processing** to eliminate runtime overhead.

## How It Works

```
Build Time (once):
  assets/rustbot-icon-rust.png
    ↓ (build.rs)
  [Crop, Resize, Round Corners]
    ↓
  OUT_DIR/processed_icon.bin

Runtime (every app start):
  include_bytes!() → IconData
  (< 1 microsecond)
```

## Key Files

| File | Purpose |
|------|---------|
| `build.rs` | Processes icon during compilation |
| `src/ui/icon.rs` | Loads pre-processed icon data |
| `assets/rustbot-icon-rust.png` | Source icon (1MB PNG) |
| `OUT_DIR/processed_icon.bin` | Processed icon (64KB binary) |

## Build Triggers

Icon processing runs when:
- ✅ `assets/rustbot-icon-rust.png` changes
- ✅ `build.rs` changes
- ✅ `cargo clean` is run

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Build-time processing | ~50-100ms | Once per icon change |
| Runtime loading | ~0.1-0.5μs | Essentially instant |
| Old runtime method | ~15-50ms | 100-500x slower |

## Usage

### Normal Build
```bash
cargo build
# Icon automatically processed
```

### Clean Build
```bash
cargo clean
cargo build
# Icon reprocessed from scratch
```

### Update Icon
```bash
# 1. Replace assets/rustbot-icon-rust.png
# 2. Build will automatically reprocess
cargo build
```

## Icon Processing Details

**Input**: `assets/rustbot-icon-rust.png` (any size PNG)

**Processing**:
1. Auto-crop transparent borders (+5% padding)
2. Resize to 128×128 (Lanczos3 filter)
3. Apply rounded corners (22.37% radius, macOS style)
4. Export as binary RGBA data

**Output**: 65,544 byte binary file
- 4 bytes: width (128)
- 4 bytes: height (128)
- 65,536 bytes: RGBA pixel data

## Debugging

### Enable Runtime Processing
```bash
cargo build --features runtime-icon-processing
```

### Verify Icon Data
```bash
# Check file exists
ls -lh target/*/build/rustbot-*/out/processed_icon.bin

# Should show: 64K (65,544 bytes)
```

### Check Build Messages
```bash
cargo build 2>&1 | grep "Processed icon"
# Expected: warning: rustbot@0.0.2: Processed icon: 128x128 (65536 bytes)
```

## Technical Details

### Binary Format
```
Offset | Size | Content
-------|------|------------------
0      | 4    | Width (u32 LE)
4      | 4    | Height (u32 LE)
8      | N    | RGBA data (w×h×4)
```

### Code Path
```rust
// Compile-time: Icon data embedded in binary
const ICON_DATA: &[u8] = include_bytes!(
    concat!(env!("OUT_DIR"), "/processed_icon.bin")
);

// Runtime: Parse and return
pub fn create_window_icon() -> egui::IconData {
    let width = u32::from_le_bytes([...]);
    let height = u32::from_le_bytes([...]);
    let rgba = ICON_DATA[8..].to_vec();
    egui::IconData { rgba, width, height }
}
```

## Benefits

✅ **100-500x faster** icon loading
✅ **Simpler** runtime code (26 lines vs 117)
✅ **Smaller** runtime memory footprint
✅ **Same** visual quality
✅ **No** runtime image processing dependencies

## See Also

- `BUILD_OPTIMIZATION_SUMMARY.md` - Full implementation details
- `ICON_OPTIMIZATION.md` - Performance analysis
- `build.rs` - Build script source code

---

*Icon processing moved to compile time on 2025-11-13*
