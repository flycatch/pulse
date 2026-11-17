//! Tray icon with live speeds, for platforms whose tray cannot show text (Windows, Linux).
#![cfg_attr(target_os = "macos", allow(dead_code))]

use font8x8::legacy::BASIC_LEGACY;
use tray_icon::Icon;

const SIZE: usize = 32;
const DOWN: [u8; 4] = [0x4c, 0xc9, 0x6b, 0xff];
const UP: [u8; 4] = [0xf0, 0x9a, 0x3c, 0xff];

/// Rate in at most 4 chars, e.g. `950B`, `1.6K`, `16M`, `160M`.
pub fn compact(bytes_per_sec: f64) -> String {
    let (mut v, mut i) = (bytes_per_sec, 0);
    while v >= 1000.0 && i < 3 {
        v /= 1000.0;
        i += 1;
    }
    let unit = ["B", "K", "M", "G"][i];
    if i > 0 && v < 10.0 {
        format!("{v:.1}{unit}")
    } else {
        format!("{v:.0}{unit}")
    }
}

/// Two rows of text: down (green) over up (orange). Glyphs are stretched 2x vertically.
pub fn render(down: &str, up: &str) -> Option<Icon> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    draw_row(&mut rgba, 0, down, DOWN);
    draw_row(&mut rgba, 16, up, UP);
    Icon::from_rgba(rgba, SIZE as u32, SIZE as u32).ok()
}

fn draw_row(rgba: &mut [u8], y0: usize, s: &str, color: [u8; 4]) {
    for (ci, ch) in s.chars().take(4).enumerate() {
        let idx = if (ch as usize) < 128 { ch as usize } else { b'?' as usize };
        for (row, bits) in BASIC_LEGACY[idx].iter().enumerate() {
            for col in 0..8 {
                if bits >> col & 1 == 0 {
                    continue;
                }
                for dy in 0..2 {
                    let (x, y) = (ci * 8 + col, y0 + row * 2 + dy);
                    rgba[(y * SIZE + x) * 4..][..4].copy_from_slice(&color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_fmt() {
        assert_eq!(compact(0.0), "0B");
        assert_eq!(compact(950.0), "950B");
        assert_eq!(compact(1_600.0), "1.6K");
        assert_eq!(compact(16_000.0), "16K");
        assert_eq!(compact(160_000_000.0), "160M");
        assert_eq!(compact(12_500_000_000.0), "12G"); // 100 Gb/s link
    }

    #[test]
    fn renders() {
        assert!(render("1.6M", "950B").is_some());
    }
}
