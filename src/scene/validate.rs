use std::fmt;

use super::format::SceneFile;

#[derive(Debug, Clone, PartialEq)]
pub enum SceneValidationError {
    EmptyScene,
    InvalidRenderWidth(u32),
    InvalidRenderHeight(u32),
    InvalidSamplesPerPixel(u32),
    InvalidMaxDepth(u32),
    InvalidCameraVfov(f64),
    NegativeAperture(f64),
    InvalidFocusDistance(f64),
    InvalidSphereRadius { index: usize, radius: f64 },
    ZeroPlaneNormal { index: usize },
    EmptyIncludePath { index: usize },
}

impl fmt::Display for SceneValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScene => write!(f, "scene must contain at least one object or plane"),
            Self::InvalidRenderWidth(w) => write!(f, "render width must be > 0 (got {w})"),
            Self::InvalidRenderHeight(h) => write!(f, "render height must be > 0 (got {h})"),
            Self::InvalidSamplesPerPixel(n) => {
                write!(f, "samples_per_pixel must be >= 1 (got {n})")
            }
            Self::InvalidMaxDepth(d) => write!(f, "max_depth must be >= 1 (got {d})"),
            Self::InvalidCameraVfov(v) => write!(f, "camera vfov must be in (0, 180) (got {v})"),
            Self::NegativeAperture(a) => write!(f, "camera aperture must be >= 0 (got {a})"),
            Self::InvalidFocusDistance(d) => write!(f, "camera focus_distance must be > 0 (got {d})"),
            Self::InvalidSphereRadius { index, radius } => write!(
                f,
                "sphere {index} radius must be > 0 (got {radius})"
            ),
            Self::ZeroPlaneNormal { index } => write!(f, "plane {index} normal must be non-zero"),
            Self::EmptyIncludePath { index } => {
                write!(f, "include[{index}] must not be an empty path")
            }
        }
    }
}

impl std::error::Error for SceneValidationError {}

/// Apply schema defaults that depend on multiple fields (e.g. camera focus distance).
pub fn normalize(scene: &mut SceneFile) {
    if scene.camera.focus_distance <= 0.0 {
        let from = scene.camera.lookfrom;
        let at = scene.camera.lookat;
        let dx = from[0] - at[0];
        let dy = from[1] - at[1];
        let dz = from[2] - at[2];
        scene.camera.focus_distance = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
    }
}

pub fn validate(scene: &SceneFile) -> Result<(), SceneValidationError> {
    for (index, path) in scene.include.iter().enumerate() {
        if path.trim().is_empty() {
            return Err(SceneValidationError::EmptyIncludePath { index });
        }
    }

    if scene.render.width == 0 {
        return Err(SceneValidationError::InvalidRenderWidth(scene.render.width));
    }
    if scene.render.height == 0 {
        return Err(SceneValidationError::InvalidRenderHeight(scene.render.height));
    }
    if scene.render.samples_per_pixel == 0 {
        return Err(SceneValidationError::InvalidSamplesPerPixel(
            scene.render.samples_per_pixel,
        ));
    }
    if scene.render.max_depth == 0 {
        return Err(SceneValidationError::InvalidMaxDepth(scene.render.max_depth));
    }

    if scene.camera.vfov <= 0.0 || scene.camera.vfov >= 180.0 {
        return Err(SceneValidationError::InvalidCameraVfov(scene.camera.vfov));
    }
    if scene.camera.aperture < 0.0 {
        return Err(SceneValidationError::NegativeAperture(scene.camera.aperture));
    }
    if scene.camera.focus_distance <= 0.0 {
        return Err(SceneValidationError::InvalidFocusDistance(
            scene.camera.focus_distance,
        ));
    }

    for (index, sphere) in scene.objects.iter().enumerate() {
        if sphere.radius <= 0.0 {
            return Err(SceneValidationError::InvalidSphereRadius {
                index,
                radius: sphere.radius,
            });
        }
    }

    for (index, plane) in scene.planes.iter().enumerate() {
        let n = plane.normal;
        let len_sq = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        if len_sq <= 1e-12 {
            return Err(SceneValidationError::ZeroPlaneNormal { index });
        }
    }

    if scene.objects.is_empty() && scene.planes.is_empty() {
        return Err(SceneValidationError::EmptyScene);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::format::{CameraDesc, MaterialDesc, PlaneDesc, RenderDesc, SceneFile, SphereDesc};

    fn minimal_scene() -> SceneFile {
        SceneFile {
            include: Vec::new(),
            camera: CameraDesc {
                lookfrom: [0.0, 0.0, 5.0],
                lookat: [0.0, 0.0, 0.0],
                vup: [0.0, 1.0, 0.0],
                vfov: 45.0,
                aperture: 0.0,
                focus_distance: 5.0,
            },
            render: RenderDesc {
                width: 64,
                height: 64,
                samples_per_pixel: 1,
                max_depth: 4,
                output: "test.png".into(),
                gamma: Default::default(),
                exposure: 1.0,
                aa: Default::default(),
            },
            objects: vec![SphereDesc {
                center: [0.0, 0.0, -1.0],
                radius: 0.5,
                material: MaterialDesc::Lambertian {
                    albedo: [0.7, 0.3, 0.1],
                },
            }],
            planes: Vec::new(),
        }
    }

    #[test]
    fn valid_minimal_scene_passes() {
        validate(&minimal_scene()).unwrap();
    }

    #[test]
    fn rejects_empty_scene() {
        let mut scene = minimal_scene();
        scene.objects.clear();
        assert_eq!(validate(&scene), Err(SceneValidationError::EmptyScene));
    }

    #[test]
    fn rejects_zero_sphere_radius() {
        let mut scene = minimal_scene();
        scene.objects[0].radius = 0.0;
        assert_eq!(
            validate(&scene),
            Err(SceneValidationError::InvalidSphereRadius {
                index: 0,
                radius: 0.0
            })
        );
    }

    #[test]
    fn rejects_zero_plane_normal() {
        let mut scene = minimal_scene();
        scene.planes.push(PlaneDesc {
            point: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
            material: MaterialDesc::Lambertian {
                albedo: [0.5, 0.5, 0.5],
            },
        });
        assert_eq!(
            validate(&scene),
            Err(SceneValidationError::ZeroPlaneNormal { index: 0 })
        );
    }

    #[test]
    fn rejects_invalid_render_dimensions() {
        let mut scene = minimal_scene();
        scene.render.width = 0;
        assert_eq!(
            validate(&scene),
            Err(SceneValidationError::InvalidRenderWidth(0))
        );
    }

    #[test]
    fn rejects_empty_include_path() {
        let mut scene = minimal_scene();
        scene.include.push("".into());
        assert_eq!(
            validate(&scene),
            Err(SceneValidationError::EmptyIncludePath { index: 0 })
        );
    }

    #[test]
    fn normalize_sets_focus_distance_from_camera_vectors() {
        let mut scene = minimal_scene();
        scene.camera.focus_distance = 0.0;
        normalize(&mut scene);
        assert!((scene.camera.focus_distance - 5.0).abs() < 1e-9);
        validate(&scene).unwrap();
    }

    #[test]
    fn plane_only_scene_is_valid() {
        let mut scene = minimal_scene();
        scene.objects.clear();
        scene.planes.push(PlaneDesc {
            point: [0.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            material: MaterialDesc::Lambertian {
                albedo: [0.5, 0.5, 0.5],
            },
        });
        validate(&scene).unwrap();
    }
}
