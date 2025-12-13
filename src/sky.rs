use serde::Deserialize;

use crate::vec3::{Color, Vec3};

/// Vertical gradient used when a ray misses geometry.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SkyDesc {
    pub horizon: [f64; 3],
    pub zenith: [f64; 3],
}

impl Default for SkyDesc {
    fn default() -> Self {
        Self {
            horizon: [1.0, 1.0, 1.0],
            zenith: [0.5, 0.7, 1.0],
        }
    }
}

impl SkyDesc {
    pub fn into_sky(self) -> SkyGradient {
        SkyGradient {
            horizon: arr3(self.horizon),
            zenith: arr3(self.zenith),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyGradient {
    pub horizon: Color,
    pub zenith: Color,
}

impl Default for SkyGradient {
    fn default() -> Self {
        SkyDesc::default().into_sky()
    }
}

impl SkyGradient {
    /// Evaluate background radiance for a ray that missed the scene.
    pub fn sample(&self, direction: Vec3) -> Color {
        let unit = direction.normalize();
        let t = 0.5 * (unit.y + 1.0);
        (1.0 - t) * self.horizon + t * self.zenith
    }
}

fn arr3(v: [f64; 3]) -> Color {
    Color::new(v[0], v[1], v[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_legacy_renderer_gradient() {
        let sky = SkyGradient::default();
        assert_eq!(sky.sample(Vec3::new(0.0, -1.0, 0.0)), Color::new(1.0, 1.0, 1.0));
        assert_eq!(sky.sample(Vec3::new(0.0, 1.0, 0.0)), Color::new(0.5, 0.7, 1.0));
    }

    #[test]
    fn horizon_and_zenith_colors_at_poles() {
        let sky = SkyGradient {
            horizon: Color::new(1.0, 0.5, 0.2),
            zenith: Color::new(0.1, 0.2, 0.8),
        };
        assert_eq!(sky.sample(Vec3::new(0.0, -1.0, 0.0)), Color::new(1.0, 0.5, 0.2));
        assert_eq!(sky.sample(Vec3::new(0.0, 1.0, 0.0)), Color::new(0.1, 0.2, 0.8));
    }

    #[test]
    fn sky_desc_deserializes_from_json() {
        let desc: SkyDesc = serde_json::from_str(
            r#"{"horizon": [1.0, 0.6, 0.3], "zenith": [0.2, 0.3, 0.6]}"#,
        )
        .unwrap();
        assert_eq!(desc.horizon, [1.0, 0.6, 0.3]);
        assert_eq!(desc.zenith, [0.2, 0.3, 0.6]);
    }
}
