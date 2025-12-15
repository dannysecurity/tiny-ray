//! Command-line argument parsing and scene override application.

use std::env;
use std::path::PathBuf;

use crate::color::GammaEncoding;
use crate::sampling::AntiAliasing;
use crate::scene::{Scene, SceneFormat};

pub const USAGE: &str = "\
Usage: tiny-ray [OPTIONS] [SCENE]

Path-traced sphere and plane renderer. Loads a scene file (RON, JSON, or YAML) and writes
an image to the path given in the scene, or overridden with --output.

Arguments:
  SCENE    Scene file path (default: scenes/demo.ron)

Options:
  -o, --output PATH     Override the output image path from the scene file
  -s, --samples N       Override samples per pixel (useful for quick previews)
      --format FMT      Force scene parser: ron, json, or yaml (default: from extension)
      --gamma MODE      Override output gamma: gamma2, srgb, or linear
      --exposure F      Override linear exposure multiplier (default 1.0)
      --aa MODE         Override anti-aliasing: random, stratified, or halton
  -h, --help            Show this help message

Examples:
  cargo run --release -- scenes/studio.ron
  cargo run --release -- --samples 10 --output preview.png scenes/studio.json
  cargo run --release -- --format yaml scenes/cornell-modular.yaml
  cargo run --release -- --gamma srgb --aa stratified scenes/studio.ron
";

#[derive(Debug, Default, PartialEq)]
pub struct CliOptions {
    pub scene_path: PathBuf,
    pub output: Option<String>,
    pub samples: Option<u32>,
    pub format: Option<SceneFormat>,
    pub gamma: Option<GammaEncoding>,
    pub exposure: Option<f64>,
    pub aa: Option<AntiAliasing>,
}

impl CliOptions {
    pub fn from_env() -> Result<Self, String> {
        Self::parse_from(env::args().skip(1))
    }

    pub fn parse_from<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = args.into_iter();
        let mut scene_path = None;
        let mut output = None;
        let mut samples = None;
        let mut format = None;
        let mut gamma = None;
        let mut exposure = None;
        let mut aa = None;

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
                "--gamma" => {
                    gamma = Some(parse_gamma(&next_value(&mut args, arg)?)?);
                }
                "--format" => {
                    format = Some(SceneFormat::parse_name(&next_value(&mut args, arg)?)?);
                }
                "--exposure" => {
                    let value = next_value(&mut args, arg)?;
                    exposure = Some(
                        value
                            .parse()
                            .map_err(|_| format!("invalid exposure value: {value}"))?,
                    );
                }
                "--aa" => {
                    aa = Some(parse_aa(&next_value(&mut args, arg)?)?);
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

        Ok(Self {
            scene_path: scene_path.unwrap_or_else(|| PathBuf::from("scenes/demo.ron")),
            output,
            samples,
            format,
            gamma,
            exposure,
            aa,
        })
    }

    /// Apply CLI overrides onto a loaded scene's render settings.
    pub fn apply_to_scene(&self, scene: &mut Scene) {
        if let Some(ref path) = self.output {
            scene.render.output = path.clone();
        }
        if let Some(samples) = self.samples {
            scene.render.samples_per_pixel = samples;
        }
        if let Some(gamma) = self.gamma {
            scene.render.gamma = gamma;
        }
        if let Some(exposure) = self.exposure {
            scene.render.exposure = exposure;
        }
        if let Some(aa) = self.aa {
            scene.render.aa = aa;
        }
    }
}

fn parse_gamma(value: &str) -> Result<GammaEncoding, String> {
    match value {
        "gamma2" => Ok(GammaEncoding::Gamma2),
        "srgb" => Ok(GammaEncoding::Srgb),
        "linear" => Ok(GammaEncoding::Linear),
        _ => Err(format!(
            "invalid gamma mode: {value} (expected gamma2, srgb, or linear)"
        )),
    }
}

fn parse_aa(value: &str) -> Result<AntiAliasing, String> {
    match value {
        "random" => Ok(AntiAliasing::Random),
        "stratified" => Ok(AntiAliasing::Stratified),
        "halton" => Ok(AntiAliasing::Halton),
        _ => Err(format!(
            "invalid aa mode: {value} (expected random, stratified, or halton)"
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_demo_scene() {
        let options = CliOptions::parse_from::<_, &str>([]).unwrap();
        assert_eq!(options.scene_path, PathBuf::from("scenes/demo.ron"));
        assert_eq!(options.output, None);
        assert_eq!(options.samples, None);
        assert_eq!(options.gamma, None);
        assert_eq!(options.exposure, None);
        assert_eq!(options.aa, None);
        assert_eq!(options.format, None);
    }

    #[test]
    fn parse_args_accepts_format_override() {
        let options = CliOptions::parse_from(["--format", "yaml", "scenes/cornell.yaml"]).unwrap();
        assert_eq!(options.format, Some(SceneFormat::Yaml));
    }

    #[test]
    fn parse_args_rejects_unknown_format() {
        assert!(CliOptions::parse_from(["--format", "toml"]).is_err());
    }

    #[test]
    fn parse_args_accepts_gamma_exposure_and_aa() {
        let options = CliOptions::parse_from([
            "--gamma",
            "srgb",
            "--exposure",
            "1.5",
            "--aa",
            "stratified",
            "scenes/demo.ron",
        ])
        .unwrap();
        assert_eq!(options.gamma, Some(GammaEncoding::Srgb));
        assert_eq!(options.exposure, Some(1.5));
        assert_eq!(options.aa, Some(AntiAliasing::Stratified));
    }

    #[test]
    fn parse_args_accepts_halton_aa() {
        let options = CliOptions::parse_from(["--aa", "halton", "scenes/demo.ron"]).unwrap();
        assert_eq!(options.aa, Some(AntiAliasing::Halton));
    }

    #[test]
    fn parse_args_rejects_unknown_gamma() {
        assert!(CliOptions::parse_from(["--gamma", "rec709"]).is_err());
    }

    #[test]
    fn parse_args_accepts_scene_and_overrides() {
        let options = CliOptions::parse_from([
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
        assert!(CliOptions::parse_from(["--samples", "0"]).is_err());
    }

    #[test]
    fn apply_to_scene_updates_render_settings() {
        let mut scene = Scene::default_demo();
        let options = CliOptions {
            scene_path: PathBuf::from("scenes/demo.ron"),
            output: Some("override.png".into()),
            samples: Some(4),
            format: None,
            gamma: Some(GammaEncoding::Srgb),
            exposure: Some(0.8),
            aa: Some(AntiAliasing::Stratified),
        };
        options.apply_to_scene(&mut scene);
        assert_eq!(scene.render.output, "override.png");
        assert_eq!(scene.render.samples_per_pixel, 4);
        assert_eq!(scene.render.gamma, GammaEncoding::Srgb);
        assert_eq!(scene.render.exposure, 0.8);
        assert_eq!(scene.render.aa, AntiAliasing::Stratified);
    }
}
