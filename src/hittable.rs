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

fn slab_interval(
    origin: f64,
    direction: f64,
    axis_min: f64,
    axis_max: f64,
) -> (f64, f64) {
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
