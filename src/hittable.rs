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

    pub fn hit(&self, ray: &Ray, mut t_min: f64, mut t_max: f64) -> bool {
        for axis in 0..3 {
            let (origin, direction, min, max) = match axis {
                0 => (ray.origin.x, ray.direction.x, self.min.x, self.max.x),
                1 => (ray.origin.y, ray.direction.y, self.min.y, self.max.y),
                _ => (ray.origin.z, ray.direction.z, self.min.z, self.max.z),
            };

            let inv_direction = 1.0 / direction;
            let mut t0 = (min - origin) * inv_direction;
            let mut t1 = (max - origin) * inv_direction;
            if inv_direction < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }

            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
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

pub fn set_face_normal(record: &mut HitRecord, ray: &Ray, outward_normal: Vec3) {
    record.front_face = ray.direction.dot(outward_normal) < 0.0;
    record.normal = if record.front_face {
        outward_normal
    } else {
        -outward_normal
    };
}
