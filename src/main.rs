//! TopoFlow Application Entry Point
//!
//! Command-line interface for batch processing and
//! GUI launcher for interactive retopology.
//!
//! Usage:
//!   topoflow info <file>              - Analyze mesh
//!   topoflow retopo <input> <output>  - Auto-retopology
//!   topoflow validate <file>          - Check animation readiness
//!   topoflow decimate <input> <output> - Reduce polygon count
//!   topoflow gui [file]               - Launch interactive viewer

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use topoflow::io::{ObjLoader, ObjExporter};
use topoflow::mesh::validation;
use topoflow::algorithms::decimation::{decimate, DecimationOptions};
use topoflow::algorithms::remesh::{remesh_voxel, RemeshOptions};

#[derive(Parser)]
#[command(name = "topoflow")]
#[command(about = "High-end retopology tool for animation-ready meshes")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import and analyze a mesh file
    Info {
        /// Input file (.obj, .blend)
        input: PathBuf,
    },
    /// Auto-retopology: convert high-poly to animation-ready mesh
    Retopo {
        /// Input high-poly mesh
        input: PathBuf,
        /// Output file
        output: PathBuf,
        /// Target face count (auto if not specified)
        #[arg(short, long)]
        faces: Option<usize>,
        /// Target vertex count
        #[arg(short, long)]
        vertices: Option<usize>,
        /// Voxel size for remeshing
        #[arg(short, long, default_value = "0.1")]
        voxel_size: f32,
        /// Preserve sharp edges (angle threshold in degrees)
        #[arg(long, default_value = "30.0")]
        sharp: f32,
        /// Smooth iterations after retopology
        #[arg(long, default_value = "2")]
        smooth: u32,
    },
    /// Decimate mesh: reduce polygon count while preserving shape
    Decimate {
        /// Input mesh
        input: PathBuf,
        /// Output file
        output: PathBuf,
        /// Target ratio (0.5 = 50% of original vertices)
        #[arg(short, long, default_value = "0.5")]
        ratio: f32,
        /// Target vertex count (overrides ratio)
        #[arg(short, long)]
        vertices: Option<usize>,
        /// Preserve boundary edges
        #[arg(long, default_value = "true")]
        preserve_boundary: bool,
        /// Maximum error threshold
        #[arg(long, default_value = "1.0")]
        max_error: f32,
    },
    /// Validate mesh quality for animation
    Validate {
        /// Input mesh file
        input: PathBuf,
        /// Show detailed report
        #[arg(short, long)]
        detailed: bool,
    },
    /// Launch interactive GUI
    Gui {
        /// Optional file to open on startup
        input: Option<PathBuf>,
    },
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Info { input } => {
            run_info(input);
        }
        Commands::Retopo { input, output, faces, vertices, voxel_size, sharp, smooth } => {
            run_retopo(input, output, faces, vertices, voxel_size, sharp, smooth);
        }
        Commands::Decimate { input, output, ratio, vertices, preserve_boundary, max_error } => {
            run_decimate(input, output, ratio, vertices, preserve_boundary, max_error);
        }
        Commands::Validate { input, detailed } => {
            run_validate(input, detailed);
        }
        Commands::Gui { input } => {
            println!("GUI mode not yet implemented. Use CLI commands instead.");
            println!("Planned: wgpu + egui interactive viewport");
            if let Some(path) = input {
                println!("Would open: {}", path.display());
            }
        }
    }
}

fn run_info(input: PathBuf) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TopoFlow - Mesh Information                            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Loading: {}", input.display());

    let mesh = match load_mesh(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading mesh: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Mesh loaded successfully");
    println!();
    println!("┌─ Topology Statistics ───────────────────────────────────┐");
    println!("│  Vertices:     {:>8}", mesh.vertex_count());
    println!("│  Half-edges:   {:>8}", mesh.halfedge_count());
    println!("│  Faces:        {:>8}", mesh.face_count());
    println!("│  Edges:        {:>8}", mesh.halfedge_count() / 2);
    println!("│  Euler char:    {:>8}", mesh.euler_characteristic());
    println!("│  Watertight:   {:>8}", if mesh.is_watertight() { "Yes" } else { "No" });
    println!("└────────────────────────────────────────────────────────┘");

    // Quality analysis
    let report = validation::analyze_mesh(&mesh);
    println!();
    println!("┌─ Face Type Distribution ───────────────────────────────┐");
    println!("│  Triangles:    {:>8}  ({:.1}%)", 
        report.triangle_count, 
        100.0 * report.triangle_count as f32 / report.face_count.max(1) as f32);
    println!("│  Quads:        {:>8}  ({:.1}%)", 
        report.quad_count,
        100.0 * report.quad_count as f32 / report.face_count.max(1) as f32);
    println!("│  N-gons (>4):  {:>8}  ({:.1}%)", 
        report.ngon_count,
        100.0 * report.ngon_count as f32 / report.face_count.max(1) as f32);
    println!("└────────────────────────────────────────────────────────┘");

    println!();
    println!("┌─ Quality Metrics ──────────────────────────────────────┐");
    println!("│  Boundary edges:        {:>8}", report.boundary_edge_count);
    println!("│  Poles (>6 edges):      {:>8}", report.poles.len());
    println!("│  Degenerate faces:       {:>8}", report.degenerate_faces.len());
    println!("│  Average face area:      {:>10.4}", report.average_face_area);
    println!("│  Min face area:          {:>10.6}", report.min_face_area);
    println!("│  Max face area:          {:>10.4}", report.max_face_area);
    println!("└────────────────────────────────────────────────────────┘");

    println!();
    if report.is_animation_ready() {
        println!("✅ Mesh is ANIMATION-READY");
    } else {
        println!("⚠️  Mesh NOT animation-ready:");
        if report.ngon_count > 0 {
            println!("   • Contains {} n-gons (convert to quads)", report.ngon_count);
        }
        if !report.poles.is_empty() {
            println!("   • Contains {} poles (vertices with >6 edges)", report.poles.len());
        }
        if !report.degenerate_faces.is_empty() {
            println!("   • Contains {} degenerate faces", report.degenerate_faces.len());
        }
        if !report.non_manifold_edges.is_empty() {
            println!("   • Contains {} non-manifold edges", report.non_manifold_edges.len());
        }
    }
}

fn run_retopo(
    input: PathBuf,
    output: PathBuf,
    faces: Option<usize>,
    vertices: Option<usize>,
    voxel_size: f32,
    sharp: f32,
    smooth: u32,
) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TopoFlow - Auto-Retopology                             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Input:  {}", input.display());
    println!("Output: {}", output.display());
    println!("Settings:");
    println!("  • Voxel size:     {}", voxel_size);
    println!("  • Sharp threshold: {}°", sharp);
    println!("  • Smooth passes:  {}", smooth);
    if let Some(f) = faces {
        println!("  • Target faces:   {}", f);
    }
    if let Some(v) = vertices {
        println!("  • Target vertices: {}", v);
    }
    println!();

    let mut mesh = match load_mesh(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading mesh: {}", e);
            std::process::exit(1);
        }
    };

    println!("Original: {} vertices, {} faces", mesh.vertex_count(), mesh.face_count());

    // Step 1: Voxel remeshing
    println!("Step 1: Voxel remeshing...");
    let remesh_opts = RemeshOptions {
        voxel_size,
        sharp_threshold: sharp,
        smooth_iterations: smooth,
        ..Default::default()
    };

    match remesh_voxel(&mut mesh, &remesh_opts) {
        Ok(remeshed) => {
            mesh = remeshed;
            println!("  → {} vertices, {} faces", mesh.vertex_count(), mesh.face_count());
        }
        Err(e) => {
            eprintln!("Remeshing error: {}", e);
        }
    }

    // Step 2: Decimation if target specified
    if faces.is_some() || vertices.is_some() {
        println!("Step 2: Decimation...");
        let decimate_opts = DecimationOptions {
            target_vertices: vertices.unwrap_or(0),
            target_ratio: faces.map(|f| f as f32 / mesh.face_count() as f32).unwrap_or(0.5),
            preserve_boundary: true,
            max_error: 1.0,
            preserve_uv_seams: true,
        };

        match decimate(&mut mesh, &decimate_opts) {
            Ok(_) => {
                println!("  → {} vertices, {} faces", mesh.vertex_count(), mesh.face_count());
            }
            Err(e) => {
                eprintln!("Decimation error: {}", e);
            }
        }
    }

    // Step 3: Validate result
    let report = validation::analyze_mesh(&mesh);
    println!();
    if report.is_animation_ready() {
        println!("✅ Result is animation-ready");
    } else {
        println!("⚠️  Result needs manual cleanup");
    }

    // Export
    println!("Exporting to {}...", output.display());
    match ObjExporter::to_file(&mesh, &output) {
        Ok(_) => println!("✓ Export complete"),
        Err(e) => {
            eprintln!("Export error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_decimate(
    input: PathBuf,
    output: PathBuf,
    ratio: f32,
    vertices: Option<usize>,
    preserve_boundary: bool,
    max_error: f32,
) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TopoFlow - Mesh Decimation                             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Input:  {}", input.display());
    println!("Output: {}", output.display());

    let mut mesh = match load_mesh(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading mesh: {}", e);
            std::process::exit(1);
        }
    };

    let original_verts = mesh.vertex_count();
    let original_faces = mesh.face_count();

    println!("Original: {} vertices, {} faces", original_verts, original_faces);

    let target = vertices.unwrap_or((original_verts as f32 * ratio) as usize);
    println!("Target:   {} vertices ({:.1}% reduction)", 
        target, 
        100.0 * (1.0 - target as f32 / original_verts as f32));
    println!();

    let options = DecimationOptions {
        target_vertices: vertices.unwrap_or(0),
        target_ratio: ratio,
        preserve_boundary,
        max_error,
        preserve_uv_seams: true,
    };

    match decimate(&mut mesh, &options) {
        Ok(_) => {
            println!("Result:   {} vertices, {} faces", mesh.vertex_count(), mesh.face_count());
            println!("Reduced:  {:.1}% vertices, {:.1}% faces",
                100.0 * (1.0 - mesh.vertex_count() as f32 / original_verts as f32),
                100.0 * (1.0 - mesh.face_count() as f32 / original_faces as f32));

            // Validate
            let report = validation::analyze_mesh(&mesh);
            if report.is_animation_ready() {
                println!("✅ Result is animation-ready");
            }

            // Export
            match ObjExporter::to_file(&mesh, &output) {
                Ok(_) => println!("✓ Exported to {}", output.display()),
                Err(e) => eprintln!("Export error: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Decimation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_validate(input: PathBuf, detailed: bool) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TopoFlow - Mesh Validation                             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("File: {}", input.display());

    let mesh = match load_mesh(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading mesh: {}", e);
            std::process::exit(1);
        }
    };

    let report = validation::analyze_mesh(&mesh);

    println!();
    println!("┌─ Animation Readiness Check ────────────────────────────┐");

    let mut issues = Vec::new();
    let mut passes = Vec::new();

    if report.ngon_count == 0 {
        passes.push("No n-gons (all faces are triangles or quads)");
    } else {
        issues.push(format!("{} n-gons detected (should be 0)", report.ngon_count));
    }

    if report.non_manifold_edges.is_empty() {
        passes.push("No non-manifold edges");
    } else {
        issues.push(format!("{} non-manifold edges", report.non_manifold_edges.len()));
    }

    if report.degenerate_faces.is_empty() {
        passes.push("No degenerate faces");
    } else {
        issues.push(format!("{} degenerate faces", report.degenerate_faces.len()));
    }

    if report.poles.iter().all(|(_, count)| *count <= 6) {
        passes.push("No excessive poles (all vertices ≤ 6 edges)");
    } else {
        let bad_poles = report.poles.iter().filter(|(_, c)| *c > 6).count();
        issues.push(format!("{} poles with >6 edges", bad_poles));
    }

    if mesh.is_watertight() {
        passes.push("Mesh is watertight (no boundary edges)");
    } else {
        issues.push(format!("{} boundary edges (may be intentional)", report.boundary_edge_count));
    }

    for pass in &passes {
        println!("│  ✅ {}", pass);
    }
    for issue in &issues {
        println!("│  ⚠️  {}", issue);
    }

    println!("└────────────────────────────────────────────────────────┘");

    if report.is_animation_ready() {
        println!();
        println!("✅ MESH IS ANIMATION-READY");
    } else {
        println!();
        println!("⚠️  MESH NEEDS RETOPOLOGY");
        println!("   Run: topoflow retopo {} output.obj", input.display());
    }

    if detailed {
        println!();
        println!("┌─ Detailed Statistics ────────────────────────────────┐");
        println!("│  Vertices:        {:>8}", report.vertex_count);
        println!("│  Faces:           {:>8}", report.face_count);
        println!("│  Triangles:       {:>8}", report.triangle_count);
        println!("│  Quads:           {:>8}", report.quad_count);
        println!("│  N-gons:          {:>8}", report.ngon_count);
        println!("│  Boundary edges:  {:>8}", report.boundary_edge_count);
        println!("│  Euler char:      {:>8}", mesh.euler_characteristic());
        println!("│  Average area:    {:>10.4}", report.average_face_area);
        println!("└────────────────────────────────────────────────────────┘");

        if !report.poles.is_empty() {
            println!();
            println!("Pole details:");
            for (vid, count) in &report.poles {
                println!("  Vertex {}: {} edges {}", 
                    vid.0, count,
                    if *count > 6 { "⚠️" } else { "" });
            }
        }
    }
}

/// Load mesh from file (auto-detect format)
fn load_mesh(path: &PathBuf) -> Result<topoflow::mesh::HalfEdgeMesh, Box<dyn std::error::Error>> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "obj" => {
            Ok(ObjLoader::from_file(path)?)
        }
        "blend" => {
            use topoflow::io::BlendLoader;
            if !BlendLoader::is_blender_available() {
                return Err("Blender is not installed or not in PATH".into());
            }
            Ok(BlendLoader::from_file(path)?)
        }
        _ => {
            Err(format!("Unsupported file format: {}", ext).into())
        }
    }
}
