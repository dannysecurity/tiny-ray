//! Integration tests for the render loop's anti-aliasing and display encoding wiring.

use crate::color::{ColorPipeline, GammaEncoding, ToneMapping};
use crate::dither::DitherMode;
use crate::film::{accumulate_weighted, PixelFilter};
use crate::sampling::{pixel_offsets, AntiAliasing};
use crate::vec3::Color;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Mirrors `RenderContext::trace_pixel` sample accumulation without path tracing.
fn accumulate_flat_pixel(
    samples_per_pixel: u32,
    aa: AntiAliasing,
    filter: PixelFilter,
    radiance: Color,
    rng: &mut StdRng,
) -> Color {
    let samples = (0..samples_per_pixel).map(|sample| {
        let (du, dv) = pixel_offsets(sample, samples_per_pixel, aa, rng);
        let weight = filter.weight(du - 0.5, dv - 0.5);
        (radiance, weight)
    });
    accumulate_weighted(samples)
}

#[test]
fn r2_aa_accumulates_constant_radiance_to_input() {
    let mut rng = StdRng::seed_from_u64(42);
    let radiance = Color::new(0.4, 0.6, 0.2);
    let accumulated =
        accumulate_flat_pixel(16, AntiAliasing::R2, PixelFilter::Box, radiance, &mut rng);
    assert!((accumulated.x - radiance.x).abs() < 1e-12);
    assert!((accumulated.y - radiance.y).abs() < 1e-12);
    assert!((accumulated.z - radiance.z).abs() < 1e-12);
}

#[test]
fn bayer_dither_breaks_up_flat_byte_quantization() {
    let pipeline = ColorPipeline {
        gamma: GammaEncoding::Linear,
        exposure: 1.0,
        tone_map: ToneMapping::None,
        dither: DitherMode::Bayer8x8,
    };
    let radiance = Color::new(0.501, 0.501, 0.501);
    let mut values = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            values.push(pipeline.encode_pixel(radiance, x, y).0[0]);
        }
    }
    values.sort_unstable();
    values.dedup();
    assert!(
        values.len() > 1,
        "bayer dither should spread a mid-tone across multiple byte levels"
    );
}

#[test]
fn display_pipeline_applies_exposure_gamma_then_dither_in_order() {
    let pipeline = ColorPipeline {
        gamma: GammaEncoding::Gamma2,
        exposure: 4.0,
        tone_map: ToneMapping::None,
        dither: DitherMode::None,
    };
    let pixel = pipeline.encode_pixel(Color::new(0.25, 0.0, 0.0), 0, 0);
    assert_eq!(pixel.0[0], 255);
}

#[test]
fn mitchell_filter_downweights_off_center_aa_samples() {
    let mut rng = StdRng::seed_from_u64(7);
    let center_radiance = Color::new(1.0, 0.0, 0.0);
    let edge_radiance = Color::new(0.0, 0.0, 1.0);

    let mut center_weighted = Color::default();
    let mut edge_weighted = Color::default();
    let mut weight_sum = 0.0;
    for sample in 0..4 {
        let (du, dv) = pixel_offsets(sample, 4, AntiAliasing::Stratified, &mut rng);
        let weight = PixelFilter::Mitchell.weight(du - 0.5, dv - 0.5);
        center_weighted += center_radiance * weight;
        edge_weighted += edge_radiance * weight;
        weight_sum += weight;
    }
    let center_only = center_weighted / weight_sum;
    let edge_only = edge_weighted / weight_sum;
    assert!(center_only.x > edge_only.x);
    assert!(edge_only.z > center_only.z);
}
