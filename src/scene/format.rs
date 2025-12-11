use std::sync::Arc;

use serde::Deserialize;

use crate::color::GammaEncoding;
use crate::material::Material;
use crate::sampling::AntiAliasing;
use crate::vec3::Color;

#[derive(Debug, Deserialize, PartialEq)]
pub struct SceneFile {
    pub camera: CameraDesc,
    pub render: RenderDesc,
    pub objects: Vec<SphereDesc>,
    #[serde(default)]
    pub planes: Vec<PlaneDesc>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct CameraDesc {
    pub lookfrom: [f64; 3],
    pub lookat: [f64; 3],
    pub vup: [f64; 3],
    pub vfov: f64,
    pub aperture: f64,
    pub focus_distance: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
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

#[derive(Debug, Deserialize, PartialEq)]
pub struct SphereDesc {
    pub center: [f64; 3],
    pub radius: f64,
    pub material: MaterialDesc,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct PlaneDesc {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub material: MaterialDesc,
}

/// Externally tagged enum: works with RON (`Lambertian(albedo: ...)`), JSON
/// (`{"Lambertian": {"albedo": [...]}}`), and YAML (`Lambertian: {albedo: [...]}`).
#[derive(Debug, Deserialize, PartialEq)]
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
                albedo: arr3(albedo),
            },
            MaterialDesc::Metal { albedo, fuzz } => Material::Metal {
                albedo: arr3(albedo),
                fuzz,
            },
            MaterialDesc::Dielectric { index } => Material::Dielectric { index },
            MaterialDesc::Emissive { color, intensity } => Material::Emissive {
                color: arr3(color),
                intensity,
            },
        })
    }
}

fn arr3(v: [f64; 3]) -> Color {
    Color::new(v[0], v[1], v[2])
}
