use std::f64::consts::PI;

use rand::Rng;

use crate::hittable::Hittable;
use crate::material::Material;
use crate::ray::Ray;
use crate::sphere::Sphere;
use crate::vec3::{Color, Point3, Vec3};

/// An emissive sphere treated as an area light for direct sampling.
#[derive(Clone, Debug)]
pub struct EmissiveSphere {
    pub center: Point3,
    pub radius: f64,
    pub radiance: Color,
}

impl EmissiveSphere {
    pub fn from_sphere(sphere: &Sphere) -> Option<Self> {
        match sphere.material.as_ref() {
            Material::Emissive { color, intensity } => Some(Self {
                center: sphere.center,
                radius: sphere.radius,
                radiance: *color * *intensity,
            }),
            _ => None,
        }
    }

    pub fn surface_area(&self) -> f64 {
        4.0 * PI * self.radius * self.radius
    }

    /// Uniformly sample a point on the sphere surface.
    pub fn sample_surface<R: Rng + ?Sized>(&self, rng: &mut R) -> (Point3, Vec3) {
        let outward = Vec3::random_unit_vector(rng);
        let point = self.center + outward * self.radius;
        (point, outward)
    }

    /// Sample toward this light from `shading_point` and return radiance and solid-angle PDF.
    pub fn sample_toward<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        shading_point: Point3,
    ) -> Option<LightSample> {
        let (light_point, light_normal) = self.sample_surface(rng);
        let to_light = light_point - shading_point;
        let distance = to_light.length();
        if distance <= 1e-8 {
            return None;
        }
        let direction = to_light / distance;

        let cos_at_light = (-direction).dot(light_normal);
        if cos_at_light <= 0.0 {
            return None;
        }

        let pdf = distance * distance / (cos_at_light * self.surface_area());
        Some(LightSample {
            point: light_point,
            direction,
            distance,
            radiance: self.radiance,
            pdf,
        })
    }
}

#[derive(Clone, Debug)]
pub struct LightSample {
    pub point: Point3,
    pub direction: Vec3,
    pub distance: f64,
    pub radiance: Color,
    pub pdf: f64,
}

/// Collection of emissive spheres sampled for next-event estimation.
#[derive(Clone, Debug, Default)]
pub struct LightList {
    lights: Vec<EmissiveSphere>,
}

impl LightList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, light: EmissiveSphere) {
        self.lights.push(light);
    }

    pub fn from_spheres(spheres: &[Sphere]) -> Self {
        let mut list = Self::new();
        for sphere in spheres {
            if let Some(light) = EmissiveSphere::from_sphere(sphere) {
                list.push(light);
            }
        }
        list
    }

    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }

    pub fn len(&self) -> usize {
        self.lights.len()
    }

    /// Trace a shadow ray; returns true when the light is visible from `origin`.
    pub fn is_visible(
        world: &dyn Hittable,
        origin: Point3,
        sample: &LightSample,
        ray_time: f64,
    ) -> bool {
        let shadow_ray = Ray::new(
            origin + sample.direction * 1e-4,
            sample.direction,
            ray_time,
        );
        !world
            .any_hit(&shadow_ray, 1e-4, sample.distance - 1e-4)
    }

    /// Lambertian direct lighting via uniform light and surface-point sampling.
    pub fn sample_direct<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        world: &dyn Hittable,
        shading_point: Point3,
        shading_normal: Vec3,
        albedo: Color,
        ray_time: f64,
    ) -> Color {
        if self.lights.is_empty() {
            return Color::default();
        }

        let index = rng.gen_range(0..self.lights.len());
        let light = &self.lights[index];
        let Some(sample) = light.sample_toward(rng, shading_point) else {
            return Color::default();
        };

        let cos_theta = shading_normal.dot(sample.direction).max(0.0);
        if cos_theta <= 0.0 {
            return Color::default();
        }

        if !Self::is_visible(world, shading_point, &sample, ray_time) {
            return Color::default();
        }

        let light_pdf = sample.pdf * self.lights.len() as f64;
        sample.radiance * (albedo / PI) * cos_theta / light_pdf
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::material::Material;

    #[test]
    fn emissive_sphere_from_material() {
        let sphere = Sphere::new(
            Point3::new(0.0, 5.0, 0.0),
            2.0,
            Arc::new(Material::Emissive {
                color: Color::new(1.0, 0.5, 0.25),
                intensity: 3.0,
            }),
        );
        let light = EmissiveSphere::from_sphere(&sphere).expect("emissive sphere");
        assert_eq!(light.center, sphere.center);
        assert_eq!(light.radius, 2.0);
        assert_eq!(light.radiance, Color::new(3.0, 1.5, 0.75));
    }

    #[test]
    fn non_emissive_sphere_is_skipped() {
        let sphere = Sphere::new(
            Point3::default(),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        );
        assert!(EmissiveSphere::from_sphere(&sphere).is_none());
    }

    #[test]
    fn surface_area_matches_sphere_geometry() {
        let light = EmissiveSphere {
            center: Point3::default(),
            radius: 2.0,
            radiance: Color::default(),
        };
        let expected = 4.0 * PI * 4.0;
        assert!((light.surface_area() - expected).abs() < 1e-10);
    }

    #[test]
    fn sample_toward_pdf_is_positive_for_visible_points() {
        let mut rng = StdRng::seed_from_u64(7);
        let light = EmissiveSphere {
            center: Point3::new(0.0, 10.0, 0.0),
            radius: 1.0,
            radiance: Color::new(1.0, 1.0, 1.0),
        };
        let shading_point = Point3::new(0.0, 0.0, 0.0);

        let mut samples = 0;
        for _ in 0..256 {
            if let Some(sample) = light.sample_toward(&mut rng, shading_point) {
                assert!(sample.pdf > 0.0);
                assert!(sample.distance > 0.0);
                assert!(sample.direction.length() > 0.999);
                samples += 1;
            }
        }
        assert!(samples > 0, "expected at least one valid light sample");
    }

    #[test]
    fn light_list_collects_emissive_spheres_only() {
        let spheres = vec![
            Sphere::new(
                Point3::new(0.0, 8.0, 0.0),
                3.0,
                Arc::new(Material::Emissive {
                    color: Color::new(1.0, 1.0, 1.0),
                    intensity: 2.0,
                }),
            ),
            Sphere::new(
                Point3::new(0.0, 1.0, 0.0),
                1.0,
                Arc::new(Material::Lambertian {
                    albedo: Color::new(0.8, 0.2, 0.2),
                }),
            ),
        ];
        let list = LightList::from_spheres(&spheres);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn shadow_ray_detects_occluder() {
        let occluder = Sphere::new(
            Point3::new(0.0, 5.0, 0.0),
            2.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        );
        let shading_point = Point3::new(0.0, 1.0, 0.0);
        let light_point = Point3::new(0.0, 10.0, 0.0);
        let direction = (light_point - shading_point).normalize();
        let sample = LightSample {
            point: light_point,
            direction,
            distance: (light_point - shading_point).length(),
            radiance: Color::new(1.0, 1.0, 1.0),
            pdf: 1.0,
        };

        assert!(!LightList::is_visible(&occluder, shading_point, &sample, 0.0));
    }

    #[test]
    fn shadow_ray_passes_when_unblocked() {
        let occluder = Sphere::new(
            Point3::new(5.0, 5.0, 0.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::new(0.5, 0.5, 0.5),
            }),
        );
        let shading_point = Point3::new(0.0, 1.0, 0.0);
        let light_point = Point3::new(0.0, 10.0, 0.0);
        let direction = (light_point - shading_point).normalize();
        let sample = LightSample {
            point: light_point,
            direction,
            distance: (light_point - shading_point).length(),
            radiance: Color::new(1.0, 1.0, 1.0),
            pdf: 1.0,
        };

        assert!(LightList::is_visible(&occluder, shading_point, &sample, 0.0));
    }
}
