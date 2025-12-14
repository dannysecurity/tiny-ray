use std::sync::Arc;

use serde::Deserialize;

use crate::color::GammaEncoding;
use crate::material::Material;
use crate::sampling::AntiAliasing;
pub use crate::sky::SkyDesc;
use crate::vec3::Color;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SceneFile {
    /// Paths to fragment scene files, resolved relative to the including file.
    /// Fragments contribute `objects`, `planes`, and nested `include` entries;
    /// their `camera` and `render` blocks are ignored.
    #[serde(default)]
    pub include: Vec<String>,
    pub camera: CameraDesc,
    pub render: RenderDesc,
    #[serde(default)]
    pub sky: SkyDesc,
    #[serde(default)]
    pub objects: Vec<SphereDesc>,
    #[serde(default)]
    pub planes: Vec<PlaneDesc>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CameraDesc {
    pub lookfrom: [f64; 3],
    pub lookat: [f64; 3],
    pub vup: [f64; 3],
    pub vfov: f64,
    pub aperture: f64,
    pub focus_distance: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RenderDesc {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_depth: u32,
    pub output: String,
    #[serde(default)]
    pub gamma: GammaEncoding,
    #[serde(default = "default_exposure")]
    pub exposure: f64,
    #[serde(default)]
    pub aa: AntiAliasing,
}

fn default_exposure() -> f64 {
    1.0
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SphereDesc {
    pub center: [f64; 3],
    pub radius: f64,
    pub material: MaterialDesc,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PlaneDesc {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub material: MaterialDesc,
}

/// Externally tagged enum: works with RON (`Lambertian(albedo: ...)`), JSON
/// (`{"Lambertian": {"albedo": [...]}}`), and YAML (`Lambertian: {albedo: [...]}`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum MaterialDesc {
    Lambertian { albedo: [f64; 3] },
    Metal { albedo: [f64; 3], fuzz: f64 },
    Dielectric { index: f64 },
    Emissive { color: [f64; 3], intensity: f64 },
}

impl MaterialDesc {
    pub fn into_material(self) -> Arc<Material> {
        Arc::new(match self {
            MaterialDesc::Lambertian { albedo } => Material::Lambertian {
                albedo: Color::from_array(albedo),
            },
            MaterialDesc::Metal { albedo, fuzz } => Material::Metal {
                albedo: Color::from_array(albedo),
                fuzz,
            },
            MaterialDesc::Dielectric { index } => Material::Dielectric { index },
            MaterialDesc::Emissive { color, intensity } => Material::Emissive {
                color: Color::from_array(color),
                intensity,
            },
        })
    }
}
