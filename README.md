# tiny-ray

A minimal path tracer written from scratch in Rust. Renders scenes of spheres with physically inspired materials, area lighting, and a bounding volume hierarchy (BVH) for fast ray–object intersection.

## Features

- **Primitives** — analytic sphere intersection and infinite planes
- **Materials** — Lambertian diffuse, fuzzy metal, dielectric (glass), and emissive light sources; metal surfaces use specular next-event estimation for direct highlights
- **Lighting** — emissive spheres act as area lights with next-event estimation (direct diffuse and specular sampling plus shadow rays); configurable vertical sky gradient for ambient fill
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
| `--format FMT` | Force scene parser: `ron`, `json`, or `yaml` (default: from extension) |
| `--gamma MODE` | Output encoding: `gamma2` (default), `srgb`, or `linear` |
| `--exposure F` | Linear exposure multiplier applied before gamma encoding |
| `--aa MODE` | Anti-aliasing: `random` (default), `stratified`, or `halton` |
| `--filter MODE` | Pixel reconstruction filter: `box` (default), `gaussian`, or `mitchell` |
| `-h`, `--help` | Print usage |

## Example render: starter demo

The `scenes/demo.*` files reproduce the classic “Ray Tracing in One Weekend” layout: three large spheres on a matte gray ground — red and green diffuse balls on the sides, a silver metal sphere on the right, and a glass dielectric orb toward the back. A bright emissive sphere overhead provides area lighting; depth of field comes from a thin-lens camera (`aperture: 0.1`). This is the scene loaded when you run `cargo run --release` with no arguments.

```bash
cargo run --release -- scenes/demo.ron
# writes output.png (800×450, 50 spp)

# same layout in JSON or YAML
cargo run --release -- scenes/demo.json
cargo run --release -- scenes/demo.yaml

# quick preview while adjusting camera or materials
cargo run --release -- --samples 8 --output demo-preview.png scenes/demo.ron
```

Scene excerpt (RON):

```ron
(
    camera: (
        lookfrom: (13.0, 2.0, 3.0),
        lookat: (0.0, 1.0, 0.0),
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
        (
            center: (4.0, 1.0, 0.0),
            radius: 1.0,
            material: Metal(albedo: (0.8, 0.8, 0.9), fuzz: 0.05),
        ),
        (
            center: (0.0, 1.0, -4.0),
            radius: 1.0,
            material: Dielectric(index: 1.5),
        ),
        (
            center: (0.0, 8.0, 0.0),
            radius: 3.0,
            material: Emissive(color: (1.0, 0.95, 0.8), intensity: 4.0),
        ),
        // ... green diffuse sphere and ground plane
    ],
)
```

The same scene is available as `scenes/demo.json` and `scenes/demo.yaml`.

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

## Example render: Cornell box

The `scenes/cornell.*` files build a classic Cornell box from five infinite planes — white floor, ceiling, and back wall, a red left wall, and a green right wall — instead of the giant-sphere room hack used in `studio.*`. Three spheres on the floor showcase glass, metal, and diffuse materials; a small emissive sphere in the ceiling provides area lighting.

```bash
cargo run --release -- scenes/cornell.ron
# writes cornell.png (800×450, 100 spp)

cargo run --release -- --samples 16 --output cornell-preview.png scenes/cornell.json
```

Scene excerpt (RON):

```ron
(
    camera: (
        lookfrom: (0.0, 1.4, 3.8),
        lookat: (0.0, 1.0, 0.0),
        vfov: 40.0,
    ),
    render: (
        width: 800,
        height: 450,
        samples_per_pixel: 100,
        max_depth: 50,
        output: "cornell.png",
        gamma: srgb,
        aa: stratified,
    ),
    objects: [
        (
            center: (0.0, 0.5, 0.0),
            radius: 0.5,
            material: Dielectric(index: 1.5),
        ),
        // ... ceiling light, metal and diffuse spheres
    ],
    planes: [
        (
            point: (0.0, 0.0, 0.0),
            normal: (0.0, 1.0, 0.0),
            material: Lambertian(albedo: (0.73, 0.73, 0.73)),
        ),
        (
            point: (-1.4, 0.0, 0.0),
            normal: (1.0, 0.0, 0.0),
            material: Lambertian(albedo: (0.65, 0.05, 0.05)),
        ),
        // ... ceiling, back wall, and right wall
    ],
)
```

Spheres go in `objects`; room walls and floors go in the optional `planes` array. Existing sphere-only scene files are unchanged. The same scene is available as `scenes/cornell.json` and `scenes/cornell.yaml`.

## Example render: sunset patio

The `scenes/sunset.*` files set up an open outdoor scene with a warm horizon-to-zenith sky gradient, a sand-colored ground plane, and three small spheres on the floor — a glass dielectric orb in the center, a gold metal sphere on the left, and a terracotta diffuse sphere on the right. A distant emissive sphere low on the horizon acts as the setting sun; rays that miss geometry pick up the custom sky colors instead of the default pale blue gradient. Depth of field comes from a thin-lens camera (`aperture: 0.03`).

```bash
cargo run --release -- scenes/sunset.ron
# writes sunset.png (800×450, 100 spp)

# quick preview while tuning sky colors or sun position
cargo run --release -- --samples 16 --output sunset-preview.png scenes/sunset.json
```

Scene excerpt (RON):

```ron
(
    camera: (
        lookfrom: (0.0, 1.1, 5.5),
        lookat: (0.0, 0.35, 0.0),
        vfov: 38.0,
        aperture: 0.03,
        focus_distance: 5.5,
    ),
    render: (
        width: 800,
        height: 450,
        samples_per_pixel: 100,
        max_depth: 50,
        output: "sunset.png",
        gamma: srgb,
        aa: stratified,
    ),
    sky: (
        horizon: (1.0, 0.55, 0.32),
        zenith: (0.12, 0.22, 0.55),
    ),
    planes: [
        (
            point: (0.0, 0.0, 0.0),
            normal: (0.0, 1.0, 0.0),
            material: Lambertian(albedo: (0.78, 0.72, 0.62)),
        ),
    ],
    objects: [
        (
            center: (10.0, 4.0, -18.0),
            radius: 2.5,
            material: Emissive(color: (1.0, 0.72, 0.45), intensity: 6.0),
        ),
        (
            center: (0.0, 0.45, 0.0),
            radius: 0.45,
            material: Dielectric(index: 1.5),
        ),
        // ... metal and diffuse spheres
    ],
)
```

The optional `sky` block sets `horizon` and `zenith` RGB colors for the background gradient; omit it to keep the default white-to-blue sky used by `demo.*` and other indoor scenes. The same scene is available as `scenes/sunset.json` and `scenes/sunset.yaml`.

### Modular scenes with `include`

Split large JSON/YAML scenes into reusable fragments and wire them together with an `include` array. Paths are resolved relative to the file that lists them; nested includes and mixed formats (for example, a JSON root including YAML fragments) are supported.

```yaml
include:
  - fragments/cornell-walls.yaml
  - fragments/cornell-objects.yaml
camera:
  lookfrom: [0.0, 1.4, 3.8]
  lookat: [0.0, 1.0, 0.0]
  # ...
render:
  width: 800
  height: 450
  output: cornell.png
```

Fragment files only need geometry (`objects`, `planes`, and optional nested `include` entries). The root scene supplies `camera` and `render`. See `scenes/cornell-modular.yaml` and `scenes/fragments/` for a working example equivalent to `scenes/cornell.yaml`.

After parsing, the loader validates scene semantics (positive radii, non-zero plane normals, sane render dimensions, and more) and reports clear errors before building the BVH.

## Scene format

Scenes are loaded by file extension (`.ron`, `.json`, `.yaml`/`.yml`), with content sniffing as a fallback for extensionless files. Override the parser with `--format` when needed. Each file describes the camera, render settings, an optional sky gradient, a list of spheres (`objects`), an optional list of planes (`planes`), and an optional list of fragment paths (`include`). The same schema works across all three formats.

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

### Render settings

| Field | Default | Description |
|-------|---------|-------------|
| `width`, `height` | — | Image resolution in pixels |
| `samples_per_pixel` | — | Monte Carlo samples per pixel (anti-aliasing + noise reduction) |
| `max_depth` | — | Maximum path bounces |
| `output` | — | Output PNG path |
| `gamma` | `gamma2` | Output encoding: `gamma2`, `srgb`, or `linear` |
| `exposure` | `1.0` | Linear exposure multiplier before gamma encoding |
| `aa` | `random` | Sub-pixel sampling: `random`, `stratified`, or `halton` |
| `filter` | `box` | Pixel reconstruction filter: `box`, `gaussian`, or `mitchell` |

Tracing accumulates linear radiance; sub-pixel samples are weighted by the chosen reconstruction filter before averaging. The color pipeline applies exposure and gamma when writing 8-bit PNG pixels. The studio scene uses sRGB output with stratified anti-aliasing and a Mitchell filter for sharper edge reconstruction. For faster convergence at low sample counts, try Halton quasi-random offsets (`--aa halton`).

```ron
render: (
    width: 800,
    height: 450,
    samples_per_pixel: 100,
    max_depth: 50,
    output: "studio.png",
    gamma: srgb,
    exposure: 1.0,
    aa: stratified,
    filter: mitchell,
),
```

### Material variants

| Variant | Fields | Description |
|---------|--------|-------------|
| `Lambertian` | `albedo: (r, g, b)` | Matte diffuse surface |
| `Metal` | `albedo`, `fuzz` | Reflective metal with optional roughness; direct specular highlights from emissive lights |
| `Dielectric` | `index` | Glass / water with index of refraction |
| `Emissive` | `color`, `intensity` | Self-illuminating light source |

### Object types

| Type | Array | Fields | Description |
|------|-------|--------|-------------|
| Sphere | `objects` | `center`, `radius`, `material` | Analytic sphere primitive |
| Plane | `planes` | `point`, `normal`, `material` | Infinite plane through `point` with outward `normal` |

The `planes` array is optional and defaults to empty, so existing sphere-only scenes load unchanged.

### Sky gradient

| Field | Default | Description |
|-------|---------|-------------|
| `horizon` | `(1, 1, 1)` | Background color at the horizon (ray direction y = 0) |
| `zenith` | `(0.5, 0.7, 1)` | Background color at the zenith (ray direction y = 1) |

Linear interpolation between horizon and zenith is applied based on the y component of the miss ray direction. Indoor scenes can omit the block entirely.

```ron
sky: (
    horizon: (1.0, 0.55, 0.32),
    zenith: (0.12, 0.22, 0.55),
),
```

## Project layout

```
src/
  vec3.rs       — vectors, colors, sampling helpers
  ray.rs        — ray origin + direction
  sphere.rs     — sphere primitive
  plane.rs      — infinite plane primitive
  material.rs   — BSDF-style scatter functions
  hittable.rs   — hit records and AABB tests
  bvh.rs        — bounding volume hierarchy
  camera.rs     — thin-lens perspective camera
  color.rs      — gamma correction, exposure, and pixel encoding
  sampling.rs   — anti-aliasing sample strategies (random, stratified, halton)
  film.rs       — pixel reconstruction filters (box, Gaussian, Mitchell)
  sky.rs        — configurable vertical sky gradient for miss rays
  scene/
    format.rs   — serde schema (SceneFile, descriptors)
    loader.rs   — format detection, includes, and parsing
    validate.rs — semantic validation after load
    mod.rs      — runtime Scene type and world construction
  lights.rs     — emissive sphere lights and direct sampling
  renderer.rs   — Monte Carlo path tracing loop
scenes/         — example scene files (demo, studio, cornell, sunset, modular cornell)
scenes/fragments/ — reusable geometry fragments for include-based scenes
```

## License

MIT
