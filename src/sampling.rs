use rand::Rng;
use serde::Deserialize;

/// Anti-aliasing strategy for primary-ray sub-pixel sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    /// Uniform random offsets within the pixel (default Monte Carlo AA).
    Random,
    /// Jittered stratified grid; reduces clumping for a given sample count.
    Stratified,
    /// Halton quasi-random sequence (bases 2 and 3); low-discrepancy offsets.
    Halton,
    /// Roberts R2 low-discrepancy sequence; fast quasi-random offsets with good 2D coverage.
    R2,
}

impl Default for AntiAliasing {
    fn default() -> Self {
        Self::Random
    }
}

/// Sub-pixel offsets in [0, 1) for anti-aliasing sample `sample_index`.
pub fn pixel_offsets<R: Rng + ?Sized>(
    sample_index: u32,
    samples_per_pixel: u32,
    strategy: AntiAliasing,
    rng: &mut R,
) -> (f64, f64) {
    match strategy {
        AntiAliasing::Random => (rng.gen(), rng.gen()),
        AntiAliasing::Stratified => stratified_offset(sample_index, samples_per_pixel, rng),
        AntiAliasing::Halton => halton_offset(sample_index),
        AntiAliasing::R2 => r2_offset(sample_index),
    }
}

/// Roberts R2 sequence constants (Martin Roberts, 2018).
const R2_G1: f64 = 0.7548776662466927;
const R2_G2: f64 = 0.5698404659933789;

fn r2_offset(sample_index: u32) -> (f64, f64) {
    let n = (sample_index + 1) as f64;
    ((n * R2_G1).fract(), (n * R2_G2).fract())
}

/// Halton sequence sample in [0, 1) for 1-based `index` and prime `base`.
fn halton(index: u32, base: u32) -> f64 {
    let mut f = 1.0;
    let mut result = 0.0;
    let mut i = index;
    while i > 0 {
        f /= base as f64;
        result += f * (i % base) as f64;
        i /= base;
    }
    result
}

fn halton_offset(sample_index: u32) -> (f64, f64) {
    // Halton sequences are conventionally 1-indexed; skip index 0 (all zeros).
    let index = sample_index + 1;
    (halton(index, 2), halton(index, 3))
}

fn stratified_offset<R: Rng + ?Sized>(
    sample_index: u32,
    samples_per_pixel: u32,
    rng: &mut R,
) -> (f64, f64) {
    let grid = (samples_per_pixel as f64).sqrt().ceil() as u32;
    let cell_x = sample_index % grid;
    let cell_y = sample_index / grid;
    let cell_w = 1.0 / grid as f64;
    let cell_h = 1.0 / grid as f64;
    let jitter_x = rng.gen::<f64>();
    let jitter_y = rng.gen::<f64>();
    (
        cell_x as f64 * cell_w + jitter_x * cell_w,
        cell_y as f64 * cell_h + jitter_y * cell_h,
    )
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn stratified_offsets_stay_inside_pixel() {
        let mut rng = StdRng::seed_from_u64(7);
        for sample in 0..16 {
            let (du, dv) = pixel_offsets(sample, 16, AntiAliasing::Stratified, &mut rng);
            assert!((0.0..1.0).contains(&du));
            assert!((0.0..1.0).contains(&dv));
        }
    }

    #[test]
    fn stratified_cells_are_distinct_for_perfect_square_grid() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut cells = Vec::new();
        for sample in 0..4 {
            let (du, dv) = pixel_offsets(sample, 4, AntiAliasing::Stratified, &mut rng);
            let cell_x = (du * 2.0).floor() as u32;
            let cell_y = (dv * 2.0).floor() as u32;
            cells.push((cell_x, cell_y));
        }
        cells.sort();
        cells.dedup();
        assert_eq!(cells.len(), 4);
    }

    #[test]
    fn random_offsets_cover_unit_square() {
        let mut rng = StdRng::seed_from_u64(99);
        let mut min_u: f64 = 1.0;
        let mut max_u: f64 = 0.0;
        for _ in 0..256 {
            let (du, dv) = pixel_offsets(0, 1, AntiAliasing::Random, &mut rng);
            min_u = min_u.min(du);
            max_u = max_u.max(du);
            assert!((0.0..1.0).contains(&dv));
        }
        assert!(min_u < 0.1);
        assert!(max_u > 0.9);
    }

    #[test]
    fn stratified_is_deterministic_with_fixed_rng_seed() {
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        for sample in 0..9 {
            let a = pixel_offsets(sample, 9, AntiAliasing::Stratified, &mut rng_a);
            let b = pixel_offsets(sample, 9, AntiAliasing::Stratified, &mut rng_b);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn halton_offsets_stay_inside_pixel() {
        let mut rng = StdRng::seed_from_u64(0);
        for sample in 0..64 {
            let (du, dv) = pixel_offsets(sample, 64, AntiAliasing::Halton, &mut rng);
            assert!((0.0..1.0).contains(&du));
            assert!((0.0..1.0).contains(&dv));
        }
    }

    #[test]
    fn halton_is_deterministic_and_independent_of_rng() {
        let mut rng_a = StdRng::seed_from_u64(1);
        let mut rng_b = StdRng::seed_from_u64(99);
        for sample in 0..16 {
            let a = pixel_offsets(sample, 16, AntiAliasing::Halton, &mut rng_a);
            let b = pixel_offsets(sample, 16, AntiAliasing::Halton, &mut rng_b);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn halton_samples_are_unique_for_small_counts() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut offsets = Vec::new();
        for sample in 0..16 {
            offsets.push(pixel_offsets(sample, 16, AntiAliasing::Halton, &mut rng));
        }
        offsets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        offsets.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12);
        assert_eq!(offsets.len(), 16);
    }

    #[test]
    fn halton_sequence_matches_known_values() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-12);
        assert!((halton(2, 2) - 0.25).abs() < 1e-12);
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn r2_offsets_stay_inside_pixel() {
        let mut rng = StdRng::seed_from_u64(0);
        for sample in 0..64 {
            let (du, dv) = pixel_offsets(sample, 64, AntiAliasing::R2, &mut rng);
            assert!((0.0..1.0).contains(&du));
            assert!((0.0..1.0).contains(&dv));
        }
    }

    #[test]
    fn r2_is_deterministic_and_independent_of_rng() {
        let mut rng_a = StdRng::seed_from_u64(1);
        let mut rng_b = StdRng::seed_from_u64(99);
        for sample in 0..16 {
            let a = pixel_offsets(sample, 16, AntiAliasing::R2, &mut rng_a);
            let b = pixel_offsets(sample, 16, AntiAliasing::R2, &mut rng_b);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn r2_samples_are_unique_for_small_counts() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut offsets = Vec::new();
        for sample in 0..16 {
            offsets.push(pixel_offsets(sample, 16, AntiAliasing::R2, &mut rng));
        }
        offsets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        offsets.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12);
        assert_eq!(offsets.len(), 16);
    }

    #[test]
    fn r2_first_sample_matches_known_values() {
        let (du, dv) = r2_offset(0);
        assert!((du - R2_G1).abs() < 1e-12);
        assert!((dv - R2_G2).abs() < 1e-12);
    }
}
