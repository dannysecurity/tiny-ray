//! Path-tracing integration: one-bounce shading and recursive ray tracing.

use rand::Rng;

use crate::hittable::{HitRecord, Hittable};
use crate::lights::LightList;
use crate::ray::Ray;
use crate::sky::SkyGradient;
use crate::vec3::Color;

/// Scene references needed to shade rays without coupling to the image loop.
pub struct PathTracer<'a> {
    pub world: &'a dyn Hittable,
    pub lights: &'a LightList,
    pub sky: &'a SkyGradient,
}

impl<'a> PathTracer<'a> {
    pub fn new(world: &'a dyn Hittable, lights: &'a LightList, sky: &'a SkyGradient) -> Self {
        Self {
            world,
            lights,
            sky,
        }
    }

    /// Integrate radiance along `ray` up to `depth` bounces.
    pub fn trace_ray<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        ray: &Ray,
        depth: u32,
    ) -> Color {
        if depth == 0 {
            return Color::default();
        }

        if let Some(hit) = self.world.hit(ray, 1e-3, f64::INFINITY) {
            self.shade_hit(rng, ray, &hit, depth)
        } else {
            self.sky.sample(ray.direction)
        }
    }

    /// Shade a single surface interaction: direct light, emission, and scatter.
    fn shade_hit<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        ray: &Ray,
        hit: &HitRecord,
        depth: u32,
    ) -> Color {
        let mut color = hit.material.sample_direct(
            rng,
            self.lights,
            self.world,
            hit,
            ray.time,
        );

        let emitted = if hit.material.is_emissive() {
            hit.material.emitted()
        } else {
            Color::default()
        };

        if let Some((attenuation, scattered)) = hit.material.scatter(rng, ray, hit) {
            color += attenuation * self.trace_ray(rng, &scattered, depth - 1);
        } else {
            color += emitted;
        }

        color
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::geometry_tests::{assert_vec3_close, ray_from};
    use crate::material::Material;
    use crate::sphere::Sphere;
    use crate::vec3::{Color, Point3};

    fn tracer_for<'a>(
        world: &'a dyn Hittable,
        lights: &'a LightList,
        sky: &'a SkyGradient,
    ) -> PathTracer<'a> {
        PathTracer::new(world, lights, sky)
    }

    #[test]
    fn zero_depth_returns_black() {
        let sphere = Sphere::new(
            Point3::new(0.0, 0.0, -5.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.8, 0.2, 0.2),
            }),
        );
        let lights = LightList::new();
        let sky = SkyGradient::default();
        let mut rng = StdRng::seed_from_u64(1);
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 0.0, -1.0));

        let color = tracer_for(&sphere, &lights, &sky).trace_ray(&mut rng, &ray, 0);
        assert_eq!(color, Color::default());
    }

    #[test]
    fn miss_ray_samples_sky() {
        let sphere = Sphere::new(
            Point3::new(0.0, 0.0, -5.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        );
        let lights = LightList::new();
        let sky = SkyGradient {
            horizon: Color::new(1.0, 0.0, 0.0),
            zenith: Color::new(0.0, 0.0, 1.0),
        };
        let mut rng = StdRng::seed_from_u64(2);
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 1.0, 0.0));

        let color = tracer_for(&sphere, &lights, &sky).trace_ray(&mut rng, &ray, 1);
        assert_eq!(color, sky.sample(ray.direction));
    }

    #[test]
    fn emissive_hit_returns_emitted_radiance() {
        let sphere = Sphere::new(
            Point3::new(0.0, 0.0, -2.0),
            1.0,
            Arc::new(Material::Emissive {
                color: Color::new(1.0, 0.5, 0.25),
                intensity: 2.0,
            }),
        );
        let lights = LightList::new();
        let sky = SkyGradient::default();
        let mut rng = StdRng::seed_from_u64(3);
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 0.0, -1.0));

        let color = tracer_for(&sphere, &lights, &sky).trace_ray(&mut rng, &ray, 1);
        assert_vec3_close(color, Color::new(2.0, 1.0, 0.5));
    }

    #[test]
    fn lambertian_hit_with_visible_light_adds_direct_contribution() {
        let floor = Sphere::new(
            Point3::new(0.0, -100.5, -1.0),
            100.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.8, 0.8, 0.8),
            }),
        );
        let light_sphere = Sphere::new(
            Point3::new(0.0, 10.0, -1.0),
            2.0,
            Arc::new(Material::Emissive {
                color: Color::new(1.0, 1.0, 1.0),
                intensity: 10.0,
            }),
        );
        let lights = LightList::from_spheres(&[light_sphere]);
        let world = crate::bvh::BvhNode::build(vec![Arc::new(floor) as Arc<dyn Hittable>]);
        let sky = SkyGradient::default();
        let mut rng = StdRng::seed_from_u64(4);
        let ray = ray_from((0.0, 1.0, 0.0), (0.0, -1.0, 0.0));

        let color = tracer_for(&world, &lights, &sky).trace_ray(&mut rng, &ray, 1);
        assert!(color.x > 0.0 || color.y > 0.0 || color.z > 0.0);
    }

    #[test]
    fn metal_hit_without_scatter_returns_black_at_depth_one() {
        let sphere = Sphere::new(
            Point3::new(0.0, 0.0, -2.0),
            1.0,
            Arc::new(Material::Metal {
                albedo: Color::new(0.9, 0.9, 0.9),
                fuzz: 0.0,
            }),
        );
        let lights = LightList::new();
        let sky = SkyGradient::default();
        let mut rng = StdRng::seed_from_u64(5);
        let ray = ray_from((0.0, 0.0, 0.0), (0.0, 0.0, -1.0));

        let color = tracer_for(&sphere, &lights, &sky).trace_ray(&mut rng, &ray, 1);
        assert_eq!(color, Color::default());
    }
}
