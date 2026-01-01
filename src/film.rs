use serde::Deserialize;

use crate::vec3::Color;

/// Pixel reconstruction filter applied when accumulating sub-pixel samples.
///
/// Offsets are measured in pixel units relative to the pixel center (0, 0).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFilter {
    /// Uniform box filter; every sample inside the pixel gets equal weight.
    Box,
    /// Gaussian falloff centered on the pixel; softens high-frequency aliasing.
    Gaussian,
    /// Mitchell–Netravali cubic filter (B = C = 1/3); sharp but smooth edges.
    Mitchell,
}

impl Default for PixelFilter {
    fn default() -> Self {
        Self::Box
    }
}

impl PixelFilter {
    /// Filter weight at offset `(dx, dy)` from the pixel center.
    pub fn weight(&self, dx: f64, dy: f64) -> f64 {
        match self {
            Self::Box => 1.0,
            Self::Gaussian => gaussian_weight(dx, dy),
            Self::Mitchell => mitchell_weight(dx, dy),
        }
    }
}

/// Weighted accumulation of Monte Carlo samples into a single pixel color.
pub fn accumulate_weighted(samples: impl IntoIterator<Item = (Color, f64)>) -> Color {
    let mut color = Color::default();
    let mut weight_sum = 0.0;
    for (sample, weight) in samples {
        if weight > 0.0 && weight.is_finite() {
            color += sample * weight;
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 {
        color / weight_sum
    } else {
        Color::default()
    }
}

fn gaussian_weight(dx: f64, dy: f64) -> f64 {
    // sigma ≈ 0.35 px gives a practical reconstruction kernel for path tracing.
    const SIGMA: f64 = 0.35;
    let r2 = dx * dx + dy * dy;
    (-0.5 * r2 / (SIGMA * SIGMA)).exp()
}

fn mitchell_weight(dx: f64, dy: f64) -> f64 {
    const B: f64 = 1.0 / 3.0;
    const C: f64 = 1.0 / 3.0;
    mitchell_1d(dx, B, C) * mitchell_1d(dy, B, C)
}

fn mitchell_1d(x: f64, b: f64, c: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        (1.0 / 6.0)
            * ((12.0 - 9.0 * b - 6.0 * c) * x.powi(3)
                + (-18.0 + 12.0 * b + 6.0 * c) * x.powi(2)
                + (6.0 - 2.0 * b))
    } else if x < 2.0 {
        (1.0 / 6.0)
            * ((-b - 6.0 * c) * x.powi(3)
                + (6.0 * b + 30.0 * c) * x.powi(2)
                + (-12.0 * b - 48.0 * c) * x
                + (8.0 * b + 24.0 * c))
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_filter_is_constant() {
        assert_eq!(PixelFilter::Box.weight(0.0, 0.0), 1.0);
        assert_eq!(PixelFilter::Box.weight(0.49, -0.49), 1.0);
    }

    #[test]
    fn gaussian_peaks_at_center() {
        let center = PixelFilter::Gaussian.weight(0.0, 0.0);
        let edge = PixelFilter::Gaussian.weight(0.45, 0.45);
        assert!(center > edge);
        assert!(edge > 0.0);
    }

    #[test]
    fn mitchell_is_separable_and_zero_outside_radius_two() {
        let center = PixelFilter::Mitchell.weight(0.0, 0.0);
        assert!(center > PixelFilter::Mitchell.weight(1.5, 0.0));
        assert_eq!(PixelFilter::Mitchell.weight(2.0, 0.0), 0.0);
        assert_eq!(PixelFilter::Mitchell.weight(0.0, 2.5), 0.0);
    }

    #[test]
    fn mitchell_off_center_is_positive() {
        assert!(PixelFilter::Mitchell.weight(0.5, 0.0) > 0.0);
    }

    #[test]
    fn accumulate_weighted_averages_by_weight() {
        let red = Color::new(1.0, 0.0, 0.0);
        let blue = Color::new(0.0, 0.0, 1.0);
        let mixed = accumulate_weighted([(red, 3.0), (blue, 1.0)]);
        assert!((mixed.x - 0.75).abs() < 1e-12);
        assert!((mixed.z - 0.25).abs() < 1e-12);
    }

    #[test]
    fn accumulate_weighted_ignores_non_positive_weights() {
        let color = Color::new(1.0, 0.0, 0.0);
        let result = accumulate_weighted([(color, 0.0), (color, -1.0)]);
        assert_eq!(result, Color::default());
    }

    #[test]
    fn box_filter_matches_uniform_average() {
        let samples = [
            (Color::new(1.0, 0.0, 0.0), 1.0),
            (Color::new(0.0, 1.0, 0.0), 1.0),
            (Color::new(0.0, 0.0, 1.0), 1.0),
        ];
        let avg = accumulate_weighted(samples);
        assert!((avg.x - 1.0 / 3.0).abs() < 1e-12);
        assert!((avg.y - 1.0 / 3.0).abs() < 1e-12);
        assert!((avg.z - 1.0 / 3.0).abs() < 1e-12);
    }
}
