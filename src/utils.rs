//! # Utility Functions Module
//!
//! This module provides shared helper functions used throughout the application.
//! Key utilities include:
//! - `generate_path`: A highly optimized function to generate SVG path commands from a history buffer.
//!   it pre-allocates strings to minimize heap churn during real-time updates.
//! - `hex_to_color` / `brush_to_hex`: Functions to convert between string representations of colors (for storage) and Slint types (for UI).

use slint::SharedString;

/// Helper function to convert a hex string (e.g., "#RRGGBB") to a `slint::Color`.
/// Returns a default gray color if parsing fails or format is invalid.
pub fn hex_to_color(hex: &str) -> slint::Color {
    if hex.len() == 7 && hex.starts_with('#') {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        slint::Color::from_rgb_u8(r, g, b)
    } else {
        slint::Color::from_rgb_u8(100, 100, 100) // Fallback
    }
}

/// Helper function to convert a `slint::Brush` (assuming solid color) back to a hex string.
/// Used for saving the current color state to the configuration file.
pub fn brush_to_hex(brush: slint::Brush) -> String {
    let color = brush.color();
    format!(
        "#{:02x}{:02x}{:02x}",
        color.red(),
        color.green(),
        color.blue()
    )
}

/// Returns a `SharedString` containing the SVG `d` attribute commands (M, L).
pub fn generate_path(history: &[f32], max_val: f32) -> SharedString {
    if history.is_empty() {
        return "".into();
    }

    // Pre-allocate to reduce reallocations.
    // Approx 15 bytes per point (" L 60 100.00") * 60 points = ~900 bytes.
    let mut path = String::with_capacity(history.len() * 15);

    let normalize = |val: f32| -> f32 { 100.0 - (val.min(max_val) / max_val * 100.0) };

    // Use write! or fmt for potentially faster formatting, but push_str is fine.
    // Manual formatting might be faster but less readable.
    use std::fmt::Write;
    let _ = write!(path, "M 0 {:.2}", normalize(history[0]));

    for (i, val) in history.iter().enumerate().skip(1) {
        let _ = write!(path, " L {} {:.2}", i, normalize(*val));
    }

    path.into()
}
