//! Generates 6 PNG patterns for hardware verification of G635L USB packet
//! protocol changes. Patterns are 68x34 grayscale, sized to match the G635L
//! input grid in pixel-image mode.
//!
//! Run with:
//!     cargo run --example anime-test-patterns
//!
//! Output: /tmp/g635l-patterns/{single-pixel,corners,horizontal-lines,
//!         vertical-lines,rectangle-outline,diagonal-stripe}.png
//!
//! Push each pattern to the matrix with:
//!     asusctl anime pixel-image --path /tmp/g635l-patterns/<pattern>.png

use std::fs;
use std::path::{Path, PathBuf};

const WIDTH: u32 = 68;
const HEIGHT: u32 = 34;
const OUT_DIR: &str = "/tmp/g635l-patterns";

/// Writes an 8-bit grayscale PNG. White = 0xff, black = 0x00.
fn write_png(path: &Path, pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        pixels.len(),
        (WIDTH * HEIGHT) as usize,
        "pixel buffer must be {} bytes",
        WIDTH * HEIGHT
    );

    image::save_buffer(path, pixels, WIDTH, HEIGHT, image::ExtendedColorType::L8)?;
    Ok(())
}

fn pattern_single_pixel() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    buf[0] = 0xff; // input (0, 0)
    buf
}

fn pattern_corners() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    buf[0] = 0xff; // top-left
    buf[w - 1] = 0xff; // top-right
    buf[(h - 1) * w] = 0xff; // bottom-left
    buf[h * w - 1] = 0xff; // bottom-right
    buf
}

fn pattern_horizontal_lines() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let w = WIDTH as usize;
    for &y in &[
        0usize, 16, 33,
    ] {
        for x in 0..w {
            buf[y * w + x] = 0xff;
        }
    }
    buf
}

fn pattern_vertical_lines() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    for &x in &[
        0usize, 33, 67,
    ] {
        for y in 0..h {
            buf[y * w + x] = 0xff;
        }
    }
    buf
}

fn pattern_rectangle_outline() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    // Top and bottom edges
    for x in 0..w {
        buf[x] = 0xff;
        buf[(h - 1) * w + x] = 0xff;
    }
    // Left and right edges
    for y in 0..h {
        buf[y * w] = 0xff;
        buf[y * w + (w - 1)] = 0xff;
    }
    buf
}

fn pattern_diagonal_stripe() -> Vec<u8> {
    let mut buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    // One pixel per row at (y*2, y) — exercises the staggered LED packing.
    // Bound x to width to avoid overflow at large y.
    for y in 0..h {
        let x = (y * 2).min(w - 1);
        buf[y * w + x] = 0xff;
    }
    buf
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(OUT_DIR);
    fs::create_dir_all(&out_dir)?;

    #[allow(clippy::type_complexity)]
    let patterns: &[(&str, fn() -> Vec<u8>)] = &[
        ("single-pixel", pattern_single_pixel),
        ("corners", pattern_corners),
        ("horizontal-lines", pattern_horizontal_lines),
        ("vertical-lines", pattern_vertical_lines),
        ("rectangle-outline", pattern_rectangle_outline),
        ("diagonal-stripe", pattern_diagonal_stripe),
    ];

    for (name, generator) in patterns {
        let path = out_dir.join(format!("{name}.png"));
        let pixels = generator();
        write_png(&path, &pixels)?;
        println!("wrote {}", path.display());
    }

    println!();
    println!("All patterns written to {}", out_dir.display());
    println!("Push each to the matrix with:");
    println!(
        "  asusctl anime pixel-image --path {}/<pattern>.png",
        out_dir.display()
    );

    Ok(())
}
