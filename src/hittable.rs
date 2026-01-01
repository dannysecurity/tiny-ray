use std::sync::Arc;

use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::{Point3, Vec3};

#[derive(Clone, Debug)]
pub struct HitRecord {
    pub point: Point3,
    pub normal: Vec3,
    pub t: f64,
    pub front_face: bool,
    pub material: Arc<Material>,
}

pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
    fn bounding_box(&self) -> Aabb;

    /// Return true when any surface blocks the ray in `[t_min, t_max]`.
    /// BVH nodes override this to exit early without building a full hit record.
    fn any_hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> bool {
        self.hit(ray, t_min, t_max).is_some()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    /// Span along each axis from minimum to maximum corner.
    pub fn extent(self) -> Vec3 {
        self.max - self.min
    }

    /// Index of the longest axis (0 = x, 1 = y, 2 = z).
    pub fn longest_axis(self) -> usize {
        let extent = self.extent();
        if extent.x > extent.y && extent.x > extent.z {
            0
        } else if extent.y > extent.z {
            1
        } else {
            2
        }
    }

    /// Surface area of the box faces, used by SAH BVH construction.
    pub fn surface_area(self) -> f64 {
        let extent = self.extent();
        2.0 * (extent.x * extent.y + extent.y * extent.z + extent.z * extent.x)
    }

    pub fn hit(&self, ray: &Ray, mut t_min: f64, mut t_max: f64) -> bool {
        for axis in 0..3 {
            let (origin, direction, axis_min, axis_max) =
                ray_axis_bounds(ray, self, axis);
            let (t_near, t_far) = slab_interval(origin, direction, axis_min, axis_max);
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);
            if t_max <= t_min {
                return false;
            }
        }
        true
    }

    pub fn surrounding_box(boxes: &[Aabb]) -> Self {
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for b in boxes {
            min.x = min.x.min(b.min.x);
            min.y = min.y.min(b.min.y);
            min.z = min.z.min(b.min.z);
            max.x = max.x.max(b.max.x);
            max.y = max.y.max(b.max.y);
            max.z = max.z.max(b.max.z);
        }
        Self { min, max }
    }
}

fn ray_axis_bounds(ray: &Ray, bbox: &Aabb, axis: usize) -> (f64, f64, f64, f64) {
    (
        ray.origin.axis(axis),
        ray.direction.axis(axis),
        bbox.min.axis(axis),
        bbox.max.axis(axis),
    )
}

/// Rays nearly parallel to a slab face must not divide by a vanishing direction.
const RAY_DIRECTION_EPSILON: f64 = 1e-8;

fn slab_interval(
    origin: f64,
    direction: f64,
    axis_min: f64,
    axis_max: f64,
) -> (f64, f64) {
    if direction.abs() < RAY_DIRECTION_EPSILON {
        if origin < axis_min || origin > axis_max {
            return (f64::INFINITY, f64::NEG_INFINITY);
        }
        return (f64::NEG_INFINITY, f64::INFINITY);
    }

    let inv_direction = 1.0 / direction;
    let mut t_near = (axis_min - origin) * inv_direction;
    let mut t_far = (axis_max - origin) * inv_direction;
    if inv_direction < 0.0 {
        std::mem::swap(&mut t_near, &mut t_far);
    }
    (t_near, t_far)
}

pub fn set_face_normal(record: &mut HitRecord, ray: &Ray, outward_normal: Vec3) {
    record.front_face = ray.direction.dot(outward_normal) < 0.0;
    record.normal = if record.front_face {
        outward_normal
    } else {
        -outward_normal
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_tests::{assert_close, assert_vec3_close, ray_from, test_material};

    fn unit_cube() -> Aabb {
        Aabb::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0))
    }

    #[test]
    fn extent_reports_axis_spans() {
        let bbox = Aabb::new(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 6.0, 8.0));
        assert_vec3_close(bbox.extent(), Vec3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn longest_axis_prefers_largest_extent() {
        let x_long = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(5.0, 1.0, 1.0));
        let y_long = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 5.0, 1.0));
        let z_long = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 5.0));
        assert_eq!(x_long.longest_axis(), 0);
        assert_eq!(y_long.longest_axis(), 1);
        assert_eq!(z_long.longest_axis(), 2);
    }

    #[test]
    fn surface_area_sums_opposing_face_pairs() {
        let bbox = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 3.0, 4.0));
        assert_close(bbox.surface_area(), 52.0);
    }

    #[test]
    fn surrounding_box_unions_child_bounds() {
        let a = Aabb::new(Point3::new(-2.0, 0.0, 0.0), Point3::new(-1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(1.0, -3.0, 2.0), Point3::new(4.0, 0.0, 5.0));
        let merged = Aabb::surrounding_box(&[a, b]);
        assert_vec3_close(merged.min, Point3::new(-2.0, -3.0, 0.0));
        assert_vec3_close(merged.max, Point3::new(4.0, 1.0, 5.0));
    }

    #[test]
    fn aabb_hit_accepts_ray_through_center() {
        let bbox = unit_cube();
        let ray = ray_from((0.0, 0.0, -5.0), (0.0, 0.0, 1.0));
        assert!(bbox.hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn aabb_hit_rejects_ray_missing_box() {
        let bbox = unit_cube();
        let ray = ray_from((0.0, 5.0, 0.0), (1.0, 0.0, 0.0));
        assert!(!bbox.hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn aabb_hit_handles_negative_direction() {
        let bbox = unit_cube();
        let ray = ray_from((0.0, 0.0, 5.0), (0.0, 0.0, -1.0));
        assert!(bbox.hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn aabb_hit_honors_t_interval() {
        let bbox = unit_cube();
        let ray = ray_from((0.0, 0.0, -5.0), (0.0, 0.0, 1.0));
        assert!(!bbox.hit(&ray, 0.001, 3.0));
        assert!(bbox.hit(&ray, 0.001, 5.0));
    }

    #[test]
    fn aabb_hit_accepts_ray_inside_box_with_near_zero_direction() {
        let bbox = unit_cube();
        let ray = ray_from((0.0, 0.0, 0.0), (1e-12, 0.0, 1.0));
        assert!(bbox.hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn aabb_hit_rejects_ray_outside_box_with_near_zero_direction() {
        let bbox = unit_cube();
        let ray = ray_from((5.0, 0.0, 0.0), (1e-12, 0.0, 1.0));
        assert!(!bbox.hit(&ray, 0.001, f64::INFINITY));
    }

    #[test]
    fn set_face_normal_orients_toward_ray_origin() {
        let outward = Vec3::new(0.0, 1.0, 0.0);
        let mut record = HitRecord {
            point: Point3::new(0.0, 0.0, 0.0),
            normal: outward,
            t: 1.0,
            front_face: false,
            material: test_material(),
        };

        let front_ray = ray_from((0.0, 1.0, 0.0), (0.0, -1.0, 0.0));
        set_face_normal(&mut record, &front_ray, outward);
        assert!(record.front_face);
        assert_vec3_close(record.normal, outward);

        let back_ray = ray_from((0.0, -1.0, 0.0), (0.0, 1.0, 0.0));
        set_face_normal(&mut record, &back_ray, outward);
        assert!(!record.front_face);
        assert_vec3_close(record.normal, -outward);
    }
}
