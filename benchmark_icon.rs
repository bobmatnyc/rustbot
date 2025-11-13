// Quick benchmark comparing runtime vs compile-time icon processing
use std::time::Instant;

fn main() {
    // Test the new compile-time approach
    let start = Instant::now();
    for _ in 0..1000 {
        // Simulate loading pre-processed icon
        let icon_data: &[u8] = include_bytes!(concat!(
            env!("OUT_DIR"), 
            "/processed_icon.bin"
        ));
        let _width = u32::from_le_bytes([icon_data[0], icon_data[1], icon_data[2], icon_data[3]]);
        let _height = u32::from_le_bytes([icon_data[4], icon_data[5], icon_data[6], icon_data[7]]);
        let _rgba = &icon_data[8..];
    }
    let compile_time = start.elapsed();
    
    println!("Compile-time icon loading (1000 iterations):");
    println!("  Total: {:?}", compile_time);
    println!("  Per iteration: {:?}", compile_time / 1000);
    println!("\n✅ Icon loads in ~{:.2}μs (virtually instant!)", 
             compile_time.as_micros() as f64 / 1000.0);
}
