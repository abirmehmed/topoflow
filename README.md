# TopoFlow — High-End Retopology Tool

> **Convert high-poly sculpts into animation-ready, quad-dominant meshes.**

![Rust](https://img.shields.io/badge/rust-1.78%2B-orange)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)

## What is Retopology?

Retopology is the process of rebuilding a dense, high-polygon sculpt into a clean, low-poly mesh with proper edge flow. The result is optimized for:

- **Animation & Rigging** — Clean edge loops deform naturally
- **Subdivision Surfaces** — Quads subdivide predictably (Catmull-Clark)
- **UV Mapping** — Fewer seams, less distortion
- **Real-Time Performance** — Lower poly count = faster rendering

## Architecture

```
TopoFlow/
├── src/
│   ├── mesh/           # Half-edge data structure
│   │   ├── halfedge.rs # Core mesh container
│   │   ├── topology.rs # Edge collapse, split, flip
│   │   ├── validation.rs # Quality metrics
│   │   └── attributes.rs # Per-element data
│   ├── io/             # File format support
│   │   ├── obj.rs      # Wavefront OBJ loader/exporter
│   │   └── blend.rs    # Blender bridge via Python
│   ├── algorithms/     # Retopology algorithms
│   │   ├── remesh.rs   # Voxel-based remeshing
│   │   ├── decimation.rs # Quadric error metrics (QEM)
│   │   ├── quad_dominant.rs # Triangle → Quad conversion
│   │   └── smoothing.rs # Laplacian, Taubin, mean curvature
│   ├── viewport/       # GPU renderer (wgpu)
│   │   ├── renderer.rs # wgpu pipeline
│   │   ├── camera.rs   # Orbital camera
│   │   └── mesh_buffer.rs # GPU upload
│   ├── ui/             # Immediate-mode GUI (egui)
│   │   ├── panels.rs   # Toolbar, properties, outliner
│   │   └── tools.rs    # Interactive retopology tools
│   ├── utils/          # Math & spatial utilities
│   │   ├── math.rs     # Barycentric, cotangent, AABB
│   │   └── spatial.rs  # BVH, AABB tree
│   ├── lib.rs          # Library exports
│   └── main.rs         # CLI + GUI entry point
├── tests/              # Unit & integration tests
├── assets/             # Sample meshes
└── Cargo.toml          # Dependencies
```

## Algorithms

### Implemented
- **Half-Edge Mesh** — Industry-standard topology data structure
- **OBJ Loader/Exporter** — Full support for v/vn/vt/f
- **Blender Bridge** — Headless Blender Python export
- **Laplacian Smoothing** — Noise reduction
- **Taubin Smoothing** — Volume-preserving smoothing
- **Mesh Validation** — Quality reports (poles, ngons, degenerates)

### Planned
- **Instant Meshes** — Field-aligned quad remeshing (via FFI)
- **Quadric Error Metrics** — Optimal mesh decimation
- **Voxel Remeshing** — Uniform quad-dominant output
- **Polystrips** — Draw quad strips on surface
- **Contour Extraction** — Edge loop detection
- **Live Surface Snap** — Project retopo mesh to sculpt

## Quick Start

### CLI Usage

```bash
# Build the project
cargo build --release

# Analyze a mesh
./topoflow info sculpt.obj

# Auto-retopology
./topoflow retopo highpoly.obj output.obj --faces 5000 --voxel-size 0.05

# Validate animation readiness
./topoflow validate character.obj

# Launch GUI
./topoflow gui sculpt.obj
```

### Library Usage

```rust
use topoflow::io::ObjLoader;
use topoflow::mesh::{HalfEdgeMesh, validation};

// Load high-poly sculpt
let mesh = ObjLoader::from_file("sculpt.obj")?;

// Analyze quality
let report = validation::analyze_mesh(&mesh);
println!("Faces: {}, Quads: {}, N-gons: {}", 
    report.face_count, report.quad_count, report.ngon_count);

// Check if animation-ready
if report.is_animation_ready() {
    println!("✓ Mesh is ready for rigging!");
} else {
    println!("✗ Issues found: {} poles, {} ngons", 
        report.poles.len(), report.ngon_count);
}
```

## Retopology Pipeline

```
High-Poly Sculpt (.obj/.blend)
         │
         ▼
┌─────────────────────┐
│  1. Import & Clean  │  → Remove duplicates, fix normals
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  2. Analyze Quality │  → Check poles, ngons, edge flow
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  3. Auto-Retopo     │  → Voxel remesh + quad conversion
│     (or Manual)     │  → Or: polystrips, contours, quad-draw
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  4. Optimize Flow   │  → Relax vertices, snap to surface
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  5. Validate        │  → Ensure animation-ready topology
└─────────────────────┘
         │
         ▼
Animation-Ready Mesh (.obj/.blend)
```

## Topology Rules for Animation

| Rule | Why It Matters |
|------|---------------|
| **Quad-dominant** (>95% quads) | Clean subdivision, predictable deformation |
| **Edge loops** around eyes/mouth | Natural facial expression deformation |
| **Poles ≤ 6 edges** | Avoids pinching during subdivision |
| **No 3-poles on deformation zones** | Causes unwanted creasing |
| **Even valence on joints** | Smooth bending at elbows/knees |
| **No n-gons** | Break subdivision, create artifacts |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `nalgebra` | Linear algebra, vectors, matrices |
| `wgpu` | Cross-platform GPU rendering |
| `egui` | Immediate-mode GUI |
| `winit` | Window creation |
| `obj-rs` | OBJ file parsing |
| `rayon` | Parallel processing |
| `thiserror` | Error handling |

## References

- **Instant Meshes** — Jakob et al. (SIGGRAPH Asia 2015) — Field-aligned quad remeshing
- **QEM Decimation** — Garland & Heckbert (1997) — Optimal mesh simplification
- **Quad-Dominant Remeshing** — Lai, Kobbelt, Hu (2009) — Incremental feature alignment
- **Half-Edge Data Structure** — Weiler (1985) — Manifold mesh representation

## License

Dual-licensed under MIT OR Apache-2.0.
