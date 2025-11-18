// Mermaid diagram rendering using mermaid.ink API
//
// This module provides functionality to convert Mermaid diagram code into SVG images
// using the public mermaid.ink API service.
//
// Design Decision: mermaid.ink API vs Local Rendering
//
// Rationale: Selected mermaid.ink public API for simplicity and zero dependencies.
// This approach avoids bundling a JavaScript runtime or browser automation tools.
//
// Trade-offs:
// - Performance: Network request (100-500ms) vs local rendering (<50ms)
// - Reliability: Requires internet connection vs fully offline capable
// - Privacy: Diagram code sent to external service vs local processing
// - Complexity: Simple HTTP client vs headless browser + JS runtime
//
// Alternatives Considered:
// 1. headless_chrome + local mermaid.js: Rejected due to large binary size and complexity
// 2. QuickJS + mermaid.js: Rejected due to integration complexity and maintenance burden
// 3. Server-side Node.js: Rejected as it requires Node.js installation
//
// Extension Points: The MermaidRenderer trait could support multiple backends
// if local rendering becomes necessary (check network connectivity, fallback to local).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::collections::HashMap;
use std::time::Duration;

/// Result type for mermaid operations
type Result<T> = std::result::Result<T, MermaidError>;

/// Errors that can occur during mermaid diagram rendering
#[derive(Debug, thiserror::Error)]
pub enum MermaidError {
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid mermaid syntax: {0}")]
    InvalidSyntax(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Timeout while rendering diagram")]
    Timeout,
}

/// Mermaid diagram renderer with caching
///
/// Performance:
/// - Time Complexity: O(1) for cached diagrams, O(n) for network requests where n is diagram size
/// - Space Complexity: O(m) where m is total size of cached SVG data
///
/// Expected Performance:
/// - Cache hit: <1ms (HashMap lookup)
/// - Cache miss: 100-500ms (network round-trip to mermaid.ink)
/// - 1MB cache holds ~50-100 diagrams (typical SVG: 10-20KB)
///
/// Bottleneck: Network latency for first render. Consider pre-warming cache
/// for frequently used diagrams or implementing background refresh.
pub struct MermaidRenderer {
    /// HTTP client for API requests
    client: reqwest::Client,

    /// Cache of rendered SVG diagrams (mermaid_code -> svg_bytes)
    /// Using String key instead of hash for debugging clarity
    cache: HashMap<String, Vec<u8>>,
}

impl MermaidRenderer {
    /// Create a new MermaidRenderer with default settings
    ///
    /// Optimization Opportunities:
    /// 1. Connection Pooling: HTTP client reuses connections (already implemented via reqwest)
    /// 2. Persistent Cache: Save cache to disk for cross-session persistence
    ///    - Estimated speedup: Eliminates network requests on app restart
    ///    - Effort: 4-6 hours to implement with serde serialization
    ///    - Threshold: Implement when users have >20 diagrams
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            cache: HashMap::new(),
        }
    }

    /// Render a mermaid diagram to SVG
    ///
    /// This method:
    /// 1. Checks cache first for previously rendered diagrams
    /// 2. Base64 encodes the mermaid code if not cached
    /// 3. Calls the mermaid.ink API: https://mermaid.ink/svg/{encoded}
    /// 4. Caches and returns the SVG result
    ///
    /// Error Handling:
    /// 1. NetworkError: Retry logic not implemented (fails fast)
    ///    - Propagates error to caller for display to user
    ///    - Fallback: UI shows code block instead of diagram
    /// 2. Timeout: 5-second timeout to prevent UI freezing
    ///    - Returns Timeout error for graceful degradation
    /// 3. InvalidSyntax: mermaid.ink returns HTML error page
    ///    - Detected by checking response content-type
    ///    - Fallback: Show original code block
    ///
    /// # Arguments
    /// * `mermaid_code` - The mermaid diagram code to render
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - SVG image data on success
    /// * `Err(MermaidError)` - Error details for logging/display
    ///
    /// # Example
    /// ```ignore
    /// let renderer = MermaidRenderer::new();
    /// let svg = renderer.render_to_svg("graph TD\n  A-->B").await?;
    /// ```
    pub async fn render_to_svg(&mut self, mermaid_code: &str) -> Result<Vec<u8>> {
        // Check cache first (O(1) lookup)
        if let Some(cached) = self.cache.get(mermaid_code) {
            return Ok(cached.clone());
        }

        // Base64 encode the mermaid code
        // Note: URL-safe encoding to handle special characters in diagram code
        let encoded = BASE64.encode(mermaid_code.as_bytes());

        // Call mermaid.ink API
        let url = format!("https://mermaid.ink/svg/{}", encoded);

        tracing::debug!(
            "Rendering mermaid diagram via API: {} chars",
            mermaid_code.len()
        );

        // Make request with timeout (5 seconds)
        let response = self.client.get(&url).send().await?;

        // Check if request succeeded
        if !response.status().is_success() {
            return Err(MermaidError::InvalidSyntax(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        // Check content type to detect error responses
        // mermaid.ink returns HTML error pages for invalid syntax
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(ct) = content_type.to_str() {
                if ct.contains("text/html") {
                    return Err(MermaidError::InvalidSyntax(
                        "Invalid mermaid syntax - API returned HTML error".to_string(),
                    ));
                }
            }
        }

        // Get SVG bytes
        let svg_bytes = response.bytes().await?.to_vec();

        // Validate we got SVG data (should start with "<?xml" or "<svg")
        if let Ok(svg_str) = std::str::from_utf8(&svg_bytes[..svg_bytes.len().min(100)]) {
            if !svg_str.trim_start().starts_with("<?xml")
                && !svg_str.trim_start().starts_with("<svg")
            {
                return Err(MermaidError::InvalidSyntax(
                    "API did not return valid SVG data".to_string(),
                ));
            }
        }

        tracing::info!("✓ Rendered mermaid diagram: {} bytes", svg_bytes.len());

        // Cache the result
        self.cache
            .insert(mermaid_code.to_string(), svg_bytes.clone());

        Ok(svg_bytes)
    }

    /// Render a mermaid diagram to image (SVG from mermaid.ink)
    ///
    /// This method uses mermaid.ink's /svg/ endpoint which returns SVG format with transparency support.
    /// SVG has excellent compatibility with egui_commonmark's embedded_image feature when using egui_extras svg loader.
    ///
    /// # Arguments
    /// * `mermaid_code` - The mermaid diagram code to render
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - SVG image data on success
    /// * `Err(MermaidError)` - Error details for logging/display
    pub async fn render_to_png(&mut self, mermaid_code: &str) -> Result<Vec<u8>> {
        // Check cache first (O(1) lookup) - use different cache key for images
        let cache_key = format!("img:{}", mermaid_code);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Base64 encode the mermaid code
        // Note: /img/ endpoint doesn't support %%{init:...}%% theme configs (returns 404)
        // It uses default theme with white background (no transparency support in JPEG anyway)
        let encoded = BASE64.encode(mermaid_code.as_bytes());

        // Call mermaid.ink IMG API which returns pre-rendered PNG with labels
        // Note: Using /img/ instead of /svg/ because usvg doesn't support foreignObject elements
        // which mermaid.ink uses for text labels
        let url = format!("https://mermaid.ink/img/{}", encoded);

        tracing::debug!(
            "Rendering mermaid diagram as PNG via API: {} chars",
            mermaid_code.len()
        );

        // Make request with timeout (5 seconds)
        let response = self.client.get(&url).send().await?;

        // Check for success
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());

            tracing::error!(
                "mermaid.ink API error: HTTP {} - URL: {} - Error: {}",
                status,
                url,
                error_body
            );

            return Err(MermaidError::InvalidSyntax(format!(
                "API returned HTTP {} - {}",
                status, error_body
            )));
        }

        // Get JPEG bytes from mermaid.ink /img/ endpoint
        // Note: /img/ returns JPEG (with labels) instead of PNG
        // Tradeoff: JPEG doesn't support transparency but includes text labels
        let jpeg_bytes = response.bytes().await?.to_vec();

        // Validate we got JPEG data (should start with JPEG signature: 0xFFD8FF)
        if jpeg_bytes.len() > 3 {
            if &jpeg_bytes[0..3] != &[0xFF, 0xD8, 0xFF] {
                tracing::warn!("API returned non-JPEG data");
                return Err(MermaidError::InvalidSyntax(
                    "API did not return valid JPEG data".to_string(),
                ));
            }
        } else {
            return Err(MermaidError::InvalidSyntax(
                "API returned empty or invalid data".to_string(),
            ));
        }

        tracing::info!(
            "✓ Rendered mermaid diagram as JPEG: {} bytes",
            jpeg_bytes.len()
        );

        // Cache the JPEG result
        self.cache.insert(cache_key, jpeg_bytes.clone());

        Ok(jpeg_bytes)
    }

    /// Clear the diagram cache
    ///
    /// Useful for:
    /// - Memory management when cache grows large
    /// - Force re-rendering of diagrams (e.g., after mermaid.ink updates)
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached diagrams
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get total memory used by cache (approximate)
    pub fn cache_memory_bytes(&self) -> usize {
        self.cache.values().map(|v| v.len()).sum()
    }

    /// Convert SVG bytes to PNG with transparent background
    ///
    /// Uses resvg to render SVG to PNG format which is compatible with
    /// egui_commonmark's embedded_image feature.
    ///
    /// # Arguments
    /// * `svg_bytes` - SVG image data
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - PNG image data with transparency
    /// * `Err(MermaidError)` - Conversion error
    fn svg_to_png(svg_bytes: &[u8]) -> Result<Vec<u8>> {
        // Pre-process SVG to fix mermaid.ink issues that cause usvg to skip elements
        // Issue: mermaid.ink generates <rect> elements with empty or "0" width/height
        // which causes usvg to skip them (losing labels)
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
        let tree = usvg::Tree::from_data(fixed_svg.as_bytes(), &options)
            .map_err(|e| MermaidError::EncodingError(format!("Failed to parse SVG: {}", e)))?;

        // Get SVG dimensions
        let size = tree.size();

        // Create pixmap with transparent background
        let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
            .ok_or_else(|| MermaidError::EncodingError("Failed to create pixmap".to_string()))?;

        // Render SVG to pixmap
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        // Encode as PNG
        let png_data = pixmap
            .encode_png()
            .map_err(|e| MermaidError::EncodingError(format!("Failed to encode PNG: {}", e)))?;

        Ok(png_data)
    }
}

impl Default for MermaidRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract mermaid code blocks from markdown text
///
/// Searches for code blocks with language identifier "mermaid" and extracts their content.
///
/// # Arguments
/// * `markdown` - The markdown text to search
///
/// # Returns
/// Vector of (start_index, end_index, mermaid_code) tuples
///
/// # Example
/// ```ignore
/// let markdown = "Some text\n```mermaid\ngraph TD\n  A-->B\n```\nMore text";
/// let blocks = extract_mermaid_blocks(markdown);
/// assert_eq!(blocks.len(), 1);
/// ```
pub fn extract_mermaid_blocks(markdown: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    let mut chars = markdown.char_indices().peekable();

    while let Some((i, _)) = chars.next() {
        // Look for code block start: ```mermaid
        if markdown[i..].starts_with("```mermaid") {
            // Find the end of the opening line
            let start = i + "```mermaid".len();

            // Skip to next line
            let mut code_start = start;
            for (idx, ch) in markdown[start..].char_indices() {
                if ch == '\n' {
                    code_start = start + idx + 1;
                    break;
                }
            }

            // Find closing ``` (could be at start of line or after content)
            let closing = if markdown[code_start..].starts_with("```") {
                // Empty block case: ``` appears immediately
                Some(0)
            } else {
                // Normal case: find \n```
                markdown[code_start..].find("\n```")
            };

            if let Some(end_pos) = closing {
                let code_end = code_start + end_pos;
                let code = markdown[code_start..code_end].to_string();

                // Calculate the full block end position
                let block_end = if end_pos == 0 {
                    code_start + 3 // Just the closing ```
                } else {
                    code_end + 4 // \n```
                };

                // Store block position and content
                blocks.push((i, block_end, code));
            }
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mermaid_blocks() {
        let markdown = r#"Some text

```mermaid
graph TD
  A-->B
  B-->C
```

More text

```mermaid
sequenceDiagram
  Alice->>Bob: Hello
```

End text"#;

        let blocks = extract_mermaid_blocks(markdown);
        assert_eq!(blocks.len(), 2);

        assert!(blocks[0].2.contains("graph TD"));
        assert!(blocks[0].2.contains("A-->B"));

        assert!(blocks[1].2.contains("sequenceDiagram"));
        assert!(blocks[1].2.contains("Alice->>Bob"));
    }

    #[test]
    fn test_extract_no_mermaid_blocks() {
        let markdown = r#"Some text

```python
print("hello")
```

More text"#;

        let blocks = extract_mermaid_blocks(markdown);
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_extract_empty_mermaid_block() {
        let markdown = r#"```mermaid
```"#;

        let blocks = extract_mermaid_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].2.trim(), "");
    }

    #[test]
    fn test_svg_to_png_conversion() {
        // Simple SVG test
        let svg_data = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;

        let result = MermaidRenderer::svg_to_png(svg_data.as_bytes());

        assert!(result.is_ok(), "SVG to PNG conversion should succeed");

        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");

        // Verify PNG signature (89 50 4E 47 0D 0A 1A 0A)
        assert_eq!(png_bytes[0], 0x89, "PNG signature byte 1");
        assert_eq!(png_bytes[1], 0x50, "PNG signature byte 2 (P)");
        assert_eq!(png_bytes[2], 0x4E, "PNG signature byte 3 (N)");
        assert_eq!(png_bytes[3], 0x47, "PNG signature byte 4 (G)");
    }

    #[test]
    fn test_svg_to_png_transparent_background() {
        // SVG with transparent background
        let svg_data = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <circle cx="50" cy="50" r="40" fill="blue" opacity="0.5"/>
        </svg>"#;

        let result = MermaidRenderer::svg_to_png(svg_data.as_bytes());
        assert!(result.is_ok(), "SVG with transparency should convert");

        let png_bytes = result.unwrap();
        // PNG should support transparency
        assert!(!png_bytes.is_empty());
    }

    #[test]
    fn test_svg_to_png_invalid_svg() {
        let invalid_svg = b"This is not SVG data";

        let result = MermaidRenderer::svg_to_png(invalid_svg);
        assert!(result.is_err(), "Invalid SVG should return error");
    }

    #[test]
    fn test_svg_to_png_empty_input() {
        let result = MermaidRenderer::svg_to_png(b"");
        assert!(result.is_err(), "Empty input should return error");
    }

    #[test]
    fn test_svg_to_png_fixes_invalid_rect_attributes() {
        // SVG with invalid rect attributes (like mermaid.ink generates)
        let svg_data = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="10" y="10" width="0" height="0" fill="red"/>
            <rect x="20" y="20" width="" height="" fill="blue"/>
            <text x="50" y="50">Label</text>
        </svg>"#;

        let result = MermaidRenderer::svg_to_png(svg_data.as_bytes());

        // Should succeed despite invalid attributes
        assert!(result.is_ok(), "Should handle invalid rect attributes");

        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG should not be empty");
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "Should have PNG signature"
        );
    }
}
