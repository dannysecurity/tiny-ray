use crate::vec3::{Point3, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vec3,
    pub time: f64,
}

impl Ray {
    pub fn new(origin: Point3, direction: Vec3, time: f64) -> Self {
        Self {
            origin,
            direction,
            time,
        }
    }

    pub fn at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_tests::{assert_close, assert_vec3_close, ray_from};

    #[test]
    fn at_interpolates_along_direction() {
        let ray = ray_from((0.0, 0.0, 0.0), (1.0, 2.0, 3.0));
        assert_vec3_close(ray.at(2.0), Point3::new(2.0, 4.0, 6.0));
        assert_vec3_close(ray.at(0.0), ray.origin);
    }

    #[test]
    fn at_respects_non_zero_origin() {
        let ray = Ray::new(Point3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert_vec3_close(ray.at(4.0), Point3::new(1.0, 6.0, 3.0));
        assert_close(ray.time, 0.5);
    }
}
