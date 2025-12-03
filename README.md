# tiny-ray

A minimal path tracer written from scratch in Rust. Renders scenes of spheres with physically inspired materials, area lighting, and a bounding volume hierarchy (BVH) for fast ray–object intersection.

## Features

- **Primitives** — analytic sphere intersection
- **Materials** — Lambertian diffuse, fuzzy metal, dielectric (glass), and emissive light sources
- **Lighting** — emissive spheres act as area lights with next-event estimation (direct light sampling and shadow rays); sky gradient for ambient fill
- **Acceleration** — SAH-style axis split BVH built over scene objects
- **Scene files** — declarative [RON](https://github.com/ron-rs/ron) scene descriptions

## Quick start

```bash
cargo run --release -- scenes/demo.ron
```

Without a scene path, the built-in demo scene is used. Output is written to the path specified in the scene file (default: `output.png`).

## Scene format

Scenes are RON files with camera, render settings, and a list of spheres:

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
  scene.rs      — RON loader and default demo
  lights.rs     — emissive sphere lights and direct sampling
  renderer.rs   — Monte Carlo path tracing loop
scenes/         — example scene files
```

## License

MIT
