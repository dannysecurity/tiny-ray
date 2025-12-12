use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::format::SceneFile;
use super::validate;

/// Geometry-only scene fragment for `include` resolution (no camera/render required).
#[derive(Debug, Default, serde::Deserialize)]
struct SceneFragment {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    objects: Vec<super::format::SphereDesc>,
    #[serde(default)]
    planes: Vec<super::format::PlaneDesc>,
}

#[derive(Debug)]
enum ParsedScene {
    Root(SceneFile),
    Fragment(SceneFragment),
}

#[derive(Debug)]
struct ResolvedScene {
    camera: Option<super::format::CameraDesc>,
    render: Option<super::format::RenderDesc>,
    objects: Vec<super::format::SphereDesc>,
    planes: Vec<super::format::PlaneDesc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneFormat {
    Ron,
    Json,
    Yaml,
}

impl std::fmt::Display for SceneFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneFormat::Ron => write!(f, "RON"),
            SceneFormat::Json => write!(f, "JSON"),
            SceneFormat::Yaml => write!(f, "YAML"),
        }
    }
}

impl SceneFormat {
    pub fn parse_name(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "ron" => Ok(SceneFormat::Ron),
            "json" => Ok(SceneFormat::Json),
            "yaml" | "yml" => Ok(SceneFormat::Yaml),
            _ => Err(format!(
                "invalid format: {value} (expected ron, json, or yaml)"
            )),
        }
    }

    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => SceneFormat::Json,
            Some("yaml") | Some("yml") => SceneFormat::Yaml,
            Some("ron") => SceneFormat::Ron,
            None => SceneFormat::Ron,
            _ => SceneFormat::Ron,
        }
    }

    /// Pick a parser from the file extension, falling back to content sniffing.
    pub fn detect(path: &Path, text: &str) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => SceneFormat::Json,
            Some("yaml") | Some("yml") => SceneFormat::Yaml,
            Some("ron") => SceneFormat::Ron,
            _ => Self::sniff(text).unwrap_or(SceneFormat::Ron),
        }
    }

    /// Guess the serialization format from leading bytes (useful for extensionless files).
    pub fn sniff(text: &str) -> Option<Self> {
        let trimmed = text.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Some(SceneFormat::Json);
        }
        if trimmed.starts_with('(') {
            return Some(SceneFormat::Ron);
        }
        if trimmed.contains(':') {
            return Some(SceneFormat::Yaml);
        }
        None
    }

    pub fn parse(self, text: &str) -> Result<SceneFile, Box<dyn std::error::Error>> {
        match self {
            SceneFormat::Ron => Ok(ron::from_str(text)?),
            SceneFormat::Json => Ok(serde_json::from_str(text)?),
            SceneFormat::Yaml => Ok(serde_yaml::from_str(text)?),
        }
    }

    fn parse_any(self, text: &str) -> Result<ParsedScene, Box<dyn std::error::Error>> {
        if let Ok(root) = self.parse(text) {
            return Ok(ParsedScene::Root(root));
        }

        let fragment = match self {
            SceneFormat::Ron => ron::from_str(text)?,
            SceneFormat::Json => serde_json::from_str(text)?,
            SceneFormat::Yaml => serde_yaml::from_str(text)?,
        };
        Ok(ParsedScene::Fragment(fragment))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadOptions {
    pub format_override: Option<SceneFormat>,
}

pub fn load_scene_file(path: impl AsRef<Path>) -> Result<SceneFile, Box<dyn std::error::Error>> {
    load_scene_file_with_options(path, LoadOptions::default())
}

pub fn load_scene_file_with_format(
    path: impl AsRef<Path>,
    format_override: Option<SceneFormat>,
) -> Result<SceneFile, Box<dyn std::error::Error>> {
    load_scene_file_with_options(
        path,
        LoadOptions {
            format_override,
        },
    )
}

pub fn load_scene_file_with_options(
    path: impl AsRef<Path>,
    options: LoadOptions,
) -> Result<SceneFile, Box<dyn std::error::Error>> {
    let mut visited = HashSet::new();
    let resolved = load_scene_resolved(path.as_ref(), &options, &mut visited)?;

    let camera = resolved.camera.ok_or_else(|| {
        format!(
            "scene file {} is missing a camera block (fragments must be included from a root scene)",
            path.as_ref().display()
        )
    })?;
    let render = resolved.render.ok_or_else(|| {
        format!(
            "scene file {} is missing a render block (fragments must be included from a root scene)",
            path.as_ref().display()
        )
    })?;

    let mut scene = SceneFile {
        include: Vec::new(),
        camera,
        render,
        objects: resolved.objects,
        planes: resolved.planes,
    };
    validate::normalize(&mut scene);
    validate::validate(&scene)?;
    Ok(scene)
}

fn load_scene_resolved(
    path: &Path,
    options: &LoadOptions,
    visited: &mut HashSet<PathBuf>,
) -> Result<ResolvedScene, Box<dyn std::error::Error>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical) {
        return Err(format!(
            "circular scene include detected at {}",
            path.display()
        )
        .into());
    }

    let text = fs::read_to_string(path)?;
    let format = options
        .format_override
        .unwrap_or_else(|| SceneFormat::detect(path, &text));
    let parsed = format.parse_any(&text).map_err(|e| -> Box<dyn std::error::Error> {
        format!(
            "failed to parse {} as {}: {e}",
            path.display(),
            format
        )
        .into()
    })?;

    let (includes, mut camera, mut render, mut scene_objects, mut scene_planes) = match parsed {
        ParsedScene::Root(scene) => (
            scene.include,
            Some(scene.camera),
            Some(scene.render),
            scene.objects,
            scene.planes,
        ),
        ParsedScene::Fragment(fragment) => (
            fragment.include,
            None,
            None,
            fragment.objects,
            fragment.planes,
        ),
    };

    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut merged_objects = Vec::new();
    let mut merged_planes = Vec::new();

    for rel in includes {
        let rel = rel.trim();
        if rel.is_empty() {
            return Err(validate::SceneValidationError::EmptyIncludePath { index: 0 }.into());
        }
        let include_path = base.join(rel);
        let fragment = load_scene_resolved(&include_path, options, visited)?;
        merged_objects.extend(fragment.objects);
        merged_planes.extend(fragment.planes);
        if camera.is_none() {
            camera = fragment.camera;
        }
        if render.is_none() {
            render = fragment.render;
        }
    }

    merged_objects.append(&mut scene_objects);
    merged_planes.append(&mut scene_planes);

    Ok(ResolvedScene {
        camera,
        render,
        objects: merged_objects,
        planes: merged_planes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::GammaEncoding;
    use crate::sampling::AntiAliasing;

    const MINIMAL_JSON: &str = r#"{
        "camera": {
            "lookfrom": [0.0, 0.0, 5.0],
            "lookat": [0.0, 0.0, 0.0],
            "vup": [0.0, 1.0, 0.0],
            "vfov": 45.0,
            "aperture": 0.0,
            "focus_distance": 1.0
        },
        "render": {
            "width": 64,
            "height": 64,
            "samples_per_pixel": 1,
            "max_depth": 4,
            "output": "test.png"
        },
        "objects": [
            {
                "center": [0.0, 0.0, -1.0],
                "radius": 0.5,
                "material": { "Lambertian": { "albedo": [0.7, 0.3, 0.1] } }
            }
        ]
    }"#;

    const MINIMAL_YAML: &str = r#"
camera:
  lookfrom: [0.0, 0.0, 5.0]
  lookat: [0.0, 0.0, 0.0]
  vup: [0.0, 1.0, 0.0]
  vfov: 45.0
  aperture: 0.0
  focus_distance: 1.0
render:
  width: 64
  height: 64
  samples_per_pixel: 1
  max_depth: 4
  output: test.png
objects:
  - center: [0.0, 0.0, -1.0]
    radius: 0.5
    material:
      Metal:
        albedo: [0.9, 0.9, 0.9]
        fuzz: 0.1
"#;

    const MINIMAL_RON: &str = r#"(
        camera: (
            lookfrom: (0.0, 0.0, 5.0),
            lookat: (0.0, 0.0, 0.0),
            vup: (0.0, 1.0, 0.0),
            vfov: 45.0,
            aperture: 0.0,
            focus_distance: 1.0,
        ),
        render: (
            width: 64,
            height: 64,
            samples_per_pixel: 1,
            max_depth: 4,
            output: "test.png",
        ),
        objects: [
            (
                center: (0.0, 0.0, -1.0),
                radius: 0.5,
                material: Dielectric(index: 1.5),
            ),
        ],
    )"#;

    #[test]
    fn json_scene_parses_camera_and_objects() {
        let scene = SceneFormat::Json.parse(MINIMAL_JSON).unwrap();
        assert_eq!(scene.render.width, 64);
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.camera.vfov, 45.0);
    }

    #[test]
    fn yaml_scene_parses_material_variants() {
        let scene = SceneFormat::Yaml.parse(MINIMAL_YAML).unwrap();
        assert_eq!(scene.objects.len(), 1);
        assert!(matches!(
            scene.objects[0].material,
            super::super::format::MaterialDesc::Metal { .. }
        ));
    }

    #[test]
    fn ron_scene_parses_dielectric_material() {
        let scene = SceneFormat::Ron.parse(MINIMAL_RON).unwrap();
        assert!(matches!(
            scene.objects[0].material,
            super::super::format::MaterialDesc::Dielectric { index }
            if (index - 1.5).abs() < 1e-9
        ));
    }

    #[test]
    fn format_detection_from_extension() {
        assert_eq!(
            SceneFormat::from_path(Path::new("scenes/demo.json")),
            SceneFormat::Json
        );
        assert_eq!(
            SceneFormat::from_path(Path::new("scenes/demo.yaml")),
            SceneFormat::Yaml
        );
        assert_eq!(
            SceneFormat::from_path(Path::new("scenes/demo.yml")),
            SceneFormat::Yaml
        );
        assert_eq!(
            SceneFormat::from_path(Path::new("scenes/demo.ron")),
            SceneFormat::Ron
        );
    }

    #[test]
    fn format_sniff_detects_json_and_yaml() {
        assert_eq!(SceneFormat::sniff(MINIMAL_JSON), Some(SceneFormat::Json));
        assert_eq!(SceneFormat::sniff(MINIMAL_YAML), Some(SceneFormat::Yaml));
        assert_eq!(SceneFormat::sniff(MINIMAL_RON), Some(SceneFormat::Ron));
    }

    #[test]
    fn format_detect_sniffs_extensionless_yaml() {
        let path = Path::new("scenes/my-scene");
        assert_eq!(
            SceneFormat::detect(path, MINIMAL_YAML),
            SceneFormat::Yaml
        );
    }

    #[test]
    fn format_override_parses_yaml_with_json_extension() {
        let path = std::env::temp_dir().join("tiny_ray_yaml_as_json.json");
        fs::write(&path, MINIMAL_YAML).unwrap();
        let scene = load_scene_file_with_format(
            &path,
            Some(SceneFormat::Yaml),
        )
        .unwrap();
        assert_eq!(scene.objects.len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn render_desc_defaults_gamma_exposure_and_aa() {
        let scene = SceneFormat::Json.parse(MINIMAL_JSON).unwrap();
        assert_eq!(scene.render.gamma, GammaEncoding::Gamma2);
        assert_eq!(scene.render.exposure, 1.0);
        assert_eq!(scene.render.aa, AntiAliasing::Random);
    }

    #[test]
    fn render_desc_parses_gamma_exposure_and_aa() {
        let json = r#"{
            "camera": {
                "lookfrom": [0.0, 0.0, 5.0],
                "lookat": [0.0, 0.0, 0.0],
                "vup": [0.0, 1.0, 0.0],
                "vfov": 45.0,
                "aperture": 0.0,
                "focus_distance": 1.0
            },
            "render": {
                "width": 32,
                "height": 32,
                "samples_per_pixel": 4,
                "max_depth": 4,
                "output": "tonemapped.png",
                "gamma": "srgb",
                "exposure": 1.25,
                "aa": "stratified"
            },
            "objects": []
        }"#;
        let scene = SceneFormat::Json.parse(json).unwrap();
        assert_eq!(scene.render.gamma, GammaEncoding::Srgb);
        assert_eq!(scene.render.exposure, 1.25);
        assert_eq!(scene.render.aa, AntiAliasing::Stratified);
    }

    #[test]
    fn render_desc_parses_halton_aa() {
        let json = r#"{
            "camera": {
                "lookfrom": [0.0, 0.0, 5.0],
                "lookat": [0.0, 0.0, 0.0],
                "vup": [0.0, 1.0, 0.0],
                "vfov": 45.0,
                "aperture": 0.0,
                "focus_distance": 1.0
            },
            "render": {
                "width": 32,
                "height": 32,
                "samples_per_pixel": 16,
                "max_depth": 4,
                "output": "halton.png",
                "aa": "halton"
            },
            "objects": []
        }"#;
        let scene = SceneFormat::Json.parse(json).unwrap();
        assert_eq!(scene.render.aa, AntiAliasing::Halton);
    }

    #[test]
    fn studio_formats_share_gamma_and_aa_settings() {
        let ron = load_scene_file("scenes/studio.ron").unwrap();
        let json = load_scene_file("scenes/studio.json").unwrap();
        let yaml = load_scene_file("scenes/studio.yaml").unwrap();
        assert_eq!(json.render.gamma, GammaEncoding::Srgb);
        assert_eq!(yaml.render.gamma, ron.render.gamma);
        assert_eq!(json.render.aa, AntiAliasing::Stratified);
        assert_eq!(yaml.render.aa, ron.render.aa);
    }

    #[test]
    fn demo_json_on_disk_matches_ron_object_count() {
        let json = load_scene_file("scenes/demo.json").unwrap();
        let ron = load_scene_file("scenes/demo.ron").unwrap();
        assert_eq!(json.objects.len(), ron.objects.len());
        assert_eq!(json.render.width, ron.render.width);
        assert_eq!(json.camera.lookfrom, ron.camera.lookfrom);
    }

    #[test]
    fn demo_yaml_on_disk_matches_ron_object_count() {
        let yaml = load_scene_file("scenes/demo.yaml").unwrap();
        let ron = load_scene_file("scenes/demo.ron").unwrap();
        assert_eq!(yaml.objects.len(), ron.objects.len());
        assert_eq!(yaml.render.output, ron.render.output);
    }

    #[test]
    fn studio_json_on_disk_matches_ron_object_count() {
        let json = load_scene_file("scenes/studio.json").unwrap();
        let ron = load_scene_file("scenes/studio.ron").unwrap();
        assert_eq!(json.objects.len(), ron.objects.len());
        assert_eq!(json.render.output, ron.render.output);
        assert_eq!(json.camera.lookfrom, ron.camera.lookfrom);
    }

    #[test]
    fn parse_error_includes_path_and_format() {
        let path = std::env::temp_dir().join("tiny_ray_bad_scene.json");
        fs::write(&path, "{ not valid json").unwrap();
        let err = load_scene_file(&path).unwrap_err().to_string();
        assert!(err.contains("JSON"), "{err}");
        assert!(err.contains(path.display().to_string().as_str()), "{err}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn validation_rejects_bad_radius_on_disk() {
        let path = std::env::temp_dir().join("tiny_ray_bad_radius.json");
        fs::write(
            &path,
            r#"{
                "camera": {
                    "lookfrom": [0,0,5], "lookat": [0,0,0], "vup": [0,1,0],
                    "vfov": 45, "aperture": 0, "focus_distance": 5
                },
                "render": {
                    "width": 8, "height": 8, "samples_per_pixel": 1,
                    "max_depth": 4, "output": "x.png"
                },
                "objects": [{ "center": [0,0,0], "radius": -1, "material": { "Lambertian": { "albedo": [1,1,1] } } }]
            }"#,
        )
        .unwrap();
        let err = load_scene_file(&path).unwrap_err().to_string();
        assert!(err.contains("radius must be > 0"), "{err}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn modular_cornell_yaml_matches_monolithic_planes_and_objects() {
        let modular = load_scene_file("scenes/cornell-modular.yaml").unwrap();
        let monolithic = load_scene_file("scenes/cornell.yaml").unwrap();
        assert_eq!(modular.planes.len(), monolithic.planes.len());
        assert_eq!(modular.objects.len(), monolithic.objects.len());
    }

    #[test]
    fn modular_cornell_json_matches_monolithic() {
        let modular = load_scene_file("scenes/cornell-modular.json").unwrap();
        let monolithic = load_scene_file("scenes/cornell.json").unwrap();
        assert_eq!(modular.planes.len(), monolithic.planes.len());
        assert_eq!(modular.objects.len(), monolithic.objects.len());
    }
}
