use std::sync::Arc;

use crate::hittable::{Aabb, HitRecord, Hittable};
use crate::intersection::{closest_hit, closest_hit_in_objects};
use crate::ray::Ray;

#[derive(Clone)]
pub enum BvhNode {
    Leaf {
        objects: Vec<Arc<dyn Hittable>>,
        bbox: Aabb,
    },
    Branch {
        left: Arc<BvhNode>,
        right: Arc<BvhNode>,
        bbox: Aabb,
    },
}

impl BvhNode {
    fn bbox(&self) -> Aabb {
        match self {
            BvhNode::Leaf { bbox, .. } | BvhNode::Branch { bbox, .. } => *bbox,
        }
    }

    pub fn build(mut objects: Vec<Arc<dyn Hittable>>) -> Self {
        let bbox = Aabb::surrounding_box(&objects.iter().map(|o| o.bounding_box()).collect::<Vec<_>>());

        if objects.len() <= 2 {
            return Self::Leaf { objects, bbox };
        }

        let axis = bbox.longest_axis();

        objects.sort_by(|a, b| {
            let va = a.bounding_box().min.axis(axis);
            let vb = b.bounding_box().min.axis(axis);
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mid = objects.len() / 2;
        let right = objects.split_off(mid);
        let left = BvhNode::build(objects);
        let right = BvhNode::build(right);
        let bbox = Aabb::surrounding_box(&[left.bbox(), right.bbox()]);

        Self::Branch {
            left: Arc::new(left),
            right: Arc::new(right),
            bbox,
        }
    }

}

impl Hittable for BvhNode {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        if !self.bbox().hit(ray, t_min, t_max) {
            return None;
        }

        match self {
            BvhNode::Leaf { objects, .. } => closest_hit_in_objects(objects, ray, t_min, t_max),
            BvhNode::Branch { left, right, .. } => {
                let hit_left = left.hit(ray, t_min, t_max);
                let t_far = hit_left.as_ref().map(|h| h.t).unwrap_or(t_max);
                let hit_right = right.hit(ray, t_min, t_far);
                closest_hit(hit_left, hit_right)
            }
        }
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::geometry_tests::unit_sphere_at;
    use crate::vec3::{Point3, Vec3};

    fn make_sphere(center: (f64, f64, f64), radius: f64) -> Arc<dyn Hittable> {
        Arc::new(unit_sphere_at(center, radius))
    }

    fn brute_force_hit(
        objects: &[Arc<dyn Hittable>],
        ray: &Ray,
        t_min: f64,
        t_max: f64,
    ) -> Option<HitRecord> {
        closest_hit_in_objects(objects, ray, t_min, t_max)
    }

    #[test]
    fn bvh_hit_matches_brute_force_for_several_spheres() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            make_sphere((-3.0, 0.0, 0.0), 1.0),
            make_sphere((0.0, 0.0, 0.0), 1.0),
            make_sphere((3.0, 0.0, 0.0), 1.0),
            make_sphere((0.0, 2.5, 0.0), 0.5),
            make_sphere((-1.5, -2.0, 1.0), 0.75),
        ];
        let bvh = BvhNode::build(objects.clone());

        let rays = [
            Ray::new(Point3::new(0.0, 0.0, -10.0), Vec3::new(0.0, 0.0, 1.0), 0.0),
            Ray::new(Point3::new(-3.0, 0.0, -10.0), Vec3::new(0.0, 0.0, 1.0), 0.0),
            Ray::new(Point3::new(8.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 0.0),
            Ray::new(Point3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 0.0),
            Ray::new(
                Point3::new(100.0, 100.0, 100.0),
                Vec3::new(-1.0, -1.0, -1.0).normalize(),
                0.0,
            ),
        ];

        for ray in rays {
            let expected = brute_force_hit(&objects, &ray, 0.001, f64::INFINITY);
            let actual = bvh.hit(&ray, 0.001, f64::INFINITY);
            match (&expected, &actual) {
                (None, None) => {}
                (Some(e), Some(a)) => assert!(
                    (e.t - a.t).abs() < 1e-9,
                    "BVH t={} != brute force t={} for ray {:?}",
                    a.t,
                    e.t,
                    ray
                ),
                _ => panic!(
                    "BVH hit={:?} != brute force hit={:?} for ray {:?}",
                    actual, expected, ray
                ),
            }
        }
    }

    #[test]
    fn bvh_honors_t_max_interval() {
        let objects: Vec<Arc<dyn Hittable>> = vec![make_sphere((0.0, 0.0, 0.0), 1.0)];
        let bvh = BvhNode::build(objects);

        let ray = Ray::new(Point3::new(-10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0);

        assert!(bvh.hit(&ray, 0.001, 5.0).is_none());
        assert!(bvh.hit(&ray, 0.001, 10.0).is_some());
    }

    #[test]
    fn bvh_root_bounding_box_encloses_all_objects() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            make_sphere((-2.0, 0.0, 0.0), 1.0),
            make_sphere((2.0, 0.0, 0.0), 1.0),
            make_sphere((0.0, 3.0, 0.0), 0.5),
        ];
        let bvh = BvhNode::build(objects.clone());
        let root_bbox = bvh.bounding_box();

        for object in objects {
            let bbox = object.bounding_box();
            assert!(root_bbox.min.x <= bbox.min.x);
            assert!(root_bbox.min.y <= bbox.min.y);
            assert!(root_bbox.min.z <= bbox.min.z);
            assert!(root_bbox.max.x >= bbox.max.x);
            assert!(root_bbox.max.y >= bbox.max.y);
            assert!(root_bbox.max.z >= bbox.max.z);
        }
    }
}
