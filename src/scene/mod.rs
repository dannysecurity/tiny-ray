mod format;
mod loader;

pub use format::{CameraDesc, RenderDesc, SceneFile};
pub use loader::load_scene_file;

use std::path::Path;
use std::sync::Arc;

use crate::bvh::BvhNode;
use crate::hittable::Hittable;
use crate::lights::LightList;
use crate::material::Material;
use crate::sphere::Sphere;
use crate::vec3::{Color, Point3};

pub struct Scene {
    pub camera: CameraDesc,
    pub render: RenderDesc,
    pub world: Arc<dyn Hittable>,
    pub lights: LightList,
}

impl Scene {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = load_scene_file(path)?;
        Ok(Self::from_scene_file(file))
    }

    pub fn from_scene_file(file: SceneFile) -> Self {
        let mut spheres: Vec<Sphere> = file
            .objects
            .into_iter()
            .map(|s| {
                Sphere::new(
                    Point3::new(s.center[0], s.center[1], s.center[2]),
                    s.radius,
                    s.material.into_material(),
                )
            })
            .collect();
        let lights = LightList::from_spheres(&spheres);
        let mut objects: Vec<Arc<dyn Hittable>> = spheres
            .drain(..)
            .map(|sphere| Arc::new(sphere) as Arc<dyn Hittable>)
            .collect();

        let world: Arc<dyn Hittable> = if objects.len() > 4 {
            Arc::new(BvhNode::build(objects))
        } else if objects.len() == 1 {
            Arc::clone(&objects[0])
        } else {
            Arc::new(BvhNode::build(std::mem::take(&mut objects)))
        };

        Self {
            camera: file.camera,
            render: file.render,
            world,
            lights,
        }
    }

    pub fn default_demo() -> Self {
        let spheres = vec![
            Sphere::new(
                Point3::new(0.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.8, 0.2, 0.2),
                }),
            ),
            Sphere::new(
                Point3::new(-4.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.2, 0.8, 0.2),
                }),
            ),
            Sphere::new(
                Point3::new(4.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Metal {
                    albedo: Color::new(0.8, 0.8, 0.9),
                    fuzz: 0.05,
                }),
            ),
            Sphere::new(
                Point3::new(0.0, 1.0, -4.0),
                1.0,
                Arc::new(Material::Dielectric { index: 1.5 }),
            ),
            Sphere::new(
                Point3::new(0.0, -1000.0, 0.0),
                1000.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.5, 0.5, 0.5),
                }),
            ),
            Sphere::new(
                Point3::new(0.0, 8.0, 0.0),
                3.0,
                Arc::new(Material::Emissive {
                    color: Color::new(1.0, 0.95, 0.8),
                    intensity: 4.0,
                }),
            ),
        ];
        let lights = LightList::from_spheres(&spheres);
        let objects: Vec<Arc<dyn Hittable>> = spheres
            .into_iter()
            .map(|sphere| Arc::new(sphere) as Arc<dyn Hittable>)
            .collect();
        let world = Arc::new(BvhNode::build(objects));
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
                gamma: Default::default(),
                exposure: 1.0,
                aa: Default::default(),
            },
            world,
            lights,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_file_builds_demo_lights() {
        let scene = Scene::from_file("scenes/demo.ron").unwrap();
        assert_eq!(scene.lights.len(), 1);
        assert_eq!(scene.render.samples_per_pixel, 50);
    }

    #[test]
    fn json_and_yaml_scenes_build_same_light_count_as_ron() {
        let ron = Scene::from_file("scenes/demo.ron").unwrap();
        let json = Scene::from_file("scenes/demo.json").unwrap();
        let yaml = Scene::from_file("scenes/demo.yaml").unwrap();
        assert_eq!(json.lights.len(), ron.lights.len());
        assert_eq!(yaml.lights.len(), ron.lights.len());
    }

    #[test]
    fn studio_scene_loads_emissive_light_and_bvh() {
        let scene = Scene::from_file("scenes/studio.ron").unwrap();
        assert_eq!(scene.lights.len(), 1);
        assert_eq!(scene.render.output, "studio.png");
        assert_eq!(scene.render.samples_per_pixel, 100);
        assert_eq!(scene.render.gamma, crate::color::GammaEncoding::Srgb);
        assert_eq!(scene.render.aa, crate::sampling::AntiAliasing::Stratified);
    }

    #[test]
    fn studio_formats_match_object_count() {
        use super::load_scene_file;

        let ron = load_scene_file("scenes/studio.ron").unwrap();
        let json = load_scene_file("scenes/studio.json").unwrap();
        let yaml = load_scene_file("scenes/studio.yaml").unwrap();
        assert_eq!(json.objects.len(), ron.objects.len());
        assert_eq!(yaml.objects.len(), ron.objects.len());
        assert_eq!(ron.objects.len(), 8);
    }
}
