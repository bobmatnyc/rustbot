// Version and build tracking for Rustbot
// This file should be updated with each build

pub const VERSION: &str = "0.0.1";
pub const BUILD: &str = "0001";

pub fn version_string() -> String {
    format!("v{}-{}", VERSION, BUILD)
}

pub fn full_version_info() -> String {
    format!("Rustbot {} (Build {})", VERSION, BUILD)
}
