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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_tests::{assert_close, assert_vec3_close, ray_from, test_material};

    fn unit_sphere() -> Sphere {
        Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0, test_material())
    }

    #[test]
    fn ray_through_center_hits_entry_point() {
        let sphere = unit_sphere();
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 4.0);
        assert_vec3_close(hit.point, Point3::new(-1.0, 0.0, 0.0));
        assert!(hit.front_face);
        assert_vec3_close(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn grazing_ray_hits_tangent_point() {
        let sphere = unit_sphere();
        let ray = ray_from((-1.0, 1.0, 0.0), (1.0, 0.0, 0.0));
        let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 1.0);
        assert_vec3_close(hit.point, Point3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn offset_ray_misses_sphere() {
        let sphere = unit_sphere();
        let ray = ray_from((-5.0, 2.0, 0.0), (1.0, 0.0, 0.0));
        assert!(sphere.hit(&ray, 0.001, f64::INFINITY).is_none());
    }

    #[test]
    fn ray_starting_inside_exits_through_far_side() {
        let sphere = unit_sphere();
        let ray = ray_from((0.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert_close(hit.t, 1.0);
        assert!(!hit.front_face);
        assert_vec3_close(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn hit_respects_t_max_interval() {
        let sphere = unit_sphere();
        let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert!(sphere.hit(&ray, 0.001, 3.5).is_none());
        assert!(sphere.hit(&ray, 0.001, 4.5).is_some());
    }

    #[test]
    fn hit_respects_t_min_interval() {
        let sphere = unit_sphere();
        let ray = ray_from((0.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert!(sphere.hit(&ray, 1.5, f64::INFINITY).is_none());
        assert!(sphere.hit(&ray, 0.5, f64::INFINITY).is_some());
    }

    #[test]
    fn bounding_box_wraps_center_and_radius() {
        let sphere = Sphere::new(Point3::new(1.0, -2.0, 3.0), 2.0, test_material());
        let bbox = sphere.bounding_box();
        assert_vec3_close(bbox.min, Point3::new(-1.0, -4.0, 1.0));
        assert_vec3_close(bbox.max, Point3::new(3.0, 0.0, 5.0));
    }
}
