# TopoFlow - How to Build and Run

## Prerequisites

### Required
- **Rust** (1.78 or newer) — [Install from rustup.rs](https://rustup.rs/)
- **Git** — For cloning the repository

### Optional (for .blend support)
- **Blender 3.0+** — Must be in your system PATH

## Quick Start (5 minutes)

### 1. Clone the Project

```bash
git clone https://github.com/yourusername/topoflow.git
cd topoflow
```

Or if you have the project files locally:
```bash
cd topoflow
```

### 2. Build the Project

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (slower compile, much faster runtime — recommended)
cargo build --release
```

The binary will be at:
- **Linux/macOS**: `./target/release/topoflow`
- **Windows**: `.\target\release\topoflow.exe`

### 3. Verify Installation

```bash
./target/release/topoflow --version
./target/release/topoflow --help
```

## Usage Examples

### Analyze a Mesh

```bash
./target/release/topoflow info mymodel.obj
```

Output shows:
- Vertex/face/edge counts
- Triangle vs quad vs n-gon distribution
- Boundary edges
- Whether the mesh is animation-ready

### Validate for Animation

```bash
./target/release/topoflow validate character.obj --detailed
```

Checks:
- ✅ No n-gons (all faces are triangles or quads)
- ✅ No non-manifold edges
- ✅ No degenerate faces
- ✅ No excessive poles (vertices with >6 edges)
- ✅ Watertight (no holes)

If issues are found, the tool tells you exactly what to fix.

### Auto-Retopology

Convert a high-poly sculpt to an animation-ready mesh:

```bash
# Basic retopology
./target/release/topoflow retopo highpoly_sculpt.obj clean_mesh.obj

# With specific target face count
./target/release/topoflow retopo sculpt.obj output.obj --faces 5000

# With specific voxel size (smaller = more detail)
./target/release/topoflow retopo sculpt.obj output.obj --voxel-size 0.05

# Preserve sharp edges (crease angle in degrees)
./target/release/topoflow retopo sculpt.obj output.obj --sharp 45.0
```

### Decimate (Reduce Polygons)

Reduce mesh complexity while preserving shape:

```bash
# Reduce to 50% of original vertices
./target/release/topoflow decimate highpoly.obj lowpoly.obj --ratio 0.5

# Reduce to specific vertex count
./target/release/topoflow decimate mesh.obj output.obj --vertices 1000

# Preserve boundary edges (important for open meshes)
./target/release/topoflow decimate mesh.obj output.obj --preserve-boundary

# Allow higher error for more reduction
./target/release/topoflow decimate mesh.obj output.obj --ratio 0.1 --max-error 5.0
```

### Using as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
topoflow = { path = "../topoflow" }
```

Example code:

```rust
use topoflow::io::ObjLoader;
use topoflow::mesh::{HalfEdgeMesh, validation};
use topoflow::algorithms::decimation::{decimate, DecimationOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load mesh
    let mesh = ObjLoader::from_file("sculpt.obj")?;

    // Check quality
    let report = validation::analyze_mesh(&mesh);
    println!("Faces: {}, Quads: {}", report.face_count, report.quad_count);

    // Decimate
    let mut mesh = mesh;
    let options = DecimationOptions {
        target_ratio: 0.5,
        preserve_boundary: true,
        ..Default::default()
    };
    decimate(&mut mesh, &options)?;

    // Export
    use topoflow::io::ObjExporter;
    ObjExporter::to_file(&mesh, "output.obj")?;

    Ok(())
}
```

## Working with .blend Files

TopoFlow can read Blender `.blend` files by launching a headless Blender process:

```bash
# Requires Blender installed and in PATH
./target/release/topoflow info myscene.blend
```

The tool:
1. Launches Blender in background mode
2. Runs a Python script to export the mesh to temporary OBJ
3. Loads the OBJ into TopoFlow's half-edge structure
4. Cleans up the temporary file

**Note**: This requires Blender 3.0+ installed. The first time you use it, it may take a few seconds to launch Blender.

## Understanding the Output

### Animation-Ready Checklist

| Check | What It Means | How to Fix |
|-------|---------------|------------|
| **No n-gons** | All faces are triangles or quads | Run `topoflow retopo` |
| **No poles >6** | No vertex has more than 6 edges | Manual cleanup or retopo |
| **No degenerates** | No zero-area faces | Run `topoflow retopo` |
| **Watertight** | No boundary edges (optional) | Fill holes in Blender |

### Euler Characteristic

- **Sphere** (closed): χ = 2
- **Torus** (donut): χ = 0
- **Disk** (open): χ = 1

If χ changes during decimation, the topology was corrupted.

## Troubleshooting

### "Blender is not installed"
Install Blender and ensure it's in your PATH, or only use `.obj` files.

### "Non-manifold edge" error
Your mesh has edges shared by more than 2 faces. Clean it in Blender first:
1. Select mesh → Edit Mode
2. Mesh → Clean Up → Merge by Distance
3. Mesh → Clean Up → Delete Loose

### Decimation removes too much detail
Increase `--max-error` or decrease `--ratio`.

### Build fails with nalgebra errors
Ensure you have Rust 1.78+:
```bash
rustc --version
```

## Project Structure (for Developers)

```
src/
├── mesh/
│   ├── halfedge.rs      # Core data structure (Vertex, Edge, Face, HalfEdgeMesh)
│   ├── topology.rs      # Edge collapse, flip, split operations
│   ├── validation.rs    # Quality analysis (poles, ngons, etc.)
│   └── attributes.rs    # Per-element data (UVs, normals, etc.)
├── io/
│   ├── obj.rs           # Wavefront OBJ loader/exporter
│   └── blend.rs         # Blender Python bridge
├── algorithms/
│   ├── decimation.rs    # QEM-based mesh simplification
│   ├── remesh.rs        # Voxel remeshing + smoothing
│   ├── quad_dominant.rs # Triangle to quad conversion
│   └── smoothing.rs     # Laplacian, Taubin, mean curvature
├── viewport/            # GPU rendering (wgpu) — future
├── ui/                  # egui interface — future
└── utils/               # Math helpers, spatial structures
```

## Next Steps

1. **Try the sample**: `cargo test` runs all unit tests
2. **Test with your meshes**: Start with `info` and `validate`
3. **Experiment**: Try different `retopo` and `decimate` settings
4. **Contribute**: Edge collapse is implemented — try adding edge split or polystrips!

## Algorithm References

- **Edge Collapse**: Garland & Heckbert (1997) — Quadric Error Metrics
- **Instant Meshes**: Jakob et al. (2015) — Field-aligned quad remeshing
- **Half-Edge Structure**: Weiler (1985) — Manifold mesh representation
