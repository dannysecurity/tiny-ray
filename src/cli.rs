//! Command-line argument parsing and scene override application.

use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::color::{GammaEncoding, InputColorSpace, ToneMapping};
use crate::dither::DitherMode;
use crate::film::PixelFilter;
use crate::sampling::AntiAliasing;
use crate::scene::{load_scene_file_with_format, load_scene_from_str, Scene, SceneFile, SceneFormat};

pub const USAGE: &str = "\
Usage: tiny-ray [OPTIONS] [SCENE]

Path-traced sphere and plane renderer. Loads a scene file (RON, JSON, or YAML) and writes
an image to the path given in the scene, or overridden with --output.

Arguments:
  SCENE    Scene file path (default: scenes/demo.ron)

Options:
  -o, --output PATH     Override the output image path from the scene file
  -s, --samples N       Override samples per pixel (useful for quick previews)
      --width W         Override image width in pixels
      --height H        Override image height in pixels
      --format FMT      Force scene parser: ron, json, or yaml (default: from extension)
      --gamma MODE      Override output gamma: gamma2, srgb, or linear
      --color-space MODE  Override scene color interpretation: linear or srgb
      --exposure F      Override linear exposure multiplier (default 1.0)
      --tone-map MODE   Override HDR tone mapping: none, reinhard, or aces
      --dither MODE     Override 8-bit quantization dither: none or bayer8x8
      --aa MODE         Override anti-aliasing: random, stratified, halton, or r2
      --filter MODE     Override pixel reconstruction filter: box, gaussian, or mitchell
      --bvh-stats       Print BVH node counts after scene build (bounded geometry only)
      --validate        Parse and validate the scene, print a summary, and exit without rendering
  -h, --help            Show this help message

Examples:
  cargo run --release -- scenes/studio.ron
  cargo run --release -- --samples 10 --output preview.png scenes/studio.json
  cargo run --release -- --width 400 --height 225 --samples 8 scenes/neon.ron
  cargo run --release -- --format yaml scenes/cornell-modular.yaml
  cargo run --release -- --validate scenes/cornell.json
  cargo run --release -- --format json --validate -
  cargo run --release -- --gamma srgb --tone-map aces --aa stratified --filter mitchell scenes/studio.ron
";

#[derive(Debug, Default, PartialEq)]
pub struct CliOptions {
    pub scene_path: PathBuf,
    pub output: Option<String>,
    pub samples: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<SceneFormat>,
    pub gamma: Option<GammaEncoding>,
    pub color_space: Option<InputColorSpace>,
    pub exposure: Option<f64>,
    pub tone_map: Option<ToneMapping>,
    pub dither: Option<DitherMode>,
    pub aa: Option<AntiAliasing>,
    pub filter: Option<PixelFilter>,
    pub bvh_stats: bool,
    pub validate: bool,
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
        let mut width = None;
        let mut height = None;
        let mut format = None;
        let mut gamma = None;
        let mut color_space = None;
        let mut exposure = None;
        let mut tone_map = None;
        let mut dither = None;
        let mut aa = None;
        let mut filter = None;
        let mut bvh_stats = false;
        let mut validate = false;

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
                "--width" => {
                    let value = next_value(&mut args, arg)?;
                    width = Some(
                        value
                            .parse()
                            .map_err(|_| format!("invalid width value: {value}"))?,
                    );
                    if width == Some(0) {
                        return Err("width must be at least 1".into());
                    }
                }
                "--height" => {
                    let value = next_value(&mut args, arg)?;
                    height = Some(
                        value
                            .parse()
                            .map_err(|_| format!("invalid height value: {value}"))?,
                    );
                    if height == Some(0) {
                        return Err("height must be at least 1".into());
                    }
                }
                "--gamma" => {
                    gamma = Some(parse_gamma(&next_value(&mut args, arg)?)?);
                }
                "--color-space" => {
                    color_space = Some(parse_color_space(&next_value(&mut args, arg)?)?);
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
                "--tone-map" => {
                    tone_map = Some(parse_tone_map(&next_value(&mut args, arg)?)?);
                }
                "--dither" => {
                    dither = Some(parse_dither(&next_value(&mut args, arg)?)?);
                }
                "--aa" => {
                    aa = Some(parse_aa(&next_value(&mut args, arg)?)?);
                }
                "--filter" => {
                    filter = Some(parse_filter(&next_value(&mut args, arg)?)?);
                }
                "--bvh-stats" => {
                    bvh_stats = true;
                }
                "--validate" => {
                    validate = true;
                }
                "-" => {
                    if scene_path.is_some() {
                        return Err("unexpected extra argument: -".into());
                    }
                    scene_path = Some(PathBuf::from("-"));
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
            width,
            height,
            format,
            gamma,
            color_space,
            exposure,
            tone_map,
            dither,
            aa,
            filter,
            bvh_stats,
            validate,
        })
    }

    /// Load a scene from disk, stdin (`-`), or the built-in demo fallback.
    pub fn load_scene(&self) -> Result<(Scene, SceneFile, String), Box<dyn std::error::Error>> {
        if self.scene_path.as_os_str() == "-" {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            let file = load_scene_from_str(&text, self.format)?;
            let mut scene = Scene::from_scene_file(file.clone());
            self.apply_to_scene(&mut scene);
            return Ok((scene, file, "<stdin>".into()));
        }

        if self.scene_path.exists() {
            let file = load_scene_file_with_format(&self.scene_path, self.format)?;
            let mut scene = Scene::from_scene_file(file.clone());
            self.apply_to_scene(&mut scene);
            let source = self.scene_path.display().to_string();
            return Ok((scene, file, source));
        }

        eprintln!("Scene file not found; using built-in demo scene");
        let file = Scene::default_demo_file();
        let mut scene = Scene::from_scene_file(file.clone());
        self.apply_to_scene(&mut scene);
        Ok((scene, file, "built-in demo".into()))
    }

    pub fn print_validation_summary(
        &self,
        scene: &Scene,
        file: &SceneFile,
        source: &str,
    ) {
        scene.print_validation_summary(source, file);
    }

    /// Apply CLI overrides onto render settings before building the scene world.
    pub fn apply_to_render(&self, render: &mut crate::scene::RenderDesc) {
        if let Some(ref path) = self.output {
            render.output = path.clone();
        }
        if let Some(samples) = self.samples {
            render.samples_per_pixel = samples;
        }
        if let Some(width) = self.width {
            render.width = width;
        }
        if let Some(height) = self.height {
            render.height = height;
        }
        if let Some(gamma) = self.gamma {
            render.gamma = gamma;
        }
        if let Some(color_space) = self.color_space {
            render.color_space = color_space;
        }
        if let Some(exposure) = self.exposure {
            render.exposure = exposure;
        }
        if let Some(tone_map) = self.tone_map {
            render.tone_map = tone_map;
        }
        if let Some(dither) = self.dither {
            render.dither = dither;
        }
        if let Some(aa) = self.aa {
            render.aa = aa;
        }
        if let Some(filter) = self.filter {
            render.filter = filter;
        }
    }

    /// Apply CLI overrides onto a loaded scene's render settings.
    pub fn apply_to_scene(&self, scene: &mut Scene) {
        self.apply_to_render(&mut scene.render);
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

fn parse_color_space(value: &str) -> Result<InputColorSpace, String> {
    match value {
        "linear" => Ok(InputColorSpace::Linear),
        "srgb" => Ok(InputColorSpace::Srgb),
        _ => Err(format!(
            "invalid color-space mode: {value} (expected linear or srgb)"
        )),
    }
}

fn parse_tone_map(value: &str) -> Result<ToneMapping, String> {
    match value {
        "none" => Ok(ToneMapping::None),
        "reinhard" => Ok(ToneMapping::Reinhard),
        "aces" => Ok(ToneMapping::Aces),
        _ => Err(format!(
            "invalid tone-map mode: {value} (expected none, reinhard, or aces)"
        )),
    }
}

fn parse_filter(value: &str) -> Result<PixelFilter, String> {
    match value {
        "box" => Ok(PixelFilter::Box),
        "gaussian" => Ok(PixelFilter::Gaussian),
        "mitchell" => Ok(PixelFilter::Mitchell),
        _ => Err(format!(
            "invalid filter mode: {value} (expected box, gaussian, or mitchell)"
        )),
    }
}

fn parse_dither(value: &str) -> Result<DitherMode, String> {
    match value {
        "none" => Ok(DitherMode::None),
        "bayer8x8" => Ok(DitherMode::Bayer8x8),
        _ => Err(format!(
            "invalid dither mode: {value} (expected none or bayer8x8)"
        )),
    }
}

fn parse_aa(value: &str) -> Result<AntiAliasing, String> {
    match value {
        "random" => Ok(AntiAliasing::Random),
        "stratified" => Ok(AntiAliasing::Stratified),
        "halton" => Ok(AntiAliasing::Halton),
        "r2" => Ok(AntiAliasing::R2),
        _ => Err(format!(
            "invalid aa mode: {value} (expected random, stratified, halton, or r2)"
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
        assert_eq!(options.width, None);
        assert_eq!(options.height, None);
        assert_eq!(options.gamma, None);
        assert_eq!(options.color_space, None);
        assert_eq!(options.exposure, None);
        assert_eq!(options.tone_map, None);
        assert_eq!(options.dither, None);
        assert_eq!(options.aa, None);
        assert_eq!(options.filter, None);
        assert_eq!(options.format, None);
        assert!(!options.bvh_stats);
        assert!(!options.validate);
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
    fn parse_args_accepts_color_space_override() {
        let options =
            CliOptions::parse_from(["--color-space", "srgb", "scenes/demo.ron"]).unwrap();
        assert_eq!(options.color_space, Some(InputColorSpace::Srgb));
    }

    #[test]
    fn parse_args_rejects_unknown_color_space() {
        assert!(CliOptions::parse_from(["--color-space", "rec709"]).is_err());
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
    fn parse_args_accepts_tone_map_override() {
        let options =
            CliOptions::parse_from(["--tone-map", "aces", "scenes/neon.ron"]).unwrap();
        assert_eq!(options.tone_map, Some(ToneMapping::Aces));
    }

    #[test]
    fn parse_args_rejects_unknown_tone_map() {
        assert!(CliOptions::parse_from(["--tone-map", "filmic"]).is_err());
    }

    #[test]
    fn parse_args_accepts_filter_override() {
        let options =
            CliOptions::parse_from(["--filter", "mitchell", "scenes/demo.ron"]).unwrap();
        assert_eq!(options.filter, Some(PixelFilter::Mitchell));
    }

    #[test]
    fn parse_args_rejects_unknown_filter() {
        assert!(CliOptions::parse_from(["--filter", "lanczos"]).is_err());
    }

    #[test]
    fn parse_args_accepts_dither_override() {
        let options =
            CliOptions::parse_from(["--dither", "bayer8x8", "scenes/demo.ron"]).unwrap();
        assert_eq!(options.dither, Some(DitherMode::Bayer8x8));
    }

    #[test]
    fn parse_args_rejects_unknown_dither() {
        assert!(CliOptions::parse_from(["--dither", "floyd"]).is_err());
    }

    #[test]
    fn parse_args_accepts_r2_aa() {
        let options = CliOptions::parse_from(["--aa", "r2", "scenes/demo.ron"]).unwrap();
        assert_eq!(options.aa, Some(AntiAliasing::R2));
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
    fn parse_args_accepts_bvh_stats_flag() {
        let options =
            CliOptions::parse_from(["--bvh-stats", "scenes/studio.ron"]).unwrap();
        assert!(options.bvh_stats);
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
    fn parse_args_accepts_width_and_height() {
        let options = CliOptions::parse_from([
            "--width",
            "320",
            "--height",
            "180",
            "scenes/neon.ron",
        ])
        .unwrap();
        assert_eq!(options.width, Some(320));
        assert_eq!(options.height, Some(180));
    }

    #[test]
    fn parse_args_rejects_zero_width() {
        assert!(CliOptions::parse_from(["--width", "0"]).is_err());
    }

    #[test]
    fn parse_args_rejects_zero_height() {
        assert!(CliOptions::parse_from(["--height", "0"]).is_err());
    }

    #[test]
    fn parse_args_accepts_validate_flag() {
        let options = CliOptions::parse_from(["--validate", "scenes/demo.json"]).unwrap();
        assert!(options.validate);
    }

    #[test]
    fn parse_args_accepts_stdin_scene_path() {
        let options = CliOptions::parse_from(["-", "--format", "json"]).unwrap();
        assert_eq!(options.scene_path, PathBuf::from("-"));
        assert_eq!(options.format, Some(SceneFormat::Json));
    }

    #[test]
    fn load_scene_reads_json_from_disk() {
        let options = CliOptions::parse_from(["scenes/demo.json"]).unwrap();
        let (scene, file, source) = options.load_scene().unwrap();
        assert_eq!(source, "scenes/demo.json");
        assert_eq!(file.objects.len(), 6);
        assert_eq!(scene.lights.len(), 1);
    }

    #[test]
    fn load_scene_from_str_builds_runtime_scene() {
        let json = r#"{
            "camera": {
                "lookfrom": [0.0, 0.0, 5.0],
                "lookat": [0.0, 0.0, 0.0],
                "vup": [0.0, 1.0, 0.0],
                "vfov": 45.0,
                "aperture": 0.0,
                "focus_distance": 5.0
            },
            "render": {
                "width": 16,
                "height": 16,
                "samples_per_pixel": 1,
                "max_depth": 4,
                "output": "inline.png"
            },
            "objects": [
                {
                    "center": [0.0, 0.0, -1.0],
                    "radius": 0.5,
                    "material": { "Emissive": { "color": [1.0, 1.0, 1.0], "intensity": 2.0 } }
                }
            ]
        }"#;
        let scene = Scene::from_str(json, Some(SceneFormat::Json)).unwrap();
        assert_eq!(scene.render.output, "inline.png");
        assert_eq!(scene.lights.len(), 1);
    }

    #[test]
    fn apply_to_scene_updates_render_settings() {
        let mut scene = Scene::default_demo();
        let options = CliOptions {
            scene_path: PathBuf::from("scenes/demo.ron"),
            output: Some("override.png".into()),
            samples: Some(4),
            width: Some(320),
            height: Some(240),
            format: None,
            gamma: Some(GammaEncoding::Srgb),
            color_space: Some(InputColorSpace::Srgb),
            exposure: Some(0.8),
            tone_map: Some(ToneMapping::Reinhard),
            dither: Some(DitherMode::Bayer8x8),
            aa: Some(AntiAliasing::Stratified),
            filter: Some(PixelFilter::Gaussian),
            bvh_stats: false,
            validate: false,
        };
        options.apply_to_scene(&mut scene);
        assert_eq!(scene.render.output, "override.png");
        assert_eq!(scene.render.samples_per_pixel, 4);
        assert_eq!(scene.render.width, 320);
        assert_eq!(scene.render.height, 240);
        assert_eq!(scene.render.gamma, GammaEncoding::Srgb);
        assert_eq!(scene.render.color_space, InputColorSpace::Srgb);
        assert_eq!(scene.render.exposure, 0.8);
        assert_eq!(scene.render.tone_map, ToneMapping::Reinhard);
        assert_eq!(scene.render.dither, DitherMode::Bayer8x8);
        assert_eq!(scene.render.aa, AntiAliasing::Stratified);
        assert_eq!(scene.render.filter, PixelFilter::Gaussian);
    }
}
