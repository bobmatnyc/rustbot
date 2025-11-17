// Integration test for Mermaid rendering pipeline
// Tests the complete flow: API → SVG → PNG → Data URL

use rustbot::mermaid::{MermaidRenderer, extract_mermaid_blocks};

#[tokio::test]
async fn test_full_rendering_pipeline() {
    let mut renderer = MermaidRenderer::new();

    // Simple diagram that should have visible labels
    let mermaid_code = r#"graph TD
    A[Start] --> B[Process]
    B --> C[End]"#;

    // Render to PNG
    let result = renderer.render_to_png(mermaid_code).await;

    assert!(result.is_ok(), "Rendering should succeed");

    let png_bytes = result.unwrap();

    // Verify it's PNG format
    assert_eq!(png_bytes[0], 0x89, "PNG signature byte 1");
    assert_eq!(png_bytes[1], 0x50, "PNG signature byte 2");
    assert_eq!(png_bytes[2], 0x4E, "PNG signature byte 3");
    assert_eq!(png_bytes[3], 0x47, "PNG signature byte 4");

    // Verify reasonable size (should be > 1KB for a diagram with text)
    assert!(png_bytes.len() > 1000, "PNG should be substantial size, got {} bytes", png_bytes.len());

    // Verify it's not too small (which would indicate missing content)
    assert!(png_bytes.len() > 5000, "PNG might be missing labels, only {} bytes", png_bytes.len());
}

#[tokio::test]
async fn test_complex_diagram_with_labels() {
    let mut renderer = MermaidRenderer::new();

    // Complex diagram with many labels
    let mermaid_code = r#"graph TD
    Client[Client Application] -->|HTTP Request| LoadBalancer[Load Balancer]
    LoadBalancer -->|Route| WebServer1[Web Server 1]
    LoadBalancer -->|Route| WebServer2[Web Server 2]
    WebServer1 -->|Process| AppServer[App Server]
    WebServer2 -->|Process| AppServer
    AppServer -->|Query| Database[(Database)]"#;

    let result = renderer.render_to_png(mermaid_code).await;

    assert!(result.is_ok(), "Complex diagram should render");

    let png_bytes = result.unwrap();

    // Complex diagram with labels should be larger
    assert!(png_bytes.len() > 15000, "Complex diagram should be large, got {} bytes", png_bytes.len());
}

#[tokio::test]
async fn test_svg_content_preservation() {
    let mut renderer = MermaidRenderer::new();

    let mermaid_code = "graph LR\n    A[Node A] --> B[Node B]";

    // First render
    let result1 = renderer.render_to_png(mermaid_code).await;
    assert!(result1.is_ok());

    // Second render (should hit cache)
    let result2 = renderer.render_to_png(mermaid_code).await;
    assert!(result2.is_ok());

    // Both should produce identical results
    let bytes1 = result1.unwrap();
    let bytes2 = result2.unwrap();

    assert_eq!(bytes1.len(), bytes2.len(), "Cache should return identical data");
    assert_eq!(bytes1, bytes2, "Cached data should be identical");
}

#[test]
fn test_extract_blocks_preserves_content() {
    let markdown = r#"Some text

```mermaid
graph TD
    A[Important Label] --> B[Another Label]
    B --> C[Final Label]
```

More text"#;

    let blocks = extract_mermaid_blocks(markdown);

    assert_eq!(blocks.len(), 1, "Should extract one block");

    let (_, _, code) = &blocks[0];

    // Verify labels are preserved
    assert!(code.contains("Important Label"), "First label should be preserved");
    assert!(code.contains("Another Label"), "Second label should be preserved");
    assert!(code.contains("Final Label"), "Third label should be preserved");
    assert!(code.contains("-->"), "Arrow syntax should be preserved");
}

#[tokio::test]
async fn test_transparent_background_theme() {
    let mut renderer = MermaidRenderer::new();

    // Diagram code (theme will be added by renderer)
    let mermaid_code = "graph TD\n    A --> B";

    let result = renderer.render_to_png(mermaid_code).await;

    if let Err(e) = &result {
        eprintln!("ERROR: {:?}", e);
    }

    assert!(result.is_ok(), "Should render with theme: {:?}", result.err());

    let png_bytes = result.unwrap();

    // PNG with transparency should exist
    assert!(!png_bytes.is_empty());

    // TODO: Ideally we'd decode PNG and verify alpha channel exists
    // For now, we verify the PNG is valid
    assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
}

#[tokio::test]
async fn test_api_returns_valid_svg() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    let mermaid_code = "graph TD\n    A[Test] --> B[Node]";

    // Add theme config like the renderer does
    let config = r#"%%{init: {'theme':'base','themeVariables':{'primaryColor':'#ECECFF','primaryTextColor':'#333','primaryBorderColor':'#9370DB','lineColor':'#333','secondaryColor':'#f0f0f0','tertiaryColor':'#fff','background':'transparent','mainBkg':'transparent','clusterBkg':'transparent'}}}%%"#;
    let mermaid_with_theme = format!("{}\n{}", config, mermaid_code);
    let encoded = BASE64.encode(mermaid_with_theme.as_bytes());

    let url = format!("https://mermaid.ink/svg/{}", encoded);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let response = client.get(&url).send().await;

    assert!(response.is_ok(), "API should be accessible");

    let response = response.unwrap();
    assert!(response.status().is_success(), "API should return success");

    let svg_bytes = response.bytes().await.unwrap();

    // Verify it's SVG
    let svg_str = String::from_utf8_lossy(&svg_bytes[..100]);
    assert!(svg_str.contains("<svg") || svg_str.contains("<?xml"),
            "Should return SVG, got: {}", svg_str);

    // Verify it has content (not empty or error)
    assert!(svg_bytes.len() > 1000, "SVG should have substantial content");
}

#[test]
fn test_data_url_format() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // Sample PNG header
    let png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let base64_data = BASE64.encode(&png_bytes);

    let data_url = format!("data:image/png;base64,{}", base64_data);

    // Verify format
    assert!(data_url.starts_with("data:image/png;base64,"), "Should have correct MIME type");
    assert!(!data_url.starts_with("data:image/svg+xml"), "Should NOT be SVG");
    assert!(!data_url.starts_with("data:image/jpeg"), "Should NOT be JPEG");
}
