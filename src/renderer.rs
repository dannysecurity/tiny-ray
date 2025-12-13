use image::ImageBuffer;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::camera::Camera;
use crate::color::ColorPipeline;
use crate::hittable::Hittable;
use crate::lights::LightList;
use crate::material::Material;
use crate::ray::Ray;
use crate::sampling::pixel_offsets;
use crate::scene::Scene;
use crate::sky::SkyGradient;
use crate::vec3::{Color, Vec3};

pub fn render(scene: &Scene) -> Result<(), Box<dyn std::error::Error>> {
    let aspect = scene.render.width as f64 / scene.render.height as f64;
    let camera = Camera::new(
        point(&scene.camera.lookfrom),
        point(&scene.camera.lookat),
        vec(&scene.camera.vup),
        scene.camera.vfov,
        aspect,
        scene.camera.aperture,
        scene.camera.focus_distance,
    );

    let mut rng = StdRng::seed_from_u64(42);
    let mut buffer =
        ImageBuffer::new(scene.render.width, scene.render.height);

    let pipeline = ColorPipeline {
        gamma: scene.render.gamma,
        exposure: scene.render.exposure,
    };

    eprintln!(
        "Rendering {}x{} ({} spp, depth {}, gamma {:?}, aa {:?})",
        scene.render.width,
        scene.render.height,
        scene.render.samples_per_pixel,
        scene.render.max_depth,
        scene.render.gamma,
        scene.render.aa,
    );

    for y in 0..scene.render.height {
        if y % 32 == 0 {
            eprintln!("Scanline {}/{}", y, scene.render.height);
        }
        for x in 0..scene.render.width {
            let mut pixel = Color::default();
            for sample in 0..scene.render.samples_per_pixel {
                let (du, dv) = pixel_offsets(
                    sample,
                    scene.render.samples_per_pixel,
                    scene.render.aa,
                    &mut rng,
                );
                let u = (x as f64 + du) / (scene.render.width - 1) as f64;
                let v = ((scene.render.height - 1 - y) as f64 + dv)
                    / (scene.render.height - 1) as f64;
                let time = rng.gen();
                let ray = camera.get_ray(&mut rng, u, v, time);
                pixel += ray_color(
                    &mut rng,
                    &ray,
                    scene.world.as_ref(),
                    &scene.lights,
                    &scene.sky,
                    scene.render.max_depth,
                );
            }
            pixel /= scene.render.samples_per_pixel as f64;
            buffer.put_pixel(x, y, pipeline.encode_pixel(pixel));
        }
    }

    buffer.save(&scene.render.output)?;
    eprintln!("Wrote {}", scene.render.output);
    Ok(())
}

fn ray_color<R: Rng + ?Sized>(
    rng: &mut R,
    ray: &Ray,
    world: &dyn Hittable,
    lights: &LightList,
    sky: &SkyGradient,
    depth: u32,
) -> Color {
    if depth == 0 {
        return Color::default();
    }

    if let Some(hit) = world.hit(ray, 1e-3, f64::INFINITY) {
        let mut color = Color::default();

        if let Material::Lambertian { albedo } = hit.material.as_ref() {
            color += lights.sample_direct(
                rng,
                world,
                hit.point,
                hit.normal,
                *albedo,
                ray.time,
            );
        }

        let emitted = if hit.material.is_emissive() {
            hit.material.emitted()
        } else {
            Color::default()
        };

        if let Some((attenuation, scattered)) = hit.material.scatter(rng, ray, &hit) {
            color += attenuation
                * ray_color(rng, &scattered, world, lights, sky, depth - 1);
        } else {
            color += emitted;
        }

        color
    } else {
        sky.sample(ray.direction)
    }
}

fn point(v: &[f64; 3]) -> crate::vec3::Point3 {
    crate::vec3::Point3::new(v[0], v[1], v[2])
}

fn vec(v: &[f64; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}
