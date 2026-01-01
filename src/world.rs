//! Scene-wide hit testing that keeps infinite planes outside the BVH.

use std::sync::Arc;

use crate::bvh::BvhNode;
use crate::hittable::{Aabb, HitRecord, Hittable};
use crate::intersection::{any_hit_in_objects, closest_hit, closest_hit_in_objects};
use crate::ray::Ray;

/// Composite world: bounded geometry in a BVH plus infinite planes tested on every ray.
///
/// Planes are mathematically unbounded, but their placeholder AABBs are thin slabs capped at
/// ±1000 units. Putting them in the BVH caused rays landing outside that extent to miss the
/// ground entirely. This type always intersects planes directly.
pub struct SceneWorld {
    bounded: Option<Arc<dyn Hittable>>,
    infinite_planes: Vec<Arc<dyn Hittable>>,
}

impl SceneWorld {
    /// Build a world from bounded primitives and infinite planes.
    pub fn assemble(
        bounded: Vec<Arc<dyn Hittable>>,
        infinite_planes: Vec<Arc<dyn Hittable>>,
    ) -> Arc<dyn Hittable> {
        let bounded = accelerate_bounded(bounded);
        match (bounded.as_ref(), infinite_planes.as_slice()) {
            (None, []) => panic!("scene must contain at least one object or plane"),
            (Some(sole), []) => Arc::clone(sole),
            (None, [sole]) => Arc::clone(sole),
            _ => Arc::new(Self {
                bounded,
                infinite_planes,
            }),
        }
    }
}

fn accelerate_bounded(objects: Vec<Arc<dyn Hittable>>) -> Option<Arc<dyn Hittable>> {
    match objects.as_slice() {
        [] => None,
        [sole] => Some(Arc::clone(sole)),
        _ => Some(Arc::new(BvhNode::build(objects)) as Arc<dyn Hittable>),
    }
}

impl Hittable for SceneWorld {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let bounded_hit = self
            .bounded
            .as_ref()
            .and_then(|b| b.hit(ray, t_min, t_max));
        let closest_t = bounded_hit.as_ref().map(|h| h.t).unwrap_or(t_max);
        let plane_hit =
            closest_hit_in_objects(&self.infinite_planes, ray, t_min, closest_t);
        closest_hit(bounded_hit, plane_hit)
    }

    fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        self.bounded
            .as_ref()
            .is_some_and(|b| b.any_hit(ray, t_min, t_max))
            || any_hit_in_objects(&self.infinite_planes, ray, t_min, t_max)
    }

    fn bounding_box(&self) -> Aabb {
        match (&self.bounded, self.infinite_planes.as_slice()) {
            (Some(bounded), []) => bounded.bounding_box(),
            (None, [plane]) => plane.bounding_box(),
            (Some(bounded), _) => bounded.bounding_box(),
            (None, planes) => {
                let boxes: Vec<Aabb> = planes.iter().map(|p| p.bounding_box()).collect();
                Aabb::surrounding_box(&boxes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_tests::{assert_close, floor_plane, ray_from, unit_sphere_at};
    use crate::hittable::Hittable;
    use crate::plane::Plane;
    use crate::vec3::{Point3, Vec3};

    fn sphere_at(center: (f64, f64, f64)) -> Arc<dyn Hittable> {
        Arc::new(unit_sphere_at(center, 1.0))
    }

    fn plane_at_y(y: f64) -> Arc<dyn Hittable> {
        Arc::new(Plane::new(
            Point3::new(0.0, y, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            crate::geometry_tests::test_material(),
        ))
    }

    #[test]
    fn assemble_single_sphere_skips_scene_world_wrapper() {
        let world = SceneWorld::assemble(vec![sphere_at((0.0, 0.0, 0.0))], vec![]);
        let ray = ray_from((0.0, 0.0, -5.0), (0.0, 0.0, 1.0));
        assert!(world.hit(&ray, 0.001, f64::INFINITY).is_some());
    }

    #[test]
    fn assemble_single_plane_skips_scene_world_wrapper() {
        let world = SceneWorld::assemble(vec![], vec![Arc::new(floor_plane())]);
        let ray = ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0));
        assert!(world.hit(&ray, 0.001, f64::INFINITY).is_some());
    }

    #[test]
    fn plane_hit_beyond_finite_bvh_extent_is_not_culled() {
        let objects = vec![sphere_at((0.0, 1.0, 0.0))];
        let bvh_only = BvhNode::build(vec![
            Arc::new(floor_plane()),
            Arc::clone(&objects[0]),
        ]);
        let world = SceneWorld::assemble(objects, vec![Arc::new(floor_plane())]);

        let ray = ray_from((2500.0, 10.0, 2500.0), (0.0, -1.0, 0.0));
        assert!(
            bvh_only.hit(&ray, 0.001, f64::INFINITY).is_none(),
            "BVH with finite plane bounds should miss far-off ground hits"
        );
        let hit = world.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 10.0);
    }

    #[test]
    fn closest_hit_picks_sphere_in_front_of_infinite_plane() {
        let world = SceneWorld::assemble(
            vec![sphere_at((0.0, 1.0, 0.0))],
            vec![Arc::new(floor_plane())],
        );
        let ray = ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0));
        let hit = world.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 3.0);
    }

    #[test]
    fn any_hit_sees_plane_shadow_blocker_outside_bvh_plane_extent() {
        let world = SceneWorld::assemble(
            vec![sphere_at((0.0, 5.0, 0.0))],
            vec![Arc::new(floor_plane())],
        );
        let ray = ray_from((1500.0, 2.0, 1500.0), (0.0, -1.0, 0.0));
        assert!(world.any_hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn mixed_scene_matches_brute_force_intersection() {
        let bounded = vec![
            sphere_at((0.0, 1.0, 0.0)),
            sphere_at((-2.0, 0.5, 0.0)),
        ];
        let planes: Vec<Arc<dyn Hittable>> =
            vec![Arc::new(floor_plane()) as Arc<dyn Hittable>];
        let world = SceneWorld::assemble(bounded.clone(), planes.clone());

        let mut all = planes;
        all.extend(bounded);

        let rays = [
            ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0)),
            ray_from((1800.0, 4.0, 0.0), (0.0, -1.0, 0.0)),
            ray_from((0.0, 2.0, 0.0), (1.0, 0.0, 0.0)),
        ];

        for ray in rays {
            let expected = closest_hit_in_objects(&all, &ray, 0.001, f64::INFINITY);
            let actual = world.hit(&ray, 0.001, f64::INFINITY);
            match (&expected, &actual) {
                (None, None) => {}
                (Some(e), Some(a)) => assert_close(a.t, e.t),
                _ => panic!("world hit {:?} != brute force {:?}", actual, expected),
            }
        }
    }

    #[test]
    fn bounding_box_uses_bounded_geometry_when_present() {
        let world = SceneWorld::assemble(
            vec![sphere_at((0.0, 1.0, 0.0))],
            vec![plane_at_y(0.0)],
        );
        let bbox = world.bounding_box();
        assert!(bbox.max.y < 100.0, "bounded sphere should dominate the reported box");
    }
}
