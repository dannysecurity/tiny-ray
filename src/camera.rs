use rand::Rng;

use crate::ray::Ray;
use crate::vec3::{Point3, Vec3};

#[derive(Clone, Debug)]
pub struct Camera {
    pub origin: Point3,
    pub lower_left_corner: Point3,
    pub horizontal: Vec3,
    pub vertical: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    pub w: Vec3,
    pub lens_radius: f64,
}

impl Camera {
    pub fn new(
        lookfrom: Point3,
        lookat: Point3,
        vup: Vec3,
        vertical_fov_deg: f64,
        aspect_ratio: f64,
        aperture: f64,
        focus_dist: f64,
    ) -> Self {
        let theta = vertical_fov_deg.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect_ratio * viewport_height;

        let w = (lookfrom - lookat).normalize();
        let u = vup.cross(w).normalize();
        let v = w.cross(u);

        let origin = lookfrom;
        let horizontal = focus_dist * viewport_width * u;
        let vertical = focus_dist * viewport_height * v;
        let lower_left_corner = origin - horizontal / 2.0 - vertical / 2.0 - focus_dist * w;

        Self {
            origin,
            lower_left_corner,
            horizontal,
            vertical,
            u,
            v,
            w,
            lens_radius: aperture / 2.0,
        }
    }

    pub fn get_ray<R: Rng + ?Sized>(&self, rng: &mut R, s: f64, t: f64, time: f64) -> Ray {
        let rd = self.lens_radius * Vec3::random_in_unit_disk(rng);
        let offset = self.u * rd.x + self.v * rd.y;
        Ray::new(
            self.origin + offset,
            self.lower_left_corner + self.horizontal * s + self.vertical * t - self.origin - offset,
            time,
        )
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::geometry_tests::{assert_close, assert_vec3_close};

    fn test_camera() -> Camera {
        Camera::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
            90.0,
            1.0,
            0.0,
            1.0,
        )
    }

    #[test]
    fn pinhole_rays_share_fixed_origin() {
        let camera = test_camera();
        let mut rng = StdRng::seed_from_u64(0);
        for _ in 0..8 {
            let ray = camera.get_ray(&mut rng, 0.5, 0.5, 0.0);
            assert_vec3_close(ray.origin, camera.origin);
        }
    }

    #[test]
    fn pinhole_ray_hits_viewport_corners() {
        let camera = test_camera();
        let mut rng = StdRng::seed_from_u64(0);

        let bottom_left = camera.get_ray(&mut rng, 0.0, 0.0, 0.0);
        assert_vec3_close(bottom_left.direction, Vec3::new(-1.0, -1.0, -1.0));

        let top_right = camera.get_ray(&mut rng, 1.0, 1.0, 0.0);
        assert_vec3_close(top_right.direction, Vec3::new(1.0, 1.0, -1.0));
    }

    #[test]
    fn thin_lens_shifts_ray_origin_when_aperture_open() {
        let camera = Camera::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
            90.0,
            1.0,
            0.5,
            1.0,
        );
        assert!(camera.lens_radius > 0.0);

        let mut rng = StdRng::seed_from_u64(7);
        let mut shifted = false;
        for _ in 0..32 {
            let ray = camera.get_ray(&mut rng, 0.5, 0.5, 0.0);
            if (ray.origin - camera.origin).length() > 1e-6 {
                shifted = true;
                break;
            }
        }
        assert!(shifted, "expected lens sampling to offset ray origins");
    }

    #[test]
    fn get_ray_preserves_shutter_time() {
        let camera = test_camera();
        let mut rng = StdRng::seed_from_u64(0);
        let ray = camera.get_ray(&mut rng, 0.25, 0.75, 0.42);
        assert_close(ray.time, 0.42);
    }
}
