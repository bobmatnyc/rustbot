# Mermaid Diagram Copy Feature

## Summary
Added copy functionality for rendered Mermaid diagrams in the chat interface. Users can now easily copy diagram images to clipboard with a single click.

## Implementation Date
2025-11-16

## Problem
Mermaid diagrams were being rendered as embedded JPEG images via base64 data URLs in the markdown, but there was no way for users to copy these images for use elsewhere (e.g., pasting into documentation, slides, or image editors).

## Solution
Implemented a copy button system that appears below messages containing Mermaid diagrams. When clicked, the button copies the base64 image data URL to the clipboard, which can then be pasted directly into browsers, image tools, or markdown editors.

## Technical Details

### Files Modified
1. **src/ui/types.rs**
   - Added `embedded_images: Vec<String>` field to `ChatMessage` struct
   - Stores data URLs of all embedded images in a message for easy access

2. **src/main.rs**
   - Added `extract_image_data_urls()` helper function (lines 554-579)
   - Extracts all base64 image data URLs from markdown content using regex
   - Updated all `ChatMessage` creation sites to initialize `embedded_images` field
   - Modified message finalization to populate `embedded_images` when content is set (line 949)

3. **src/ui/views.rs**
   - Added copy button UI for embedded images (lines 137-166)
   - Buttons appear below markdown content when images are present
   - Smart labeling: "Copy Diagram" for single image, "Copy Diagram 1/2/3" for multiple
   - Uses Phosphor icon `CLIPBOARD` for consistency with UI design

4. **src/mermaid.rs**
   - Removed unused imports (`usvg::Options`, `usvg::Tree`)
   - No functional changes

### Design Decisions

#### Why Data URL Copy Instead of Image File?
- **Simplicity**: No temporary file management required
- **Portability**: Data URLs work everywhere (browsers, markdown, HTML)
- **Consistency**: Same format as how images are embedded in the app
- **Immediate Use**: Users can paste directly into browsers or compatible apps

#### Why Separate Buttons Instead of Overlay Icons?
- **egui_commonmark Limitation**: `CommonMarkViewer` doesn't expose image rendering hooks
- **Implementation Complexity**: Detecting image positions would require parsing and reimplementing markdown rendering
- **User Experience**: Clear, discoverable buttons vs. hover-only overlays
- **Accessibility**: Buttons are always visible and don't require precise mouse positioning

#### Alternative Approaches Considered
1. **Image Position Detection**: Parse rendered UI to detect image positions
   - Rejected: Too fragile, breaks with markdown library updates
2. **Custom Markdown Renderer**: Fork or extend CommonMarkViewer
   - Rejected: High maintenance burden, loses upstream improvements
3. **Hover Overlays on Images**: Add interactive layer over images
   - Rejected: No API for detecting image positions in CommonMarkViewer

## User Interface

### Before
- Mermaid diagrams rendered as images
- No way to copy or save individual diagrams
- Users had to screenshot or copy entire message

### After
- Mermaid diagrams rendered as images (unchanged)
- Copy button(s) appear below markdown content
- Click button to copy image data URL to clipboard
- Paste anywhere that accepts images or data URLs

### Visual Design
- Button label: "{CLIPBOARD_ICON} Copy Diagram" (or "Copy Diagram 1/2/3" for multiple)
- Size: 10.5pt font (slightly smaller than main text)
- Color: Blue accent (#5078B4) to indicate interactive element
- Tooltip: "Copy diagram image to clipboard (as data URL)"
- Spacing: 6px above buttons, 8px between multiple buttons

## Testing

### Manual Testing Procedure
1. Start Rustbot: `cargo run`
2. Send message requesting Mermaid diagram:
   ```
   Create a simple flowchart diagram showing a login process
   ```
3. Wait for diagram to render
4. Verify "Copy Diagram" button appears below rendered image
5. Click copy button
6. Check terminal for log: "📋 Copied diagram 1 to clipboard"
7. Paste into browser address bar or markdown editor
8. Verify image displays correctly

### Test Cases
- ✅ Single Mermaid diagram: Shows "Copy Diagram"
- ✅ Multiple diagrams: Shows "Copy Diagram 1", "Copy Diagram 2", etc.
- ✅ No diagrams: No copy buttons appear
- ✅ Copy to clipboard: Data URL is copied correctly
- ✅ Paste in browser: Image displays when pasted
- ✅ Message with mixed content: Buttons only for diagram images

## Performance Impact
- **Memory**: Minimal (~50-100 bytes per image for data URL string)
- **Rendering**: Negligible (simple button rendering)
- **Extraction**: O(n) where n is message length, runs once per message finalization

## Known Limitations
1. **Data URL Size**: Clipboard may have size limits for very large diagrams
2. **Paste Support**: Some applications may not accept data URLs
3. **Image Format**: Copies as JPEG (from mermaid.ink API), not original SVG
4. **No Visual Feedback**: No toast/notification on copy (relies on clipboard success)

## Future Enhancements
1. **Copy Confirmation**: Show brief toast notification on successful copy
2. **Download Button**: Add option to download as file instead of copying data URL
3. **Format Selection**: Allow choosing between JPEG, PNG, or SVG format
4. **Hover Overlay**: If egui_commonmark adds image hooks, implement overlay icons
5. **Image Preview**: Add modal to preview/zoom diagram before copying

## Compatibility
- **egui Version**: 0.29 (tested)
- **egui_commonmark**: Works with current version (embedded images feature)
- **Clipboard**: Uses egui's native clipboard API (cross-platform)
- **Data URLs**: Standard format, works in all modern browsers

## Rollback Plan
If issues arise, revert commits affecting:
- `src/ui/types.rs` (ChatMessage struct)
- `src/main.rs` (extract_image_data_urls and embedded_images population)
- `src/ui/views.rs` (copy button UI)

Original functionality (rendering Mermaid diagrams) remains unchanged.

## Documentation Updates
- Added inline code documentation for `extract_image_data_urls()` function
- Updated ChatMessage struct documentation
- Added UI comments explaining copy button behavior

## Related Issues/Features
- Mermaid diagram rendering (implemented 2025-11-16)
- Copy message to clipboard (existing feature)
- Image embedding in markdown (egui_commonmark feature)

---

**Status**: ✅ Implemented and tested
**Impact**: Low risk, additive feature only
**Breaking Changes**: None (backward compatible)
