# Rustbot Icon Assets

This directory contains the icon assets for the Rustbot application.

## Files

- **rustbot-icon.png** - Source icon (1024x1024 PNG with transparency)
- **rustbot.icns** - macOS application icon with rounded corners (generated)

## Icon Generation

The macOS application icon is generated from the source PNG using the script:
```bash
./scripts/generate_icon.sh
```

### What the script does:
1. Takes the source `rustbot-icon.png` (1024x1024)
2. Applies rounded corners (20% corner radius - macOS style)
3. Generates all required icon sizes:
   - 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024
   - Both standard and @2x retina versions
4. Converts to `.icns` format using macOS `iconutil`

### Requirements:
- **ImageMagick**: `brew install imagemagick`
- **iconutil**: Built into macOS

## Icon Specifications

### macOS Icon Style
- **Corner Radius**: 20% of icon size (e.g., 204px for 1024px icon)
- **Format**: ICNS (Apple Icon Image format)
- **Sizes**: 10 different resolutions from 16x16 to 1024x1024
- **Transparency**: Preserved from source PNG

### Design Guidelines
The icon follows macOS Human Interface Guidelines:
- Rounded corners for modern macOS appearance
- Transparent background
- High resolution support (up to 1024x1024 for retina displays)
- Consistent appearance across all sizes

## Usage

The icon is automatically used when building the macOS app bundle through the configuration in `Cargo.toml`:

```toml
[package.metadata.bundle]
icon = ["assets/rustbot.icns"]
```

## Regenerating the Icon

If you update `rustbot-icon.png`, regenerate the `.icns` file:

```bash
./scripts/generate_icon.sh
```

The script will:
- ✅ Apply rounded corners
- ✅ Generate all required sizes
- ✅ Create optimized `.icns` file
- ✅ Clean up temporary files

## Technical Details

### Icon Sizes in .icns
- icon_16x16.png (16x16)
- icon_16x16@2x.png (32x32)
- icon_32x32.png (32x32)
- icon_32x32@2x.png (64x64)
- icon_128x128.png (128x128)
- icon_128x128@2x.png (256x256)
- icon_256x256.png (256x256)
- icon_256x256@2x.png (512x512)
- icon_512x512.png (512x512)
- icon_512x512@2x.png (1024x1024)

### Rounded Corner Implementation
The rounded corners are created using ImageMagick's `roundrectangle` drawing:
- Creates a mask with rounded corners
- Applies mask using DstIn composition
- Preserves transparency and anti-aliasing
- Corner radius scales proportionally with icon size
