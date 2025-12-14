//! Cross-primitive intersection scenarios exercising vec3 math and shared helpers.

use std::sync::Arc;

use crate::bvh::BvhNode;
use crate::geometry_tests::{
    assert_close, assert_length_close, assert_vec3_close, diagonal_plane, floor_plane,
    ray_from, test_material, unit_sphere_at,
};
use crate::hittable::{Aabb, Hittable};
use crate::intersection::{any_hit_in_objects, closest_hit_in_objects};
use crate::plane::Plane;
use crate::vec3::{Point3, Vec3};

fn scene(objects: Vec<Arc<dyn Hittable>>) -> Vec<Arc<dyn Hittable>> {
    objects
}

#[test]
fn closest_hit_picks_near_sphere_over_far_sphere() {
    let objects = scene(vec![
        Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0)),
        Arc::new(unit_sphere_at((8.0, 0.0, 0.0), 1.0)),
    ]);
    let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
    let hit = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).unwrap();
    assert_close(hit.t, 4.0);
    assert_vec3_close(hit.point, Point3::new(-1.0, 0.0, 0.0));
}

#[test]
fn closest_hit_picks_sphere_in_front_of_floor() {
    let objects = scene(vec![
        Arc::new(floor_plane()),
        Arc::new(unit_sphere_at((0.0, 1.0, 0.0), 1.0)),
    ]);
    let ray = ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0));
    let hit = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).unwrap();
    assert_close(hit.t, 3.0);
    assert_vec3_close(hit.point, Point3::new(0.0, 2.0, 0.0));
    assert!(hit.front_face);
}

#[test]
fn closest_hit_reaches_floor_when_sphere_is_above_ray() {
    let objects = scene(vec![
        Arc::new(floor_plane()),
        Arc::new(unit_sphere_at((0.0, 5.0, 0.0), 1.0)),
    ]);
    let ray = ray_from((0.0, 3.0, 0.0), (0.0, -1.0, 0.0));
    let hit = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).unwrap();
    assert_close(hit.t, 3.0);
    assert_vec3_close(hit.point, Point3::new(0.0, 0.0, 0.0));
}

#[test]
fn diagonal_plane_hit_point_lies_on_surface() {
    let plane = diagonal_plane();
    let ray = ray_from((0.0, 2.0, 0.0), (0.0, -1.0, 0.0));
    let hit = plane.hit(&ray, 0.001, f64::INFINITY).unwrap();
    assert_close(hit.t, 2.0);
    assert_close(hit.point.y, 0.0);
    assert_close(hit.point.x, hit.point.z);
    assert!(hit.front_face);
    assert_length_close(hit.normal, 1.0);
}

#[test]
fn diagonal_plane_back_face_flips_normal() {
    let plane = diagonal_plane();
    let ray = ray_from((0.0, -2.0, 0.0), (0.0, 1.0, 0.0));
    let hit = plane.hit(&ray, 0.001, f64::INFINITY).unwrap();
    assert!(!hit.front_face);
    assert!(hit.normal.dot(ray.direction) < 0.0);
}

#[test]
fn plane_normalizes_non_unit_input() {
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        test_material(),
    );
    assert_length_close(plane.normal, 1.0);
    let ray = ray_from((0.0, 1.0, 0.0), (0.0, -1.0, 0.0));
    let hit = plane.hit(&ray, 0.001, f64::INFINITY).unwrap();
    assert_close(hit.t, 1.0);
}

#[test]
fn plane_hit_respects_t_min_boundary() {
    let plane = floor_plane();
    let ray = ray_from((0.0, 2.0, 0.0), (0.0, -1.0, 0.0));
    assert!(plane.hit(&ray, 2.5, f64::INFINITY).is_none());
    assert!(plane.hit(&ray, 1.5, f64::INFINITY).is_some());
}

#[test]
fn aabb_accepts_ray_with_zero_x_direction_component() {
    let bbox = Aabb::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
    let ray = ray_from((0.0, 0.0, -5.0), (0.0, 0.0, 1.0));
    assert!(bbox.hit(&ray, 0.001, f64::INFINITY));
}

#[test]
fn aabb_rejects_ray_parallel_to_thin_face_outside_bounds() {
    let bbox = Aabb::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
    let ray = ray_from((0.0, 0.0, -5.0), (1.0, 0.0, 0.0));
    assert!(!bbox.hit(&ray, 0.001, f64::INFINITY));
}

#[test]
fn sphere_bounding_box_contains_hit_point() {
    let sphere = unit_sphere_at((2.0, -1.0, 0.5), 0.75);
    let ray = ray_from((-5.0, -1.0, 0.5), (1.0, 0.0, 0.0));
    let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
    let bbox = sphere.bounding_box();
    assert!(hit.point.x >= bbox.min.x && hit.point.x <= bbox.max.x);
    assert!(hit.point.y >= bbox.min.y && hit.point.y <= bbox.max.y);
    assert!(hit.point.z >= bbox.min.z && hit.point.z <= bbox.max.z);
}

#[test]
fn sphere_normal_points_away_from_center_on_oblique_hit() {
    let sphere = unit_sphere_at((0.0, 0.0, 0.0), 1.0);
    let ray = ray_from((-5.0, 0.5, 0.0), (1.0, 0.0, 0.0));
    let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
    assert_length_close(hit.normal, 1.0);
    assert!(hit.normal.dot(hit.point - Point3::new(0.0, 0.0, 0.0)) > 0.0);
}

#[test]
fn grazing_sphere_hit_has_unit_normal() {
    let sphere = unit_sphere_at((0.0, 0.0, 0.0), 1.0);
    let ray = ray_from((-1.0, 1.0, 0.0), (1.0, 0.0, 0.0));
    let hit = sphere.hit(&ray, 0.001, f64::INFINITY).unwrap();
    assert_length_close(hit.normal, 1.0);
    assert_vec3_close(hit.point, Point3::new(0.0, 1.0, 0.0));
}

#[test]
fn bvh_hit_matches_brute_force_for_mixed_scene() {
    let objects = scene(vec![
        Arc::new(floor_plane()),
        Arc::new(unit_sphere_at((0.0, 1.0, 0.0), 1.0)),
        Arc::new(unit_sphere_at((-2.5, 0.5, 0.0), 0.5)),
        Arc::new(unit_sphere_at((2.5, 0.5, 0.0), 0.5)),
    ]);
    let bvh = BvhNode::build(objects.clone());

    let rays = [
        ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0)),
        ray_from((0.0, 2.0, 0.0), (1.0, 0.0, 0.0)),
        ray_from((-5.0, 0.5, 0.0), (1.0, 0.0, 0.0)),
    ];

    for ray in rays {
        let expected = closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY);
        let actual = bvh.hit(&ray, 0.001, f64::INFINITY);
        match (&expected, &actual) {
            (None, None) => {}
            (Some(e), Some(a)) => assert_close(a.t, e.t),
            _ => panic!("BVH hit {:?} != brute force {:?}", actual, expected),
        }
    }
}

#[test]
fn bvh_any_hit_matches_brute_force_for_mixed_scene() {
    let objects = scene(vec![
        Arc::new(floor_plane()),
        Arc::new(unit_sphere_at((0.0, 1.0, 0.0), 1.0)),
        Arc::new(unit_sphere_at((6.0, 0.0, 0.0), 1.0)),
    ]);
    let bvh = BvhNode::build(objects.clone());

    let rays = [
        ray_from((0.0, 5.0, 0.0), (0.0, -1.0, 0.0)),
        ray_from((0.0, 2.0, 0.0), (1.0, 0.0, 0.0)),
        ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0)),
    ];

    for ray in rays {
        let expected = any_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY);
        assert_eq!(bvh.any_hit(&ray, 0.001, f64::INFINITY), expected);
    }
}

#[test]
fn mixed_scene_ray_through_gap_misses_everything() {
    let objects = scene(vec![
        Arc::new(unit_sphere_at((-3.0, 0.0, 0.0), 1.0)),
        Arc::new(unit_sphere_at((3.0, 0.0, 0.0), 1.0)),
        Arc::new(floor_plane()),
    ]);
    let ray = ray_from((0.0, 2.0, 0.0), (1.0, 0.0, 0.0));
    assert!(closest_hit_in_objects(&objects, &ray, 0.001, f64::INFINITY).is_none());
}

#[test]
fn mixed_scene_honors_t_max_when_searching_multiple_spheres() {
    let objects = scene(vec![
        Arc::new(unit_sphere_at((0.0, 0.0, 0.0), 1.0)),
        Arc::new(unit_sphere_at((6.0, 0.0, 0.0), 1.0)),
    ]);
    let ray = ray_from((-5.0, 0.0, 0.0), (1.0, 0.0, 0.0));
    assert!(closest_hit_in_objects(&objects, &ray, 0.001, 3.5).is_none());
    let hit = closest_hit_in_objects(&objects, &ray, 0.001, 10.0).unwrap();
    assert_close(hit.t, 4.0);
}

#[test]
fn diagonal_plane_bounding_box_encloses_surface_point() {
    let plane = diagonal_plane();
    let bbox = plane.bounding_box();
    let origin = Point3::new(0.0, 0.0, 0.0);
    assert!(origin.x >= bbox.min.x && origin.x <= bbox.max.x);
    assert!(origin.y >= bbox.min.y && origin.y <= bbox.max.y);
    assert!(origin.z >= bbox.min.z && origin.z <= bbox.max.z);
    let extent = bbox.max - bbox.min;
    assert!(extent.x > 100.0);
    assert!(extent.y > 100.0);
    assert!(extent.z > 100.0);
}
