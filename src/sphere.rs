use std::sync::Arc;

use crate::hittable::{set_face_normal, Aabb, HitRecord, Hittable};
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::{Point3, Vec3};

#[derive(Clone, Debug)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f64,
    pub material: Arc<Material>,
}

impl Sphere {
    pub fn new(center: Point3, radius: f64, material: Arc<Material>) -> Self {
        Self {
            center,
            radius,
            material,
        }
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let mut root = (-half_b - sqrt_d) / a;
        if root < t_min || t_max < root {
            root = (-half_b + sqrt_d) / a;
            if root < t_min || t_max < root {
                return None;
            }
        }

        let point = ray.at(root);
        let outward_normal = (point - self.center) / self.radius;
        let mut record = HitRecord {
            point,
            normal: outward_normal,
            t: root,
            front_face: false,
            material: Arc::clone(&self.material),
        };
        set_face_normal(&mut record, ray, outward_normal);
        Some(record)
    }

    fn bounding_box(&self) -> Aabb {
        let r = Vec3::new(self.radius, self.radius, self.radius);
        Aabb::new(self.center - r, self.center + r)
    }
}
