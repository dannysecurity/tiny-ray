mod bvh;
mod camera;
mod hittable;
mod lights;
mod material;
mod ray;
mod renderer;
mod scene;
mod sphere;
mod vec3;

pub use scene::Scene;

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scene_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("scenes/demo.ron"));

    let scene = if scene_path.exists() {
        Scene::from_file(&scene_path)?
    } else {
        eprintln!("Scene file not found; using built-in demo scene");
        Scene::default_demo()
    };

    renderer::render(&scene)
}
