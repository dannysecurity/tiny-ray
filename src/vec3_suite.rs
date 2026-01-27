//! Algebraic and geometric property tests for [`Vec3`] and intersection invariants.

use crate::geometry_tests::{
    assert_close, assert_length_close, assert_orthogonal, assert_parallel, assert_unit,
    assert_vec3_close, property_test_vectors, EPS,
};
use crate::vec3::Vec3;

#[test]
fn add_is_commutative() {
    let [a, b, _, _] = property_test_vectors();
    assert_vec3_close(a + b, b + a);
}

#[test]
fn add_is_associative() {
    let [a, b, c, _] = property_test_vectors();
    assert_vec3_close((a + b) + c, a + (b + c));
}

#[test]
fn scalar_multiplication_distributes_over_addition() {
    let [a, b, _, _] = property_test_vectors();
    let s = 2.5;
    assert_vec3_close(s * (a + b), s * a + s * b);
}

#[test]
fn cross_is_anti_commutative() {
    let [a, b, _, _] = property_test_vectors();
    assert_vec3_close(a.cross(b), -(b.cross(a)));
}

#[test]
fn cross_of_parallel_vectors_is_zero() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert_parallel(v, v * 3.0);
    assert!(v.cross(v * 3.0).near_zero());
}

#[test]
fn dot_self_equals_length_squared() {
    for v in property_test_vectors() {
        assert_close(v.dot(v), v.length_squared());
    }
}

#[test]
fn normalize_is_idempotent() {
    for v in property_test_vectors() {
        let n = v.normalize();
        assert_vec3_close(n.normalize(), n);
        assert_length_close(n, 1.0);
    }
}

#[test]
fn triple_product_scalar_identity() {
    let [a, b, c, _] = property_test_vectors();
    let scalar = a.dot(b.cross(c));
    assert_close(scalar, b.dot(c.cross(a)));
    assert_close(scalar, c.dot(a.cross(b)));
}

#[test]
fn reflect_preserves_magnitude_and_incidence_angle() {
    let incident = Vec3::new(0.6, -0.8, 0.0);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let reflected = incident.reflect(normal);
    assert_close(incident.length(), reflected.length());
    assert_close(
        (-incident).dot(normal).abs(),
        reflected.dot(normal).abs(),
    );
}

#[test]
fn reflect_across_normal_inverts_normal_component() {
    let incident = Vec3::new(1.0, -2.0, 3.0);
    let normal = Vec3::new(0.0, 1.0, 0.0).normalize();
    let reflected = incident.reflect(normal);
    let n = incident.dot(normal);
    let r = reflected.dot(normal);
    assert_close(n, -r);
}

#[test]
fn refract_oblique_incidence_satisfies_snells_law() {
    let eta = 1.0 / 1.5;
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let incident = Vec3::new(0.2, -0.8, 0.0).normalize();
    let refracted = incident.refract(normal, eta).unwrap();
    let sin_i = incident.cross(normal).length();
    let sin_t = refracted.cross(normal).length();
    assert_close(sin_i / sin_t, 1.0 / eta);
}

#[test]
fn negation_is_self_inverse() {
    for v in property_test_vectors() {
        assert_vec3_close(-(-v), v);
    }
}

#[test]
fn component_mul_is_commutative() {
    let [a, b, _, _] = property_test_vectors();
    assert_vec3_close(a * b, b * a);
}

#[test]
fn lerp_is_linear_between_endpoints() {
    let a = Vec3::new(-1.0, 0.0, 2.0);
    let b = Vec3::new(3.0, 4.0, -2.0);
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let expected = a * (1.0 - t) + b * t;
        assert_vec3_close(a.lerp(b, t), expected);
    }
}

#[test]
fn approx_eq_matches_geometry_tests_epsilon() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(1.0 + EPS / 2.0, 2.0, 3.0);
    assert!(a.approx_eq(b, EPS));
    assert!(!a.approx_eq(b, EPS / 4.0));
}

#[test]
fn orthonormal_basis_has_unit_length_and_is_orthogonal() {
    let x = Vec3::new(1.0, 0.0, 0.0);
    let y = Vec3::new(0.0, 1.0, 0.0);
    let z = Vec3::new(0.0, 0.0, 1.0);
    for v in [x, y, z] {
        assert_length_close(v, 1.0);
    }
    assert_orthogonal(x, y);
    assert_orthogonal(y, z);
    assert_orthogonal(z, x);
    assert_vec3_close(x.cross(y), z);
}

#[test]
fn axis_aligned_vectors_select_correct_components() {
    let v = Vec3::new(7.0, 8.0, 9.0);
    assert_close(v.axis(0), 7.0);
    assert_close(v.axis(1), 8.0);
    assert_close(v.axis(2), 9.0);
}

#[test]
fn default_is_origin() {
    assert_vec3_close(Vec3::default(), Vec3::new(0.0, 0.0, 0.0));
}

#[test]
fn sub_is_inverse_of_add() {
    let [a, b, _, _] = property_test_vectors();
    assert_vec3_close((a + b) - b, a);
}

#[test]
fn scalar_division_reverses_multiplication() {
    for v in property_test_vectors() {
        let s = 4.0;
        assert_vec3_close((v * s) / s, v);
    }
}

#[test]
fn lagrange_identity_links_dot_and_cross() {
    for [a, b, _, _] in [property_test_vectors()] {
        let cross_len_sq = a.cross(b).length_squared();
        let dot_sq = a.dot(b).powi(2);
        let expected = a.length_squared() * b.length_squared();
        assert_close(cross_len_sq + dot_sq, expected);
    }
}

#[test]
fn cross_magnitude_matches_sine_of_angle() {
    let a = Vec3::new(3.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 4.0, 0.0);
    assert_close(a.cross(b).length(), a.length() * b.length());
}

#[test]
fn reflect_twice_restores_incident_along_unit_normal() {
    let incident = Vec3::new(0.6, -0.8, 0.2);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let once = incident.reflect(normal);
    assert_vec3_close(once.reflect(normal), incident);
}

#[test]
fn refract_preserves_unit_length_for_transmitted_rays() {
    let eta = 1.0 / 1.5;
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let incident = Vec3::new(0.3, -0.7, 0.1).normalize();
    let refracted = incident.refract(normal, eta).unwrap();
    assert_unit(refracted);
}

#[test]
fn lerp_extrapolates_beyond_endpoints() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    assert_vec3_close(a.lerp(b, 2.0), a * -1.0 + b * 2.0);
    assert_vec3_close(a.lerp(b, -0.5), a * 1.5 + b * -0.5);
}

#[test]
fn partial_eq_requires_exact_component_match() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(1.0, 2.0, 3.0);
    let c = Vec3::new(1.0 + EPS, 2.0, 3.0);
    assert_eq!(a, b);
    assert_ne!(a, c);
}
