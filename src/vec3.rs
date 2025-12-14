use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use rand::Rng;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn from_array([x, y, z]: [f64; 3]) -> Self {
        Self::new(x, y, z)
    }

    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn normalize(self) -> Self {
        self / self.length()
    }

    pub fn near_zero(self) -> bool {
        self.x.abs() < 1e-8 && self.y.abs() < 1e-8 && self.z.abs() < 1e-8
    }

    /// Component along axis index (0 = x, 1 = y, 2 = z).
    pub fn axis(self, index: usize) -> f64 {
        match index {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }

    pub fn reflect(self, normal: Self) -> Self {
        self - normal * (2.0 * self.dot(normal))
    }

    pub fn refract(self, normal: Self, eta_ratio: f64) -> Option<Self> {
        let cos_theta = (-self).dot(normal).min(1.0);
        let r_out_perp = (self + normal * cos_theta) * eta_ratio;
        let r_out_perp_len_sq = r_out_perp.length_squared();
        if r_out_perp_len_sq > 1.0 {
            return None;
        }
        let r_out_parallel = normal * -(1.0 - r_out_perp_len_sq).sqrt();
        Some(r_out_perp + r_out_parallel)
    }

    pub fn random_in_unit_sphere<R: Rng + ?Sized>(rng: &mut R) -> Self {
        loop {
            let p = Self::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            );
            if p.length_squared() < 1.0 {
                return p;
            }
        }
    }

    pub fn random_unit_vector<R: Rng + ?Sized>(rng: &mut R) -> Self {
        Self::random_in_unit_sphere(rng).normalize()
    }

    pub fn random_in_hemisphere<R: Rng + ?Sized>(rng: &mut R, normal: Self) -> Self {
        let in_unit_sphere = Self::random_in_unit_sphere(rng);
        if in_unit_sphere.dot(normal) > 0.0 {
            in_unit_sphere
        } else {
            -in_unit_sphere
        }
    }

    pub fn random_in_unit_disk<R: Rng + ?Sized>(rng: &mut R) -> Self {
        loop {
            let p = Self::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
            if p.length_squared() < 1.0 {
                return p;
            }
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Vec3 {
        rhs * self
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        self * (1.0 / rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for Vec3 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        *self = *self * rhs;
    }
}

impl DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        *self = *self / rhs;
    }
}

pub type Color = Vec3;
pub type Point3 = Vec3;

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::geometry_tests::{assert_close, assert_length_close, assert_vec3_close};

    #[test]
    fn from_array_matches_component_constructor() {
        assert_vec3_close(
            Vec3::from_array([1.0, 2.0, 3.0]),
            Vec3::new(1.0, 2.0, 3.0),
        );
    }

    #[test]
    fn arithmetic_operators_combine_components() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_vec3_close(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_vec3_close(a - b, Vec3::new(-3.0, -3.0, -3.0));
        assert_vec3_close(a * b, Vec3::new(4.0, 10.0, 18.0));
        assert_vec3_close(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_vec3_close(2.0 * a, Vec3::new(2.0, 4.0, 6.0));
        assert_vec3_close(a / 2.0, Vec3::new(0.5, 1.0, 1.5));
        assert_vec3_close(-a, Vec3::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn assign_operators_update_in_place() {
        let mut v = Vec3::new(1.0, 2.0, 3.0);
        v += Vec3::new(1.0, 1.0, 1.0);
        assert_vec3_close(v, Vec3::new(2.0, 3.0, 4.0));
        v -= Vec3::new(1.0, 1.0, 1.0);
        assert_vec3_close(v, Vec3::new(1.0, 2.0, 3.0));
        v *= 2.0;
        assert_vec3_close(v, Vec3::new(2.0, 4.0, 6.0));
        v /= 2.0;
        assert_vec3_close(v, Vec3::new(1.0, 2.0, 3.0));
        v *= Vec3::new(2.0, 3.0, 4.0);
        assert_vec3_close(v, Vec3::new(2.0, 6.0, 12.0));
    }

    #[test]
    fn dot_and_cross_follow_orthonormal_basis() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        assert_close(x.dot(y), 0.0);
        assert_close(x.dot(x), 1.0);
        assert_vec3_close(x.cross(y), z);
        assert_vec3_close(y.cross(z), x);
    }

    #[test]
    fn length_and_normalize_use_pythagorean_triple() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert_close(v.length_squared(), 25.0);
        assert_close(v.length(), 5.0);
        assert_length_close(v.normalize(), 1.0);
        assert_vec3_close(v.normalize(), Vec3::new(0.6, 0.8, 0.0));
    }

    #[test]
    fn reflect_bounces_off_horizontal_surface() {
        let incident = Vec3::new(1.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        assert_vec3_close(incident.reflect(normal), Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn refract_transmits_through_interface() {
        let unit = Vec3::new(0.0, -1.0, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let eta = 1.0 / 1.5;
        let refracted = unit.refract(normal, eta).unwrap();
        assert!(refracted.y < 0.0);
        assert_length_close(refracted, 1.0);
    }

    #[test]
    fn refract_returns_none_on_total_internal_reflection() {
        let incident = Vec3::new(3.0_f64.sqrt() / 2.0, 0.5, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        assert!(incident.refract(normal, 1.5).is_none());
    }

    #[test]
    fn near_zero_detects_tiny_components() {
        assert!(Vec3::new(1e-9, 0.0, 0.0).near_zero());
        assert!(!Vec3::new(1e-7, 0.0, 0.0).near_zero());
    }

    #[test]
    fn axis_selects_components_and_falls_back_to_z() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_close(v.axis(0), 1.0);
        assert_close(v.axis(1), 2.0);
        assert_close(v.axis(2), 3.0);
        assert_close(v.axis(99), 3.0);
    }

    #[test]
    fn random_unit_vectors_have_unit_length_with_seeded_rng() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..64 {
            assert_length_close(Vec3::random_unit_vector(&mut rng), 1.0);
        }
    }

    #[test]
    fn random_in_hemisphere_faces_the_given_normal() {
        let mut rng = StdRng::seed_from_u64(7);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        for _ in 0..64 {
            let sample = Vec3::random_in_hemisphere(&mut rng, normal);
            assert!(sample.dot(normal) > 0.0);
        }
    }

    #[test]
    fn random_in_unit_disk_stays_in_xy_plane() {
        let mut rng = StdRng::seed_from_u64(11);
        for _ in 0..64 {
            let sample = Vec3::random_in_unit_disk(&mut rng);
            assert_close(sample.z, 0.0);
            assert!(sample.length_squared() < 1.0);
        }
    }

    #[test]
    fn random_in_unit_sphere_stays_inside_ball() {
        let mut rng = StdRng::seed_from_u64(19);
        for _ in 0..64 {
            let sample = Vec3::random_in_unit_sphere(&mut rng);
            assert!(sample.length_squared() < 1.0);
        }
    }

    #[test]
    fn refract_with_matching_index_returns_transmitted_direction() {
        let incident = Vec3::new(0.2, -0.8, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = incident.refract(normal, 1.0).unwrap();
        assert_length_close(refracted, 1.0);
        assert_vec3_close(refracted, incident);
    }

    #[test]
    fn reflect_preserves_length_for_unit_incident() {
        let incident = Vec3::new(1.0, -1.0, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        assert_length_close(incident.reflect(normal), 1.0);
    }
}

