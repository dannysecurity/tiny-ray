use image::Rgb;
use serde::Deserialize;

use crate::vec3::Color;

/// HDR-to-display compression applied after exposure and before gamma encoding.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneMapping {
    /// No tone mapping; linear values pass through after exposure (legacy behavior).
    None,
    /// Simple Reinhard per-channel compression: x / (1 + x).
    Reinhard,
    /// ACES filmic approximation (Narkowicz 2015).
    Aces,
}

impl Default for ToneMapping {
    fn default() -> Self {
        Self::None
    }
}

/// How scene-authored RGB triples are interpreted before path tracing.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputColorSpace {
    /// Colors are already linear light intensities (legacy default).
    Linear,
    /// Colors are sRGB-encoded display values decoded to linear at load time.
    Srgb,
}

impl Default for InputColorSpace {
    fn default() -> Self {
        Self::Linear
    }
}

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
    pub tone_map: ToneMapping,
}

impl Default for ColorPipeline {
    fn default() -> Self {
        Self {
            gamma: GammaEncoding::default(),
            exposure: 1.0,
            tone_map: ToneMapping::default(),
        }
    }
}

impl ColorPipeline {
    pub fn apply_exposure(&self, color: Color) -> Color {
        let color = sanitize_color(color);
        Color::new(
            color.x * self.exposure,
            color.y * self.exposure,
            color.z * self.exposure,
        )
    }

    pub fn apply_tone_map(&self, color: Color) -> Color {
        let color = sanitize_color(color);
        match self.tone_map {
            ToneMapping::None => color,
            ToneMapping::Reinhard => Color::new(
                reinhard(color.x),
                reinhard(color.y),
                reinhard(color.z),
            ),
            ToneMapping::Aces => Color::new(
                aces_filmic(color.x),
                aces_filmic(color.y),
                aces_filmic(color.z),
            ),
        }
    }

    pub fn encode_pixel(&self, color: Color) -> Rgb<u8> {
        let exposed = self.apply_exposure(color);
        let mapped = self.apply_tone_map(exposed);
        Rgb([
            encode_channel(mapped.x, self.gamma),
            encode_channel(mapped.y, self.gamma),
            encode_channel(mapped.z, self.gamma),
        ])
    }
}

fn reinhard(linear: f64) -> f64 {
    let linear = linear.max(0.0);
    linear / (1.0 + linear)
}

/// ACES filmic tone curve (Narkowicz approximation).
fn aces_filmic(linear: f64) -> f64 {
    const A: f64 = 2.51;
    const B: f64 = 0.03;
    const C: f64 = 2.43;
    const D: f64 = 0.59;
    const E: f64 = 0.14;
    let x = linear.max(0.0);
    (x * (A * x + B)) / (x * (C * x + D) + E)
}

/// Clamp negative values and replace NaN/Inf with black so bad paths do not poison PNG output.
pub fn sanitize_color(color: Color) -> Color {
    Color::new(
        sanitize_component(color.x),
        sanitize_component(color.y),
        sanitize_component(color.z),
    )
}

fn sanitize_component(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
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

/// Convert a scene-authored RGB triple into linear radiance for shading.
pub fn decode_scene_color(rgb: [f64; 3], space: InputColorSpace) -> Color {
    match space {
        InputColorSpace::Linear => Color::from_array(rgb),
        InputColorSpace::Srgb => Color::new(
            srgb_to_linear(rgb[0]),
            srgb_to_linear(rgb[1]),
            srgb_to_linear(rgb[2]),
        ),
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
    fn decode_scene_color_linear_is_identity() {
        let rgb = [0.25, 0.5, 0.75];
        let color = decode_scene_color(rgb, InputColorSpace::Linear);
        assert_eq!(color, Color::from_array(rgb));
    }

    #[test]
    fn decode_scene_color_srgb_decodes_to_linear() {
        let color = decode_scene_color([0.5, 0.5, 0.5], InputColorSpace::Srgb);
        let expected = srgb_to_linear(0.5);
        assert!((color.x - expected).abs() < 1e-12);
        assert!((color.y - expected).abs() < 1e-12);
        assert!((color.z - expected).abs() < 1e-12);
    }

    #[test]
    fn decode_scene_color_srgb_dark_segment_is_linear() {
        let color = decode_scene_color([0.04, 0.04, 0.04], InputColorSpace::Srgb);
        assert!((color.x - 0.04 / 12.92).abs() < 1e-12);
    }

    #[test]
    fn gamma2_matches_legacy_sqrt_encoding() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Gamma2,
            exposure: 1.0,
            tone_map: ToneMapping::None,
        };
        let pixel = pipeline.encode_pixel(Color::new(0.25, 0.25, 0.25));
        assert_eq!(pixel.0, [128, 128, 128]);
    }

    #[test]
    fn exposure_scales_before_gamma() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 2.0,
            tone_map: ToneMapping::None,
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
            tone_map: ToneMapping::None,
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
                tone_map: ToneMapping::None,
            };
            let pixel = pipeline.encode_pixel(Color::new(1.0, 1.0, 1.0));
            assert_eq!(pixel.0, [255, 255, 255], "gamma {:?}", gamma);
        }
    }

    #[test]
    fn sanitize_color_maps_non_finite_to_black() {
        let cleaned = sanitize_color(Color::new(f64::NAN, f64::INFINITY, -1.0));
        assert_eq!(cleaned, Color::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn encode_pixel_recovers_from_non_finite_radiance() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 1.0,
            tone_map: ToneMapping::None,
        };
        let pixel = pipeline.encode_pixel(Color::new(f64::NAN, 0.5, f64::INFINITY));
        assert_eq!(pixel.0, [0, 128, 0]);
    }

    #[test]
    fn reinhard_compresses_bright_highlights() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 1.0,
            tone_map: ToneMapping::Reinhard,
        };
        let pixel = pipeline.encode_pixel(Color::new(4.0, 0.0, 0.0));
        assert_eq!(pixel.0[0], 204);
    }

    #[test]
    fn reinhard_preserves_midtones_near_identity() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 1.0,
            tone_map: ToneMapping::Reinhard,
        };
        let mapped = pipeline.apply_tone_map(Color::new(0.25, 0.5, 0.75));
        assert!((mapped.x - 0.2).abs() < 1e-12);
        assert!((mapped.y - 1.0 / 3.0).abs() < 1e-12);
        assert!((mapped.z - 0.75 / 1.75).abs() < 1e-12);
    }

    #[test]
    fn aces_maps_unit_white_below_linear_one() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 1.0,
            tone_map: ToneMapping::Aces,
        };
        let mapped = pipeline.apply_tone_map(Color::new(1.0, 1.0, 1.0));
        assert!(mapped.x < 1.0);
        assert!(mapped.x > 0.8);
    }

    #[test]
    fn tone_map_runs_after_exposure() {
        let pipeline = ColorPipeline {
            gamma: GammaEncoding::Linear,
            exposure: 2.0,
            tone_map: ToneMapping::Reinhard,
        };
        let mapped = pipeline.apply_tone_map(pipeline.apply_exposure(Color::new(1.0, 0.0, 0.0)));
        assert!((mapped.x - 2.0 / 3.0).abs() < 1e-12);
    }
}
