use std::sync::Arc;

use crate::hittable::{Aabb, HitRecord, Hittable};
use crate::intersection::{any_hit_in_objects, closest_hit, closest_hit_in_objects};
use crate::ray::Ray;
use crate::vec3::Point3;

const SAH_NUM_BINS: usize = 12;
const SAH_TRAVERSAL_COST: f64 = 0.125;
const SAH_INTERSECTION_COST: f64 = 1.0;

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

#[derive(Clone, Copy)]
struct SahBin {
    count: usize,
    bbox: Aabb,
}

impl Default for SahBin {
    fn default() -> Self {
        Self {
            count: 0,
            bbox: Aabb::new(
                Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
                Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
        }
    }
}

impl SahBin {
    fn add(&mut self, bbox: Aabb) {
        self.count += 1;
        self.bbox = if self.count == 1 {
            bbox
        } else {
            Aabb::surrounding_box(&[self.bbox, bbox])
        };
    }
}

struct SahSplit {
    axis: usize,
    position: f64,
    cost: f64,
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

        let split = find_sah_split(&objects, bbox).unwrap_or_else(|| median_split(&objects, bbox));

        let axis = split.axis;
        let position = split.position;
        objects.sort_by(|a, b| {
            let ca = centroid(a.bounding_box(), axis);
            let cb = centroid(b.bounding_box(), axis);
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut mid = objects
            .iter()
            .position(|object| centroid(object.bounding_box(), axis) >= position)
            .unwrap_or(objects.len());
        if mid == 0 || mid == objects.len() {
            mid = objects.len() / 2;
        }

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

fn centroid(bbox: Aabb, axis: usize) -> f64 {
    (bbox.min.axis(axis) + bbox.max.axis(axis)) * 0.5
}

fn find_sah_split(objects: &[Arc<dyn Hittable>], bbox: Aabb) -> Option<SahSplit> {
    let parent_area = bbox.surface_area();
    if parent_area <= f64::EPSILON {
        return None;
    }

    let mut best = SahSplit {
        axis: 0,
        position: 0.0,
        cost: f64::INFINITY,
    };

    for axis in 0..3 {
        let axis_min = bbox.min.axis(axis);
        let axis_max = bbox.max.axis(axis);
        let extent = axis_max - axis_min;
        if extent <= f64::EPSILON {
            continue;
        }

        let mut bins = [SahBin::default(); SAH_NUM_BINS];
        for object in objects {
            let object_bbox = object.bounding_box();
            let c = centroid(object_bbox, axis);
            let mut bin_idx = (((c - axis_min) / extent) * SAH_NUM_BINS as f64) as usize;
            if bin_idx >= SAH_NUM_BINS {
                bin_idx = SAH_NUM_BINS - 1;
            }
            bins[bin_idx].add(object_bbox);
        }

        for split in 1..SAH_NUM_BINS {
            let mut left_count = 0usize;
            let mut left_bbox = None::<Aabb>;
            for bin in &bins[..split] {
                if bin.count == 0 {
                    continue;
                }
                left_count += bin.count;
                left_bbox = Some(match left_bbox {
                    None => bin.bbox,
                    Some(existing) => Aabb::surrounding_box(&[existing, bin.bbox]),
                });
            }

            let mut right_count = 0usize;
            let mut right_bbox = None::<Aabb>;
            for bin in &bins[split..] {
                if bin.count == 0 {
                    continue;
                }
                right_count += bin.count;
                right_bbox = Some(match right_bbox {
                    None => bin.bbox,
                    Some(existing) => Aabb::surrounding_box(&[existing, bin.bbox]),
                });
            }

            if left_count == 0 || right_count == 0 {
                continue;
            }

            let left_area = left_bbox.expect("left bbox set when count > 0").surface_area();
            let right_area = right_bbox.expect("right bbox set when count > 0").surface_area();
            let cost = SAH_TRAVERSAL_COST
                + SAH_INTERSECTION_COST
                    * (left_count as f64 * left_area / parent_area
                        + right_count as f64 * right_area / parent_area);

            if cost < best.cost {
                best = SahSplit {
                    axis,
                    position: axis_min + extent * (split as f64 / SAH_NUM_BINS as f64),
                    cost,
                };
            }
        }
    }

    if best.cost.is_finite() {
        Some(best)
    } else {
        None
    }
}

fn median_split(objects: &[Arc<dyn Hittable>], bbox: Aabb) -> SahSplit {
    let axis = bbox.longest_axis();
    SahSplit {
        axis,
        position: centroid(objects[objects.len() / 2].bounding_box(), axis),
        cost: f64::INFINITY,
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

    fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        if !self.bbox().hit(ray, t_min, t_max) {
            return false;
        }

        match self {
            BvhNode::Leaf { objects, .. } => any_hit_in_objects(objects, ray, t_min, t_max),
            BvhNode::Branch { left, right, .. } => {
                left.any_hit(ray, t_min, t_max) || right.any_hit(ray, t_min, t_max)
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
    use crate::geometry_tests::{floor_plane, unit_sphere_at};
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

    fn assert_bvh_matches_brute_force(
        objects: &[Arc<dyn Hittable>],
        bvh: &BvhNode,
        rays: &[Ray],
    ) {
        for ray in rays {
            let expected = brute_force_hit(objects, ray, 0.001, f64::INFINITY);
            let actual = bvh.hit(ray, 0.001, f64::INFINITY);
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

        assert_bvh_matches_brute_force(&objects, &bvh, &rays);
    }

    #[test]
    fn bvh_hit_matches_brute_force_for_mixed_sphere_and_plane_scene() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(floor_plane()),
            make_sphere((0.0, 1.0, 0.0), 1.0),
            make_sphere((-2.0, 0.5, 0.0), 0.5),
            make_sphere((2.0, 0.5, 0.0), 0.5),
        ];
        let bvh = BvhNode::build(objects.clone());

        let rays = [
            Ray::new(Point3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 0.0),
            Ray::new(Point3::new(0.0, 2.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 0.0),
            Ray::new(Point3::new(-5.0, 0.5, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0),
            Ray::new(Point3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0),
        ];

        assert_bvh_matches_brute_force(&objects, &bvh, &rays);
    }

    #[test]
    fn bvh_any_hit_matches_brute_force_visibility() {
        let objects: Vec<Arc<dyn Hittable>> = vec![
            Arc::new(floor_plane()),
            make_sphere((0.0, 1.0, 0.0), 1.0),
            make_sphere((6.0, 0.0, 0.0), 1.0),
        ];
        let bvh = BvhNode::build(objects.clone());

        let rays = [
            Ray::new(Point3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 0.0),
            Ray::new(Point3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0),
            Ray::new(Point3::new(-5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0),
        ];

        for ray in rays {
            let brute = any_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY);
            assert_eq!(bvh.any_hit(&ray, 0.001, f64::INFINITY), brute);
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
    fn bvh_any_hit_honors_t_max_interval() {
        let objects: Vec<Arc<dyn Hittable>> = vec![make_sphere((0.0, 0.0, 0.0), 1.0)];
        let bvh = BvhNode::build(objects);

        let ray = Ray::new(Point3::new(-10.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.0);

        assert!(!bvh.any_hit(&ray, 0.001, 5.0));
        assert!(bvh.any_hit(&ray, 0.001, 10.0));
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
