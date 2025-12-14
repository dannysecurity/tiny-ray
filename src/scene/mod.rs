mod build;
mod format;
mod loader;
mod validate;

pub use build::{accelerate_world, BuiltWorld};

pub use format::{CameraDesc, RenderDesc, SceneFile};
pub use loader::{load_scene_file, load_scene_file_with_format, SceneFormat};

use std::path::Path;
use std::sync::Arc;

use crate::hittable::Hittable;
use crate::lights::LightList;
use crate::material::Material;
use crate::sky::SkyGradient;
use crate::sphere::Sphere;
use crate::vec3::{Color, Point3};
pub struct Scene {
    pub camera: CameraDesc,
    pub render: RenderDesc,
    pub sky: SkyGradient,
    pub world: Arc<dyn Hittable>,
    pub lights: LightList,
}

impl Scene {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_file_with_format(path, None)
    }

    pub fn from_file_with_format(
        path: impl AsRef<Path>,
        format: Option<SceneFormat>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file = load_scene_file_with_format(path, format)?;
        Ok(Self::from_scene_file(file))
    }

    pub fn from_scene_file(file: SceneFile) -> Self {
        let camera = file.camera;
        let render = file.render;
        let sky = file.sky.into_sky();
        let built = BuiltWorld::from_geometry(file.objects, file.planes);
        Self {
            camera,
            render,
            sky,
            world: built.world,
            lights: built.lights,
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
        let world = accelerate_world(objects);
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
            sky: SkyGradient::default(),
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
        for path in ["scenes/studio.ron", "scenes/studio.json", "scenes/studio.yaml"] {
            let scene = Scene::from_file(path).unwrap();
            assert_eq!(scene.lights.len(), 1, "{path}");
            assert_eq!(scene.render.output, "studio.png", "{path}");
            assert_eq!(scene.render.samples_per_pixel, 100, "{path}");
            assert_eq!(
                scene.render.gamma,
                crate::color::GammaEncoding::Srgb,
                "{path}"
            );
            assert_eq!(
                scene.render.aa,
                crate::sampling::AntiAliasing::Stratified,
                "{path}"
            );
        }
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

    #[test]
    fn cornell_scene_loads_planes_and_spheres() {
        for path in ["scenes/cornell.ron", "scenes/cornell.json", "scenes/cornell.yaml"] {
            let scene = Scene::from_file(path).unwrap();
            assert_eq!(scene.lights.len(), 1, "{path}");
            assert_eq!(scene.render.output, "cornell.png", "{path}");
            assert_eq!(scene.render.samples_per_pixel, 100, "{path}");
            assert_eq!(
                scene.render.gamma,
                crate::color::GammaEncoding::Srgb,
                "{path}"
            );
            assert_eq!(
                scene.render.aa,
                crate::sampling::AntiAliasing::Stratified,
                "{path}"
            );
        }
    }

    #[test]
    fn cornell_formats_match_object_count() {
        use super::load_scene_file;

        let ron = load_scene_file("scenes/cornell.ron").unwrap();
        let json = load_scene_file("scenes/cornell.json").unwrap();
        let yaml = load_scene_file("scenes/cornell.yaml").unwrap();
        assert_eq!(json.objects.len(), ron.objects.len());
        assert_eq!(yaml.objects.len(), ron.objects.len());
        assert_eq!(ron.objects.len(), 4);
        assert_eq!(ron.planes.len(), 5);
        assert_eq!(json.planes.len(), ron.planes.len());
        assert_eq!(yaml.planes.len(), ron.planes.len());
    }

    #[test]
    fn modular_cornell_builds_same_geometry_as_monolithic() {
        let monolithic = Scene::from_file("scenes/cornell.yaml").unwrap();
        let modular = Scene::from_file("scenes/cornell-modular.yaml").unwrap();
        assert_eq!(modular.lights.len(), monolithic.lights.len());
        assert_eq!(modular.render.output, "cornell.png");
    }

    #[test]
    fn sunset_scene_loads_custom_sky_and_sun_light() {
        use crate::vec3::Color;

        for path in ["scenes/sunset.ron", "scenes/sunset.json", "scenes/sunset.yaml"] {
            let scene = Scene::from_file(path).unwrap();
            assert_eq!(scene.lights.len(), 1, "{path}");
            assert_eq!(scene.render.output, "sunset.png", "{path}");
            assert_eq!(
                scene.sky.horizon,
                Color::new(1.0, 0.55, 0.32),
                "{path}"
            );
            assert_eq!(
                scene.sky.zenith,
                Color::new(0.12, 0.22, 0.55),
                "{path}"
            );
        }
    }

    #[test]
    fn sunset_formats_match_object_count() {
        use super::load_scene_file;

        let ron = load_scene_file("scenes/sunset.ron").unwrap();
        let json = load_scene_file("scenes/sunset.json").unwrap();
        let yaml = load_scene_file("scenes/sunset.yaml").unwrap();
        assert_eq!(json.objects.len(), ron.objects.len());
        assert_eq!(yaml.objects.len(), ron.objects.len());
        assert_eq!(ron.objects.len(), 4);
        assert_eq!(ron.planes.len(), 1);
    }
}
