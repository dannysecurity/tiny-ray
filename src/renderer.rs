use image::{ImageBuffer, Rgb};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::camera::Camera;
use crate::hittable::Hittable;
use crate::ray::Ray;
use crate::scene::Scene;
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

    eprintln!(
        "Rendering {}x{} ({} spp, depth {})",
        scene.render.width,
        scene.render.height,
        scene.render.samples_per_pixel,
        scene.render.max_depth
    );

    for y in 0..scene.render.height {
        if y % 32 == 0 {
            eprintln!("Scanline {}/{}", y, scene.render.height);
        }
        for x in 0..scene.render.width {
            let mut pixel = Color::default();
            for _ in 0..scene.render.samples_per_pixel {
                let u = (x as f64 + rng.gen::<f64>()) / (scene.render.width - 1) as f64;
                let v = ((scene.render.height - 1 - y) as f64 + rng.gen::<f64>())
                    / (scene.render.height - 1) as f64;
                let time = rng.gen();
                let ray = camera.get_ray(&mut rng, u, v, time);
                pixel += ray_color(&mut rng, &ray, scene.world.as_ref(), scene.render.max_depth);
            }
            pixel /= scene.render.samples_per_pixel as f64;
            buffer.put_pixel(x, y, to_rgb(pixel));
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
    depth: u32,
) -> Color {
    if depth == 0 {
        return Color::default();
    }

    if let Some(hit) = world.hit(ray, 1e-3, f64::INFINITY) {
        let emitted = hit.material.emitted();
        if let Some((attenuation, scattered)) = hit.material.scatter(rng, ray, &hit) {
            emitted + attenuation * ray_color(rng, &scattered, world, depth - 1)
        } else {
            emitted
        }
    } else {
        let unit = ray.direction.normalize();
        let t = 0.5 * (unit.y + 1.0);
        (1.0 - t) * Color::new(1.0, 1.0, 1.0) + t * Color::new(0.5, 0.7, 1.0)
    }
}

fn to_rgb(color: Color) -> Rgb<u8> {
    Rgb([
        linear_to_gamma(color.x),
        linear_to_gamma(color.y),
        linear_to_gamma(color.z),
    ])
}

fn linear_to_gamma(v: f64) -> u8 {
    let clamped = v.clamp(0.0, 0.999);
    (256.0 * clamped.sqrt()).floor() as u8
}

fn point(v: &[f64; 3]) -> crate::vec3::Point3 {
    crate::vec3::Point3::new(v[0], v[1], v[2])
}

fn vec(v: &[f64; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}
