// Icon processing utilities for Rustbot window icon
// Icon is now pre-processed at compile time by build.rs for optimal performance

/// Create the window icon from pre-processed compile-time data
/// The icon is processed during compilation (see build.rs) with:
/// - Auto-crop to remove transparent borders
/// - Resize to 128x128 with Lanczos3 filtering
/// - Rounded corners applied (macOS style: 22.37% corner radius)
pub fn create_window_icon() -> egui::IconData {
    // Load pre-processed icon data embedded at compile time
    // Format: width (4 bytes) + height (4 bytes) + rgba data
    const ICON_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/processed_icon.bin"));

    // Parse width and height from first 8 bytes
    let width = u32::from_le_bytes([ICON_DATA[0], ICON_DATA[1], ICON_DATA[2], ICON_DATA[3]]);
    let height = u32::from_le_bytes([ICON_DATA[4], ICON_DATA[5], ICON_DATA[6], ICON_DATA[7]]);

    // Extract RGBA data (remaining bytes)
    let rgba = ICON_DATA[8..].to_vec();

    egui::IconData {
        rgba,
        width,
        height,
    }
}

// Original runtime processing functions kept for reference/debugging
// These are no longer used in production but document the processing logic
#[cfg(any(test, feature = "runtime-icon-processing"))]
mod runtime_processing {
    /// Find the bounds of actual content in an image (non-transparent pixels)
    /// Returns (x, y, width, height) of the content area with padding
    fn find_content_bounds(img: &image::RgbaImage) -> (u32, u32, u32, u32) {
        let (width, height) = img.dimensions();

        let mut min_x = width;
        let mut max_x = 0;
        let mut min_y = height;
        let mut max_y = 0;

        // Scan for non-transparent pixels (alpha > threshold)
        let alpha_threshold = 10; // Consider pixels with alpha > 10 as content

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                if pixel[3] > alpha_threshold {
                    // Alpha channel
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }

        // If no content found, return full image bounds
        if min_x > max_x || min_y > max_y {
            return (0, 0, width, height);
        }

        // Add small padding (5% of content size)
        let content_width = max_x - min_x + 1;
        let content_height = max_y - min_y + 1;
        let padding = ((content_width.max(content_height) as f32) * 0.05) as u32;

        let crop_x = min_x.saturating_sub(padding);
        let crop_y = min_y.saturating_sub(padding);
        let crop_width = (max_x - min_x + 1 + padding * 2).min(width - crop_x);
        let crop_height = (max_y - min_y + 1 + padding * 2).min(height - crop_y);

        (crop_x, crop_y, crop_width, crop_height)
    }

    /// Original runtime icon processing (now done at compile time)
    pub fn create_window_icon_runtime() -> egui::IconData {
        // Load the Rust-themed icon PNG
        let icon_bytes = include_bytes!("../../assets/rustbot-icon-rust.png");

        // Decode the PNG image
        let mut img = image::load_from_memory(icon_bytes)
            .expect("Failed to load icon PNG")
            .to_rgba8();

        // Auto-crop to remove transparent borders and whitespace
        let (crop_x, crop_y, crop_width, crop_height) = find_content_bounds(&img);

        // Crop to content bounds
        let cropped =
            image::imageops::crop(&mut img, crop_x, crop_y, crop_width, crop_height).to_image();

        // Now resize to 128x128 (standard size, but content fills more of it)
        let img =
            image::imageops::resize(&cropped, 128, 128, image::imageops::FilterType::Lanczos3);

        let (width, height) = img.dimensions();
        let mut rgba = img.into_raw();

        // Apply rounded corners (macOS style: 22.37% corner radius)
        // For 128x128 icon, this means ~28.6px corner radius
        let corner_radius = (width as f32 * 0.2237) as u32;

        // Apply rounded mask
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;

                // Calculate distance from corners
                let dx = if x < corner_radius {
                    corner_radius - x
                } else if x >= width - corner_radius {
                    x - (width - corner_radius - 1)
                } else {
                    0
                };

                let dy = if y < corner_radius {
                    corner_radius - y
                } else if y >= height - corner_radius {
                    y - (height - corner_radius - 1)
                } else {
                    0
                };

                // If in corner area, check if outside rounded corner
                if dx > 0 && dy > 0 {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    if distance > corner_radius as f32 {
                        // Outside rounded corner - make transparent
                        rgba[idx + 3] = 0; // Set alpha to 0
                    } else if distance > (corner_radius as f32 - 1.5) {
                        // Anti-aliasing at edge
                        let alpha = 1.0 - (distance - (corner_radius as f32 - 1.5)) / 1.5;
                        rgba[idx + 3] = ((rgba[idx + 3] as f32) * alpha) as u8;
                    }
                }
            }
        }

        egui::IconData {
            rgba,
            width,
            height,
        }
    }
}
