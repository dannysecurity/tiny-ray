#[cfg(test)]
mod geometry_tests;

#[cfg(test)]
mod intersection_suite;

#[cfg(test)]
mod vec3_suite;

mod bvh;
mod camera;
mod cli;
mod color;
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
use scene::load_scene_file_with_format;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = CliOptions::from_env().map_err(|message| {
        eprintln!("{message}\n");
        eprint!("{USAGE}");
        message
    })?;

    let scene = if options.scene_path.exists() {
        let mut file = load_scene_file_with_format(&options.scene_path, options.format)?;
        options.apply_to_render(&mut file.render);
        Scene::from_scene_file(file)
    } else {
        eprintln!("Scene file not found; using built-in demo scene");
        let mut file = Scene::default_demo_file();
        options.apply_to_render(&mut file.render);
        Scene::from_scene_file(file)
    };

    if options.bvh_stats {
        match scene.world.bvh_stats() {
            Some(stats) => eprintln!("{}", stats.format_summary()),
            None => eprintln!("BVH: not built (single bounded primitive or planes only)"),
        }
    }

    renderer::render(&scene)
}
