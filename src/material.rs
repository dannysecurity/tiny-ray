use rand::Rng;

use crate::hittable::{HitRecord, Hittable};
use crate::lights::LightList;
use crate::ray::Ray;
use crate::vec3::{Color, Vec3};

#[derive(Clone, Debug)]
pub enum Material {
    Lambertian { albedo: Color },
    Metal { albedo: Color, fuzz: f64 },
    Dielectric { index: f64 },
    Emissive { color: Color, intensity: f64 },
}

impl Material {
    pub fn scatter<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        ray_in: &Ray,
        hit: &HitRecord,
    ) -> Option<(Color, Ray)> {
        match self {
            Material::Lambertian { albedo } => {
                let mut scatter_direction = hit.normal + Vec3::random_unit_vector(rng);
                if scatter_direction.near_zero() {
                    scatter_direction = hit.normal;
                }
                let scattered = Ray::new(hit.point, scatter_direction, ray_in.time);
                Some((*albedo, scattered))
            }
            Material::Metal { albedo, fuzz } => {
                let reflected = ray_in.direction.normalize().reflect(hit.normal);
                let scattered = Ray::new(
                    hit.point,
                    reflected + *fuzz * Vec3::random_in_unit_sphere(rng),
                    ray_in.time,
                );
                if scattered.direction.dot(hit.normal) > 0.0 {
                    Some((*albedo, scattered))
                } else {
                    None
                }
            }
            Material::Dielectric { index } => {
                let refraction_ratio = if hit.front_face {
                    1.0 / index
                } else {
                    *index
                };
                let unit_direction = ray_in.direction.normalize();
                let cos_theta = (-unit_direction).dot(hit.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

                let cannot_refract = refraction_ratio * sin_theta > 1.0;
                let direction = if cannot_refract
                    || Self::reflectance(cos_theta, refraction_ratio)
                        > rng.gen_range(0.0..1.0)
                {
                    unit_direction.reflect(hit.normal)
                } else {
                    unit_direction.refract(hit.normal, refraction_ratio)?
                };

                Some((Color::new(1.0, 1.0, 1.0), Ray::new(hit.point, direction, ray_in.time)))
            }
            Material::Emissive { .. } => None,
        }
    }

    pub fn emitted(&self) -> Color {
        match self {
            Material::Emissive { color, intensity } => *color * *intensity,
            _ => Color::default(),
        }
    }

    pub fn is_emissive(&self) -> bool {
        matches!(self, Material::Emissive { .. })
    }

    /// Next-event estimation for materials that support direct area-light sampling.
    pub fn sample_direct<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
        lights: &LightList,
        world: &dyn Hittable,
        hit: &HitRecord,
        incoming_direction: Vec3,
        ray_time: f64,
    ) -> Color {
        match self {
            Material::Lambertian { albedo } => lights.sample_direct(
                rng,
                world,
                hit.point,
                hit.normal,
                *albedo,
                ray_time,
            ),
            Material::Metal { albedo, fuzz } => lights.sample_direct_specular(
                rng,
                world,
                hit.point,
                hit.normal,
                incoming_direction,
                *albedo,
                *fuzz,
                ray_time,
            ),
            _ => Color::default(),
        }
    }

    fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
        let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::geometry_tests::ray_from;
    use crate::hittable::HitRecord;
    use crate::sphere::Sphere;
    use crate::vec3::Point3;

    fn hit_on_y_up_normal() -> HitRecord {
        HitRecord {
            point: Point3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            t: 1.0,
            front_face: true,
            material: Arc::new(Material::Metal {
                albedo: Color::new(0.8, 0.8, 0.8),
                fuzz: 0.0,
            }),
        }
    }

    #[test]
    fn lambertian_sample_direct_delegates_to_light_list() {
        let material = Material::Lambertian {
            albedo: Color::new(0.5, 0.5, 0.5),
        };
        let hit = HitRecord {
            point: Point3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            t: 1.0,
            front_face: true,
            material: Arc::new(material.clone()),
        };
        let lights = LightList::new();
        let world = Sphere::new(
            Point3::new(100.0, 100.0, 100.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::default(),
            }),
        );
        let mut rng = StdRng::seed_from_u64(1);

        let color = material.sample_direct(
            &mut rng,
            &lights,
            &world,
            &hit,
            Vec3::new(0.0, -1.0, 0.0),
            0.0,
        );
        assert_eq!(color, Color::default());
    }

    #[test]
    fn metal_sample_direct_uses_incoming_direction() {
        let material = Material::Metal {
            albedo: Color::new(0.9, 0.9, 0.9),
            fuzz: 0.0,
        };
        let hit = hit_on_y_up_normal();
        let light_sphere = Sphere::new(
            Point3::new(0.0, 10.0, 0.0),
            1.0,
            Arc::new(Material::Emissive {
                color: Color::new(1.0, 1.0, 1.0),
                intensity: 5.0,
            }),
        );
        let lights = LightList::from_spheres(&[light_sphere]);
        let world = Sphere::new(
            Point3::new(100.0, 100.0, 100.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::default(),
            }),
        );
        let mut rng = StdRng::seed_from_u64(2);
        let ray = ray_from((0.0, 1.0, 0.0), (0.0, -1.0, 0.0));

        let color = material.sample_direct(
            &mut rng,
            &lights,
            &world,
            &hit,
            ray.direction,
            0.0,
        );
        assert!(color.x > 0.0 || color.y > 0.0 || color.z > 0.0);
    }

    #[test]
    fn dielectric_sample_direct_returns_black() {
        let material = Material::Dielectric { index: 1.5 };
        let hit = hit_on_y_up_normal();
        let lights = LightList::new();
        let world = Sphere::new(
            Point3::new(100.0, 100.0, 100.0),
            1.0,
            Arc::new(Material::Lambertian {
                albedo: Color::default(),
            }),
        );
        let mut rng = StdRng::seed_from_u64(3);

        let color = material.sample_direct(
            &mut rng,
            &lights,
            &world,
            &hit,
            Vec3::new(0.0, -1.0, 0.0),
            0.0,
        );
        assert_eq!(color, Color::default());
    }
}
