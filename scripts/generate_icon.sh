#!/bin/bash
# Generate macOS application icon with rounded corners from source PNG
# Requires: ImageMagick (brew install imagemagick)

set -e

SOURCE_ICON="assets/rustbot-icon-rust.png"
ICONSET_DIR="assets/rustbot.iconset"

# Check if source exists
if [ ! -f "$SOURCE_ICON" ]; then
    echo "Error: Source icon not found at $SOURCE_ICON"
    exit 1
fi

# Check for ImageMagick
if ! command -v magick &> /dev/null && ! command -v convert &> /dev/null; then
    echo "Error: ImageMagick not found. Install with: brew install imagemagick"
    exit 1
fi

# Use 'magick' if available (ImageMagick 7), otherwise 'convert' (ImageMagick 6)
if command -v magick &> /dev/null; then
    MAGICK_CMD="magick"
else
    MAGICK_CMD="convert"
fi

echo "Creating iconset directory..."
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Function to create rounded icon
create_rounded_icon() {
    local size=$1
    local output=$2
    local corner_radius=$((size / 5))  # 20% corner radius (macOS style)

    echo "  Generating ${size}x${size} icon with rounded corners..."

    # Create rounded rectangle mask
    $MAGICK_CMD -size ${size}x${size} xc:none \
        -draw "roundrectangle 0,0 ${size},${size} ${corner_radius},${corner_radius}" \
        /tmp/mask_${size}.png

    # Resize source and apply rounded mask
    $MAGICK_CMD "$SOURCE_ICON" \
        -resize ${size}x${size} \
        /tmp/mask_${size}.png \
        -compose DstIn \
        -composite \
        "$output"

    # Clean up temp mask
    rm -f /tmp/mask_${size}.png
}

# Generate all required icon sizes for macOS
echo "Generating icon sizes..."

# Standard resolutions
create_rounded_icon 16 "$ICONSET_DIR/icon_16x16.png"
create_rounded_icon 32 "$ICONSET_DIR/icon_16x16@2x.png"
create_rounded_icon 32 "$ICONSET_DIR/icon_32x32.png"
create_rounded_icon 64 "$ICONSET_DIR/icon_32x32@2x.png"
create_rounded_icon 128 "$ICONSET_DIR/icon_128x128.png"
create_rounded_icon 256 "$ICONSET_DIR/icon_128x128@2x.png"
create_rounded_icon 256 "$ICONSET_DIR/icon_256x256.png"
create_rounded_icon 512 "$ICONSET_DIR/icon_256x256@2x.png"
create_rounded_icon 512 "$ICONSET_DIR/icon_512x512.png"
create_rounded_icon 1024 "$ICONSET_DIR/icon_512x512@2x.png"

echo "Converting iconset to .icns..."
iconutil -c icns "$ICONSET_DIR" -o assets/rustbot.icns

echo "Cleaning up iconset directory..."
rm -rf "$ICONSET_DIR"

echo ""
echo "✅ Icon generated successfully!"
echo "   Output: assets/rustbot.icns"
echo ""
echo "To use this icon in your app:"
echo "  1. Update Cargo.toml bundle section to reference 'assets/rustbot.icns'"
echo "  2. Rebuild your application"
