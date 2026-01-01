#[cfg(test)]
mod geometry_tests;

#[cfg(test)]
mod intersection_suite;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = CliOptions::from_env().map_err(|message| {
        eprintln!("{message}\n");
        eprint!("{USAGE}");
        message
    })?;

    let mut scene = if options.scene_path.exists() {
        Scene::from_file_with_format(&options.scene_path, options.format)?
    } else {
        eprintln!("Scene file not found; using built-in demo scene");
        Scene::default_demo()
    };

    options.apply_to_scene(&mut scene);
    renderer::render(&scene)
}
