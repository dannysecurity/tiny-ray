use image::ImageBuffer;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::camera::Camera;
use crate::color::ColorPipeline;
use crate::film::{accumulate_weighted, PixelFilter};
use crate::sampling::{pixel_offsets, AntiAliasing};
use crate::scene::Scene;
use crate::shade::PathTracer;
use crate::vec3::{Color, Point3, Vec3};

/// Per-frame rendering context shared across all pixels.
struct RenderContext<'a> {
    camera: Camera,
    pipeline: ColorPipeline,
    tracer: PathTracer<'a>,
    width: u32,
    height: u32,
    samples_per_pixel: u32,
    max_depth: u32,
    aa: AntiAliasing,
    filter: PixelFilter,
}

impl<'a> RenderContext<'a> {
    fn from_scene(scene: &'a Scene) -> Self {
        let aspect = scene.render.width as f64 / scene.render.height as f64;
        Self {
            width: scene.render.width,
            height: scene.render.height,
            samples_per_pixel: scene.render.samples_per_pixel,
            max_depth: scene.render.max_depth,
            aa: scene.render.aa,
            filter: scene.render.filter,
            camera: Camera::new(
                Point3::from_array(scene.camera.lookfrom),
                Point3::from_array(scene.camera.lookat),
                Vec3::from_array(scene.camera.vup),
                scene.camera.vfov,
                aspect,
                scene.camera.aperture,
                scene.camera.focus_distance,
            ),
            pipeline: ColorPipeline {
                gamma: scene.render.gamma,
                exposure: scene.render.exposure,
            },
            tracer: PathTracer::new(
                scene.world.as_ref(),
                &scene.lights,
                &scene.sky,
            ),
        }
    }

    fn log_banner(&self) {
        eprintln!(
            "Rendering {}x{} ({} spp, depth {}, gamma {:?}, aa {:?}, filter {:?})",
            self.width,
            self.height,
            self.samples_per_pixel,
            self.max_depth,
            self.pipeline.gamma,
            self.aa,
            self.filter,
        );
    }

    fn trace_pixel<R: Rng + ?Sized>(&self, rng: &mut R, x: u32, y: u32) -> Color {
        let samples = (0..self.samples_per_pixel).map(|sample| {
            let (du, dv) = pixel_offsets(sample, self.samples_per_pixel, self.aa, rng);
            let u = (x as f64 + du) / (self.width - 1) as f64;
            let v = ((self.height - 1 - y) as f64 + dv) / (self.height - 1) as f64;
            let time = rng.gen();
            let ray = self.camera.get_ray(rng, u, v, time);
            let radiance = self.tracer.trace_ray(rng, &ray, self.max_depth);
            let weight = self.filter.weight(du - 0.5, dv - 0.5);
            (radiance, weight)
        });
        accumulate_weighted(samples)
    }
}

pub fn render(scene: &Scene) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = RenderContext::from_scene(scene);
    ctx.log_banner();

    let mut rng = StdRng::seed_from_u64(42);
    let mut buffer = ImageBuffer::new(ctx.width, ctx.height);

    for y in 0..ctx.height {
        if y % 32 == 0 {
            eprintln!("Scanline {}/{}", y, ctx.height);
        }
        for x in 0..ctx.width {
            let pixel = ctx.trace_pixel(&mut rng, x, y);
            buffer.put_pixel(x, y, ctx.pipeline.encode_pixel(pixel));
        }
    }

    buffer.save(&scene.render.output)?;
    eprintln!("Wrote {}", scene.render.output);
    Ok(())
}
