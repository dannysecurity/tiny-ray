//! Ordered dithering applied when quantizing gamma-encoded radiance to 8-bit PNG pixels.
//!
//! Dither breaks up banding in smooth gradients after tone mapping and gamma encoding,
//! especially at low sample counts where per-pixel radiance varies subtly.

use serde::Deserialize;

/// How linear display values are rounded to 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DitherMode {
    /// Round each channel independently (legacy behavior).
    None,
    /// 8×8 Bayer ordered dither; spatially varying thresholds reduce contour banding.
    Bayer8x8,
}

impl Default for DitherMode {
    fn default() -> Self {
        Self::None
    }
}

impl DitherMode {
    /// Quantize a gamma-encoded channel in `[0, 255]` to an 8-bit value.
    pub fn quantize(&self, value: f64, x: u32, y: u32, channel: u8) -> u8 {
        match self {
            Self::None => value.round().clamp(0.0, 255.0) as u8,
            Self::Bayer8x8 => {
                let threshold = bayer_threshold(x, y, channel);
                (value + threshold).floor().clamp(0.0, 255.0) as u8
            }
        }
    }
}

/// Standard 8×8 Bayer matrix threshold in `[-0.5, 0.5)` for error diffusion–free dither.
fn bayer_threshold(x: u32, y: u32, channel: u8) -> f64 {
    const MATRIX: [[u8; 8]; 8] = [
        [0, 48, 12, 60, 3, 51, 15, 63],
        [32, 16, 44, 28, 35, 19, 47, 31],
        [8, 56, 4, 52, 11, 59, 7, 55],
        [40, 24, 36, 20, 43, 27, 39, 23],
        [2, 50, 14, 62, 1, 49, 13, 61],
        [34, 18, 46, 30, 33, 17, 45, 29],
        [10, 58, 6, 54, 9, 57, 5, 53],
        [42, 26, 38, 22, 41, 25, 37, 21],
    ];
    let index = (MATRIX[(y % 8) as usize][(x % 8) as usize] + channel as u8) % 64;
    index as f64 / 64.0 - 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_rounds_to_nearest_byte() {
        assert_eq!(DitherMode::None.quantize(127.4, 0, 0, 0), 127);
        assert_eq!(DitherMode::None.quantize(127.6, 0, 0, 0), 128);
    }

    #[test]
    fn bayer_thresholds_stay_in_half_open_interval() {
        for y in 0..16 {
            for x in 0..16 {
                for channel in 0..3 {
                    let threshold = bayer_threshold(x, y, channel);
                    assert!((-0.5..0.5).contains(&threshold));
                }
            }
        }
    }

    #[test]
    fn bayer_spreads_quantization_across_neighbors() {
        let value = 128.0;
        let mut bytes = Vec::new();
        for y in 0..8 {
            for x in 0..8 {
                bytes.push(DitherMode::Bayer8x8.quantize(value, x, y, 0));
            }
        }
        bytes.sort_unstable();
        bytes.dedup();
        assert!(bytes.len() > 1, "bayer should produce more than one byte level");
    }

    #[test]
    fn bayer_is_deterministic_for_pixel_and_channel() {
        let a = DitherMode::Bayer8x8.quantize(100.25, 17, 23, 2);
        let b = DitherMode::Bayer8x8.quantize(100.25, 17, 23, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn bayer_channel_offset_changes_threshold() {
        let base = bayer_threshold(3, 5, 0);
        let shifted = bayer_threshold(3, 5, 1);
        assert_ne!(base, shifted);
    }

    #[test]
    fn bayer_clamps_out_of_range_values() {
        assert_eq!(DitherMode::Bayer8x8.quantize(-10.0, 0, 0, 0), 0);
        assert_eq!(DitherMode::Bayer8x8.quantize(300.0, 0, 0, 0), 255);
    }
}
