use rand::Rng;

use crate::hittable::HitRecord;
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

    fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
        let r0 = ((1.0 - ref_idx) / (1.0 + ref_idx)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}
