use std::fs;
use std::path::Path;

use super::format::SceneFile;

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
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => SceneFormat::Json,
            Some("yaml") | Some("yml") => SceneFormat::Yaml,
            Some("ron") | None => SceneFormat::Ron,
            _ => SceneFormat::Ron,
        }
    }

    pub fn parse(self, text: &str) -> Result<SceneFile, Box<dyn std::error::Error>> {
        match self {
            SceneFormat::Ron => Ok(ron::from_str(text)?),
            SceneFormat::Json => Ok(serde_json::from_str(text)?),
            SceneFormat::Yaml => Ok(serde_yaml::from_str(text)?),
        }
    }
}

pub fn load_scene_file(path: impl AsRef<Path>) -> Result<SceneFile, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)?;
    let format = SceneFormat::from_path(path);
    format.parse(&text).map_err(|e| {
        format!(
            "failed to parse {} as {}: {e}",
            path.display(),
            format
        )
        .into()
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
            super::super::format::MaterialDesc::Dielectric { index } if (index - 1.5).abs() < 1e-9
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
}
