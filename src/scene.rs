use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::bvh::BvhNode;
use crate::hittable::Hittable;
use crate::material::Material;
use crate::sphere::Sphere;
use crate::vec3::{Color, Point3};

#[derive(Debug, Deserialize)]
pub struct SceneFile {
    pub camera: CameraDesc,
    pub render: RenderDesc,
    pub objects: Vec<SphereDesc>,
}

#[derive(Debug, Deserialize)]
pub struct CameraDesc {
    pub lookfrom: [f64; 3],
    pub lookat: [f64; 3],
    pub vup: [f64; 3],
    pub vfov: f64,
    pub aperture: f64,
    pub focus_distance: f64,
}

#[derive(Debug, Deserialize)]
pub struct RenderDesc {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_depth: u32,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct SphereDesc {
    pub center: [f64; 3],
    pub radius: f64,
    pub material: MaterialDesc,
}

#[derive(Debug, Deserialize)]
pub enum MaterialDesc {
    Lambertian { albedo: [f64; 3] },
    Metal { albedo: [f64; 3], fuzz: f64 },
    Dielectric { index: f64 },
    Emissive { color: [f64; 3], intensity: f64 },
}

impl MaterialDesc {
    fn into_material(self) -> Arc<Material> {
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

pub struct Scene {
    pub camera: CameraDesc,
    pub render: RenderDesc,
    pub world: Arc<dyn Hittable>,
}

impl Scene {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        let file: SceneFile = ron::from_str(&text)?;
        let mut objects: Vec<Arc<dyn Hittable>> = file
            .objects
            .into_iter()
            .map(|s| {
                Arc::new(Sphere::new(
                    Point3::new(s.center[0], s.center[1], s.center[2]),
                    s.radius,
                    s.material.into_material(),
                )) as Arc<dyn Hittable>
            })
            .collect();

        let world: Arc<dyn Hittable> = if objects.len() > 4 {
            Arc::new(BvhNode::build(objects))
        } else if objects.len() == 1 {
            Arc::clone(&objects[0])
        } else {
            Arc::new(BvhNode::build(std::mem::take(&mut objects)))
        };

        Ok(Self {
            camera: file.camera,
            render: file.render,
            world,
        })
    }

    pub fn default_demo() -> Self {
        let mut objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(Sphere::new(
                Point3::new(0.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.8, 0.2, 0.2),
                }),
            )),
            Arc::new(Sphere::new(
                Point3::new(-4.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.2, 0.8, 0.2),
                }),
            )),
            Arc::new(Sphere::new(
                Point3::new(4.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Metal {
                    albedo: Color::new(0.8, 0.8, 0.9),
                    fuzz: 0.05,
                }),
            )),
            Arc::new(Sphere::new(
                Point3::new(0.0, 1.0, -4.0),
                1.0,
                Arc::new(Material::Dielectric { index: 1.5 }),
            )),
            Arc::new(Sphere::new(
                Point3::new(0.0, -1000.0, 0.0),
                1000.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.5, 0.5, 0.5),
                }),
            )),
            Arc::new(Sphere::new(
                Point3::new(0.0, 8.0, 0.0),
                3.0,
                Arc::new(Material::Emissive {
                    color: Color::new(1.0, 0.95, 0.8),
                    intensity: 4.0,
                }),
            )),
        ];

        let world = Arc::new(BvhNode::build(std::mem::take(&mut objects)));
        Self {
            camera: CameraDesc {
                lookfrom: [13.0, 2.0, 3.0],
                lookat: [0.0, 1.0, 0.0],
                vup: [0.0, 1.0, 0.0],
                vfov: 20.0,
                aperture: 0.1,
                focus_distance: 10.0,
            },
            render: RenderDesc {
                width: 800,
                height: 450,
                samples_per_pixel: 50,
                max_depth: 50,
                output: "output.png".into(),
            },
            world,
        }
    }
}

fn arr3(v: [f64; 3]) -> Color {
    Color::new(v[0], v[1], v[2])
}
