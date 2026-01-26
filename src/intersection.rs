//! Shared ray intersection helpers used by the BVH and test suites.

use std::sync::Arc;

use crate::hittable::{HitRecord, Hittable};
use crate::ray::Ray;

/// Keep the nearer of two hit records by ray parameter `t`.
pub fn closest_hit(a: Option<HitRecord>, b: Option<HitRecord>) -> Option<HitRecord> {
    match (a, b) {
        (Some(left), Some(right)) => {
            if left.t <= right.t {
                Some(left)
            } else {
                Some(right)
            }
        }
        (Some(hit), None) | (None, Some(hit)) => Some(hit),
        (None, None) => None,
    }
}

/// Find the closest intersection among `objects` within `[t_min, t_max]`.
pub fn closest_hit_in_objects(
    objects: &[Arc<dyn Hittable>],
    ray: &Ray,
    t_min: f64,
    t_max: f64,
) -> Option<HitRecord> {
    let mut closest: Option<HitRecord> = None;
    let mut closest_t = t_max;
    for object in objects {
        if let Some(hit) = object.hit(ray, t_min, closest_t) {
            closest_t = hit.t;
            closest = Some(hit);
        }
    }
    closest
}

/// Return true when any object blocks the ray in `[t_min, t_max]`.
pub fn any_hit_in_objects(
    objects: &[Arc<dyn Hittable>],
    ray: &Ray,
    t_min: f64,
    t_max: f64,
) -> bool {
    objects
        .iter()
        .any(|object| object.any_hit(ray, t_min, t_max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_tests::{assert_close, ray_from, test_material, unit_sphere_at};
    use crate::vec3::Point3;

    #[test]
    fn closest_hit_prefers_smaller_t() {
        let material = test_material();
        let near = HitRecord {
            point: Point3::new(0.0, 0.0, 0.0),
            normal: Point3::new(0.0, 1.0, 0.0),
            t: 2.0,
            front_face: true,
            material: Arc::clone(&material),
        };
        let far = HitRecord {
            point: near.point,
            normal: near.normal,
            t: 5.0,
            front_face: near.front_face,
            material,
        };
        let chosen = closest_hit(Some(far), Some(near)).unwrap();
        assert_close(chosen.t, 2.0);
    }

    #[test]
    fn closest_hit_returns_some_when_only_one_side_hits() {
        let hit = HitRecord {
            point: Point3::default(),
            normal: Point3::new(0.0, 1.0, 0.0),
            t: 1.0,
            front_face: true,
            material: test_material(),
        };
        assert_close(closest_hit(Some(hit.clone()), None).unwrap().t, 1.0);
        assert_close(closest_hit(None, Some(hit)).unwrap().t, 1.0);
    }

    #[test]
    fn closest_hit_in_objects_skips_later_hits_beyond_current_closest() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0)),
            Arc::new(unit_sphere_at((10.0, 0.0, 0.0), 1.0)),
        ];
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        let hit = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 4.0);
    }

    #[test]
    fn any_hit_in_objects_stops_at_first_blocker() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0)),
            Arc::new(unit_sphere_at((10.0, 0.0, 0.0), 1.0)),
        ];
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert!(any_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY));
        assert!(!any_hit_in_objects(&objects, &ray, 0.001, 3.5));
    }

    #[test]
    fn closest_hit_in_objects_returns_none_for_empty_scene() {
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 0.0, 1.0));
        assert!(closest_hit_in_objects(&[], &ray, 0.001, f64::INFINITY).is_none());
    }

    #[test]
    fn closest_hit_returns_none_when_both_miss() {
        assert!(closest_hit(None, None).is_none());
    }

    #[test]
    fn closest_hit_equal_t_prefers_left_operand() {
        let material = test_material();
        let left = HitRecord {
            point: Point3::new(1.0, 0.0, 0.0),
            normal: Point3::new(0.0, 1.0, 0.0),
            t: 3.0,
            front_face: true,
            material: Arc::clone(&material),
        };
        let right = HitRecord {
            point: Point3::new(2.0, 0.0, 0.0),
            normal: Point3::new(0.0, 1.0, 0.0),
            t: 3.0,
            front_face: true,
            material,
        };
        let chosen = closest_hit(Some(left.clone()), Some(right)).unwrap();
        assert_close(chosen.point.x, left.point.x);
    }

    #[test]
    fn closest_hit_in_objects_is_order_independent() {
        let near = unit_sphere_at((0.0, 0.0, 0.0), 1.0);
        let far = unit_sphere_at((10.0, 0.0, 0.0), 1.0);
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));

        let forward: Vec<Arc<dyn Hittable>> =
            vec![Arc::new(near.clone()), Arc::new(far.clone())];
        let reversed: Vec<Arc<dyn Hittable>> = vec![Arc::new(far), Arc::new(near)];

        let a = closest_hit_in_objects(&forward, &ray, 0.001, f64::INFINITY).unwrap();
        let b = closest_hit_in_objects(&reversed, &ray, 0.001, f64::INFINITY).unwrap();
        assert_close(a.t, b.t);
        assert_close(a.t, 4.0);
    }

    #[test]
    fn any_hit_in_objects_returns_false_for_empty_scene() {
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 0.0, 1.0));
        assert!(!any_hit_in_objects(&[], &ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn closest_hit_in_objects_picks_near_sphere_when_spheres_overlap() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 2.0)),
            Arc::new(unit_sphere_at((1.0, 0.0, 0.0), 1.0)),
        ];
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        let hit = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 3.0);
    }

    #[test]
    fn closest_hit_in_objects_honors_t_min_when_near_hit_is_below_threshold() {
        let objects: Vec<Arc<dyn Hittable>> = vec![Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0))];
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        let hit = closest_hit_in_objects(&objects, &ray, 5.0, f64::INFINITY).unwrap();
        assert_close(hit.t, 6.0);
    }

    #[test]
    fn any_hit_in_objects_is_false_when_only_intersection_lies_beyond_t_max() {
        let objects: Vec<Arc<dyn Hittable>> = vec![Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0))];
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert!(!any_hit_in_objects(&objects, &ray, 0.001, 3.5));
    }
}
