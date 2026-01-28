#[cfg(test)]
mod geometry_tests;

#[cfg(test)]
mod intersection_suite;

#[cfg(test)]
mod vec3_suite;

#[cfg(test)]
mod renderer_tests;

mod bvh;
mod camera;
mod cli;
mod color;
mod dither;
mod film;
mod hittable;
mod intersection;
mod lights;
mod material;
mod plane;
mod ray;
mod renderer;
mod sampling;
mod scene;
mod shade;
mod sky;
mod sphere;
mod vec3;
mod world;

pub use scene::Scene;

use cli::{CliOptions, USAGE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = CliOptions::from_env().map_err(|message| {
        eprintln!("{message}\n");
        eprint!("{USAGE}");
        message
    })?;

    let (scene, file, source) = options.load_scene()?;

    if options.validate {
        options.print_validation_summary(&scene, &file, &source);
        return Ok(());
    }

    if options.bvh_stats {
        match scene.world.bvh_stats() {
            Some(stats) => eprintln!("{}", stats.format_summary()),
            None => eprintln!("BVH: not built (single bounded primitive or planes only)"),
        }
    }

    renderer::render(&scene)
}
