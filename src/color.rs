use image::Rgb;
use serde::Deserialize;

use crate::vec3::Color;

/// How linear radiance is encoded into 8-bit display values.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GammaEncoding {
    /// Legacy sqrt encoding (effective gamma 2.0), matching the original renderer.
    Gamma2,
    /// Standard sRGB OETF for display-ready PNG output.
    Srgb,
    /// No gamma curve; linear values clamped to bytes (useful for debugging).
    Linear,
}

impl Default for GammaEncoding {
    fn default() -> Self {
        Self::Gamma2
    }
}

/// Output color pipeline applied when writing finished pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPipeline {
    pub gamma: GammaEncoding,
    pub exposure: f64,
}

impl Default for ColorPipeline {
    fn default() -> Self {
        Self {
            gamma: GammaEncoding::default(),
            exposure: 1.0,
        }
    }
}

impl ColorPipeline {
    pub fn apply_exposure(&self, color: Color) -> Color {
        Color::new(
            color.x * self.exposure,
            color.y * self.exposure,
            color.z * self.exposure,
        )
    }

    pub fn encode_pixel(&self, color: Color) -> Rgb<u8> {
        let exposed = self.apply_exposure(color);
        Rgb([
            encode_channel(exposed.x, self.gamma),
            encode_channel(exposed.y, self.gamma),
            encode_channel(exposed.z, self.gamma),
        ])
    }
}

fn encode_channel(linear: f64, gamma: GammaEncoding) -> u8 {
    let linear = linear.max(0.0);
    let encoded = match gamma {
        GammaEncoding::Gamma2 => linear.sqrt(),
        GammaEncoding::Srgb => linear_to_srgb(linear),
        GammaEncoding::Linear => linear,
    };
    (encoded.clamp(0.0, 1.0) * 255.0)
        .clamp(0.0, 255.0)
        .round() as u8
}

/// Convert a linear light intensity in [0, 1] to sRGB display space.
pub fn linear_to_srgb(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Decode an sRGB-encoded value to linear light intensity.
pub fn srgb_to_linear(srgb: f64) -> f64 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trip_near_mid_gray() {
        let linear = 0.18;
        let encoded = linear_to_srgb(linear);
        let decoded = srgb_to_linear(encoded);
        assert!((decoded - linear).abs() < 1e-6);
    }

    #[test]
    fn gamma2_matches_legacy_sqrt_encoding() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Gamma2,
            exposure: 1.0,
        };
        let pixel = pipeline.encode_pixel(Color::new(0.25, 0.25, 0.25));
        assert_eq!(pixel.0, [128, 128, 128]);
    }

    #[test]
    fn exposure_scales_before_gamma() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 2.0,
        };
        let pixel = pipeline.encode_pixel(Color::new(0.25, 0.0, 0.0));
        assert_eq!(pixel.0[0], 128);
    }

    #[test]
    fn srgb_dark_linear_segment_is_linear() {
        assert!((linear_to_srgb(0.0) - 0.0).abs() < 1e-12);
        assert!((linear_to_srgb(0.0031308) - 12.92 * 0.0031308).abs() < 1e-9);
    }

    #[test]
    fn linear_encoding_clamps_high_values() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 1.0,
        };
        let pixel = pipeline.encode_pixel(Color::new(2.0, -1.0, 0.5));
        assert_eq!(pixel.0, [255, 0, 128]);
    }

    #[test]
    fn unit_linear_white_reaches_byte_255() {
        for gamma in [GammaEncoding::Gamma2, GammaEncoding::Srgb, GammaEncoding::Linear] {
            let pipeline = ColorPipeline {
                gamma,
                exposure: 1.0,
            };
            let pixel = pipeline.encode_pixel(Color::new(1.0, 1.0, 1.0));
            assert_eq!(pixel.0, [255, 255, 255], "gamma {:?}", gamma);
        }
    }
}
