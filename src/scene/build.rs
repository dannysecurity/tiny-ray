//! Runtime world construction from validated scene descriptors.

use std::sync::Arc;

use crate::hittable::Hittable;
use crate::lights::LightList;
use crate::plane::Plane;
use crate::sphere::Sphere;
use crate::vec3::{Point3, Vec3};
use crate::world::SceneWorld;

use super::format::{PlaneDesc, SphereDesc};

/// Geometry and emissive lights assembled from a validated [`SceneFile`].
pub struct BuiltWorld {
    pub world: Arc<dyn Hittable>,
    pub lights: LightList,
}

impl BuiltWorld {
    pub fn from_geometry(
        objects: Vec<super::format::SphereDesc>,
        planes: Vec<super::format::PlaneDesc>,
    ) -> Self {
        let spheres = build_spheres(objects);
        let lights = LightList::from_spheres(&spheres);
        let infinite_planes = build_planes(planes);
        let bounded = spheres
            .into_iter()
            .map(|sphere| Arc::new(sphere) as Arc<dyn Hittable>)
            .collect();
        Self {
            world: SceneWorld::assemble(bounded, infinite_planes),
            lights,
        }
    }
}

fn build_spheres(descriptors: Vec<SphereDesc>) -> Vec<Sphere> {
    descriptors
        .into_iter()
        .map(|descriptor| {
            Sphere::new(
                Point3::from_array(descriptor.center),
                descriptor.radius,
                descriptor.material.into_material(),
            )
        })
        .collect()
}

fn build_planes(descriptors: Vec<PlaneDesc>) -> Vec<Arc<dyn Hittable>> {
    descriptors
        .into_iter()
        .map(|descriptor| {
            Arc::new(Plane::new(
                Point3::from_array(descriptor.point),
                Vec3::from_array(descriptor.normal),
                descriptor.material.into_material(),
            )) as Arc<dyn Hittable>
        })
        .collect()
}

/// Accelerate a bounded object list (spheres only — planes use [`SceneWorld`]).
pub fn accelerate_world(objects: Vec<Arc<dyn Hittable>>) -> Arc<dyn Hittable> {
    SceneWorld::assemble(objects, vec![])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::scene::load_scene_file;
    use crate::geometry_tests::unit_sphere_at;
    use crate::hittable::Hittable;
    use crate::ray::Ray;
    use crate::vec3::Point3;

    fn sphere_at(center: (f64, f64, f64)) -> Arc<dyn Hittable> {
        Arc::new(unit_sphere_at(center, 1.0))
    }

    #[test]
    fn accelerate_world_returns_single_primitive_without_bvh_wrapper() {
        let sphere = sphere_at((0.0, 0.0, -2.0));
        let world = accelerate_world(vec![Arc::clone(&sphere)]);
        let ray = Ray::new(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0), 0.0);
        assert!(world.hit(&ray, 0.001, f64::INFINITY).is_some());
        assert!(sphere.hit(&ray, 0.001, f64::INFINITY).is_some());
    }

    #[test]
    fn accelerate_world_builds_bvh_for_multiple_primitives() {
        let world = accelerate_world(vec![
            sphere_at((-1.0, 0.0, 0.0)),
            sphere_at((1.0, 0.0, 0.0)),
        ]);
        let ray = Ray::new(Point3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0), 0.0);
        assert!(world.hit(&ray, 0.001, f64::INFINITY).is_some());
    }

    #[test]
    fn from_geometry_builds_demo_lights() {
        let file = load_scene_file("scenes/demo.ron").unwrap();
        let built = BuiltWorld::from_geometry(file.objects, file.planes);
        assert_eq!(built.lights.len(), 1);
        let ray = Ray::new(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0), 0.0);
        assert!(built.world.hit(&ray, 0.001, f64::INFINITY).is_some());
    }
}
