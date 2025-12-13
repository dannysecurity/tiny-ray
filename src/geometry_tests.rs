//! Shared fixtures and float assertions for geometry and intersection unit tests.

use std::sync::Arc;

use crate::material::Material;
use crate::plane::Plane;
use crate::ray::Ray;
use crate::sphere::Sphere;
use crate::vec3::{Color, Point3, Vec3};

pub const EPS: f64 = 1e-9;

pub fn assert_close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < EPS,
        "expected {b}, got {a} (delta {})",
        (a - b).abs()
    );
}

pub fn assert_vec3_close(a: Vec3, b: Vec3) {
    assert_close(a.x, b.x);
    assert_close(a.y, b.y);
    assert_close(a.z, b.z);
}

pub fn assert_length_close(v: Vec3, expected: f64) {
    assert_close(v.length(), expected);
}

pub fn test_material() -> Arc<Material> {
    Arc::new(Material::Lambertian {
        albedo: Color::new(0.5, 0.5, 0.5),
    })
}

pub fn ray_from(origin: (f64, f64, f64), direction: (f64, f64, f64)) -> Ray {
    Ray::new(
        Point3::new(origin.0, origin.1, origin.2),
        Vec3::new(direction.0, direction.1, direction.2),
        0.0,
    )
}

pub fn unit_sphere_at(center: (f64, f64, f64), radius: f64) -> Sphere {
    Sphere::new(
        Point3::new(center.0, center.1, center.2),
        radius,
        test_material(),
    )
}

pub fn floor_plane() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        test_material(),
    )
}

/// Plane with normal (0, 1, 1) normalized — exercises non-axis-aligned intersections.
pub fn diagonal_plane() -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 1.0),
        test_material(),
    )
}
