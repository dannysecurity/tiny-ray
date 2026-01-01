use std::sync::Arc;

use crate::hittable::{set_face_normal, Aabb, HitRecord, Hittable};
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::{Point3, Vec3};

/// Infinite plane defined by a point on the surface and an outward-facing normal.
#[derive(Clone, Debug)]
pub struct Plane {
    pub point: Point3,
    pub normal: Vec3,
    pub material: Arc<Material>,
}

impl Plane {
    pub fn new(point: Point3, normal: Vec3, material: Arc<Material>) -> Self {
        Self {
            point,
            normal: normal.normalize(),
            material,
        }
    }

    /// Placeholder bounds for diagnostics; infinite planes are not BVH-culled via this box.
    fn thin_slab_bounds(&self) -> Aabb {
        let extent = 1000.0;
        let thickness = 0.001;
        let abs_n = Vec3::new(
            self.normal.x.abs(),
            self.normal.y.abs(),
            self.normal.z.abs(),
        );
        let half = Vec3::new(
            if abs_n.x > 0.9 { thickness } else { extent },
            if abs_n.y > 0.9 { thickness } else { extent },
            if abs_n.z > 0.9 { thickness } else { extent },
        );
        Aabb::new(self.point - half, self.point + half)
    }
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let denom = self.normal.dot(ray.direction);
        if denom.abs() < 1e-8 {
            return None;
        }

        let t = (self.point - ray.origin).dot(self.normal) / denom;
        if t < t_min || t > t_max {
            return None;
        }

        let point = ray.at(t);
        let outward_normal = self.normal;
        let mut record = HitRecord {
            point,
            normal: outward_normal,
            t,
            front_face: false,
            material: Arc::clone(&self.material),
        };
        set_face_normal(&mut record, ray, outward_normal);
        Some(record)
    }

    fn bounding_box(&self) -> Aabb {
        self.thin_slab_bounds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Material;
    use crate::vec3::Color;

    fn floor_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        )
    }

    #[test]
    fn ray_hits_floor_from_above() {
        let plane = floor_plane();
        let ray = Ray::new(
            Point3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            0.0,
        );
        let hit = plane.hit(&ray, 0.001, f64::INFINITY).unwrap();
        assert!((hit.t - 1.0).abs() < 1e-9);
        assert!(hit.front_face);
    }

    #[test]
    fn parallel_ray_misses() {
        let plane = floor_plane();
        let ray = Ray::new(
            Point3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.0,
        );
        assert!(plane.hit(&ray, 0.001, f64::INFINITY).is_none());
    }

    #[test]
    fn hit_respects_t_max() {
        let plane = floor_plane();
        let ray = Ray::new(
            Point3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            0.0,
        );
        assert!(plane.hit(&ray, 0.001, 1.5).is_none());
        assert!(plane.hit(&ray, 0.001, 2.5).is_some());
    }

    #[test]
    fn bounding_box_is_thin_along_normal() {
        let plane = floor_plane();
        let bbox = plane.bounding_box();
        assert!((bbox.max.y - bbox.min.y) < 0.01);
        assert!((bbox.max.x - bbox.min.x) > 100.0);
    }
}
