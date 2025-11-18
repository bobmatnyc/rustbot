// Debug test to save PNG and inspect it manually
use rustbot::mermaid::MermaidRenderer;
use std::fs;

#[tokio::test]
async fn save_rendered_png_for_inspection() {
    let mut renderer = MermaidRenderer::new();

    let mermaid_code = r#"graph TD
    A[Start] --> B[Process]
    B --> C[End]"#;

    let result = renderer.render_to_png(mermaid_code).await;

    if let Err(ref e) = result {
        println!("ERROR: {:?}", e);
    }

    assert!(
        result.is_ok(),
        "Should render successfully: {:?}",
        result.as_ref().err()
    );

    let image_bytes = result.unwrap();

    // Save to /tmp for manual inspection (will be JPEG from mermaid.ink/img/)
    fs::write("/tmp/debug_mermaid.jpg", &image_bytes).expect("Should be able to write file");

    println!(
        "✓ Saved image to /tmp/debug_mermaid.jpg ({} bytes)",
        image_bytes.len()
    );
    println!("You can open this file to see if labels are present");

    // Verify JPEG signature (0xFFD8FF)
    assert_eq!(
        &image_bytes[0..3],
        &[0xFF, 0xD8, 0xFF],
        "Should have JPEG signature"
    );
}
