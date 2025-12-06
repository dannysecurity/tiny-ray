# tiny-ray

A minimal path tracer written from scratch in Rust. Renders scenes of spheres with physically inspired materials, area lighting, and a bounding volume hierarchy (BVH) for fast ray–object intersection.

## Features

- **Primitives** — analytic sphere intersection
- **Materials** — Lambertian diffuse, fuzzy metal, dielectric (glass), and emissive light sources
- **Lighting** — emissive spheres act as area lights with next-event estimation (direct light sampling and shadow rays); sky gradient for ambient fill
- **Acceleration** — SAH-style axis split BVH built over scene objects
- **Scene files** — declarative scene descriptions in RON, JSON, or YAML

## Quick start

```bash
cargo run --release -- scenes/demo.ron
# or
cargo run --release -- scenes/demo.json
cargo run --release -- scenes/demo.yaml
```

Without a scene path, the built-in demo scene is used. Output is written to the path specified in the scene file (default: `output.png`).

### CLI overrides

Override the output path or sample count without editing the scene file:

```bash
cargo run --release -- --samples 10 --output preview.png scenes/studio.ron
```

| Flag | Description |
|------|-------------|
| `-o`, `--output PATH` | Write the image to `PATH` instead of the scene's `render.output` |
| `-s`, `--samples N` | Trace `N` samples per pixel (minimum 1) |
| `-h`, `--help` | Print usage |

## Example render: glass studio

The `scenes/studio.*` files set up a Cornell-box-style room built from large spheres: red and green side walls, a gray floor and back wall, and a small emissive sphere as the ceiling light. Three smaller spheres sit on the floor — a glass dielectric orb in the center, a gold metal sphere on the left, and a blue diffuse sphere on the right. Depth of field comes from a thin-lens camera (`aperture: 0.05`).

```bash
cargo run --release -- scenes/studio.ron
# writes studio.png (800×450, 100 spp)

# quick preview while tuning materials or camera
cargo run --release -- --samples 16 --output studio-preview.png scenes/studio.json
```

Scene excerpt (RON):

```ron
(
    camera: (
        lookfrom: (0.0, 2.5, 6.0),
        lookat: (0.0, 0.5, 0.0),
        vfov: 35.0,
        aperture: 0.05,
        focus_distance: 6.0,
    ),
    render: (
        width: 800,
        height: 450,
        samples_per_pixel: 100,
        max_depth: 50,
        output: "studio.png",
    ),
    objects: [
        (
            center: (0.0, 0.5, 0.0),
            radius: 0.5,
            material: Dielectric(index: 1.5),
        ),
        (
            center: (0.0, 2.8, 0.0),
            radius: 0.6,
            material: Emissive(color: (1.0, 0.98, 0.9), intensity: 8.0),
        ),
        // ... colored walls, metal and diffuse spheres
    ],
)
```

The same scene is available as `scenes/studio.json` and `scenes/studio.yaml`.

## Scene format

Scenes are loaded by file extension (`.ron`, `.json`, `.yaml`/`.yml`). Each file describes the camera, render settings, and a list of spheres. The same schema works across all three formats.

### RON example

```ron
(
    camera: (
        lookfrom: (13.0, 2.0, 3.0),
        lookat: (0.0, 1.0, 0.0),
        vup: (0.0, 1.0, 0.0),
        vfov: 20.0,
        aperture: 0.1,
        focus_distance: 10.0,
    ),
    render: (
        width: 800,
        height: 450,
        samples_per_pixel: 50,
        max_depth: 50,
        output: "output.png",
    ),
    objects: [
        (
            center: (0.0, 1.0, 0.0),
            radius: 1.0,
            material: Lambertian(albedo: (0.8, 0.2, 0.2)),
        ),
    ],
)
```

### JSON example

```json
{
  "camera": {
    "lookfrom": [13.0, 2.0, 3.0],
    "lookat": [0.0, 1.0, 0.0],
    "vup": [0.0, 1.0, 0.0],
    "vfov": 20.0,
    "aperture": 0.1,
    "focus_distance": 10.0
  },
  "render": {
    "width": 800,
    "height": 450,
    "samples_per_pixel": 50,
    "max_depth": 50,
    "output": "output.png"
  },
  "objects": [
    {
      "center": [0.0, 1.0, 0.0],
      "radius": 1.0,
      "material": { "Lambertian": { "albedo": [0.8, 0.2, 0.2] } }
    }
  ]
}
```

Materials use an externally tagged enum in JSON and YAML (`"Lambertian": { "albedo": [...] }`), while RON keeps its native variant syntax (`Lambertian(albedo: (...))`).

### Material variants

| Variant | Fields | Description |
|---------|--------|-------------|
| `Lambertian` | `albedo: (r, g, b)` | Matte diffuse surface |
| `Metal` | `albedo`, `fuzz` | Reflective metal with optional roughness |
| `Dielectric` | `index` | Glass / water with index of refraction |
| `Emissive` | `color`, `intensity` | Self-illuminating light source |

## Project layout

```
src/
  vec3.rs       — vectors, colors, sampling helpers
  ray.rs        — ray origin + direction
  sphere.rs     — sphere primitive
  material.rs   — BSDF-style scatter functions
  hittable.rs   — hit records and AABB tests
  bvh.rs        — bounding volume hierarchy
  camera.rs     — thin-lens perspective camera
  scene.rs      — scene loader (RON/JSON/YAML) and default demo
  lights.rs     — emissive sphere lights and direct sampling
  renderer.rs   — Monte Carlo path tracing loop
scenes/         — example scene files (demo, studio)
```

## License

MIT
