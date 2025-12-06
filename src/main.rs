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

const USAGE: &str = "\
Usage: tiny-ray [OPTIONS] [SCENE]

Path-traced sphere renderer. Loads a scene file (RON, JSON, or YAML) and writes
an image to the path given in the scene, or overridden with --output.

Arguments:
  SCENE    Scene file path (default: scenes/demo.ron)

Options:
  -o, --output PATH     Override the output image path from the scene file
  -s, --samples N       Override samples per pixel (useful for quick previews)
  -h, --help            Show this help message

Examples:
  cargo run --release -- scenes/studio.ron
  cargo run --release -- --samples 10 --output preview.png scenes/studio.json
";

#[derive(Debug, Default, PartialEq, Eq)]
struct CliOptions {
    scene_path: PathBuf,
    output: Option<String>,
    samples: Option<u32>,
}

fn parse_args_from<I, S>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let mut scene_path = None;
    let mut output = None;
    let mut samples = None;

    while let Some(arg) = args.next() {
        let arg = arg.as_ref();
        match arg {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-o" | "--output" => {
                output = Some(next_value(&mut args, arg)?);
            }
            "-s" | "--samples" => {
                let value = next_value(&mut args, arg)?;
                samples = Some(
                    value
                        .parse()
                        .map_err(|_| format!("invalid samples value: {value}"))?,
                );
                if samples == Some(0) {
                    return Err("samples must be at least 1".into());
                }
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => {
                if scene_path.is_some() {
                    return Err(format!("unexpected extra argument: {value}"));
                }
                scene_path = Some(PathBuf::from(value));
            }
        }
    }

    Ok(CliOptions {
        scene_path: scene_path.unwrap_or_else(|| PathBuf::from("scenes/demo.ron")),
        output,
        samples,
    })
}

fn parse_args() -> Result<CliOptions, String> {
    parse_args_from(env::args().skip(1))
}

fn next_value<I, S>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next()
        .map(|value| value.as_ref().to_string())
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn apply_cli_overrides(scene: &mut Scene, options: &CliOptions) {
    if let Some(ref path) = options.output {
        scene.render.output = path.clone();
    }
    if let Some(samples) = options.samples {
        scene.render.samples_per_pixel = samples;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_args().map_err(|message| {
        eprintln!("{message}\n");
        eprint!("{USAGE}");
        message
    })?;

    let mut scene = if options.scene_path.exists() {
        Scene::from_file(&options.scene_path)?
    } else {
        eprintln!("Scene file not found; using built-in demo scene");
        Scene::default_demo()
    };

    apply_cli_overrides(&mut scene, &options);
    renderer::render(&scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_demo_scene() {
        let options = parse_args_from::<_, &str>([]).unwrap();
        assert_eq!(options.scene_path, PathBuf::from("scenes/demo.ron"));
        assert_eq!(options.output, None);
        assert_eq!(options.samples, None);
    }

    #[test]
    fn parse_args_accepts_scene_and_overrides() {
        let options = parse_args_from([
            "--output",
            "out.png",
            "--samples",
            "16",
            "scenes/studio.ron",
        ])
        .unwrap();
        assert_eq!(options.scene_path, PathBuf::from("scenes/studio.ron"));
        assert_eq!(options.output.as_deref(), Some("out.png"));
        assert_eq!(options.samples, Some(16));
    }

    #[test]
    fn parse_args_rejects_zero_samples() {
        assert!(parse_args_from(["--samples", "0"]).is_err());
    }

    #[test]
    fn apply_cli_overrides_updates_render_settings() {
        let mut scene = Scene::default_demo();
        let options = CliOptions {
            scene_path: PathBuf::from("scenes/demo.ron"),
            output: Some("override.png".into()),
            samples: Some(4),
        };
        apply_cli_overrides(&mut scene, &options);
        assert_eq!(scene.render.output, "override.png");
        assert_eq!(scene.render.samples_per_pixel, 4);
    }
}
