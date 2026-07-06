//! TopoFlow - High-end Retopology Tool
//!
//! A Rust-based application for converting high-poly sculpts into
//! animation-ready, quad-dominant meshes.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         TopoFlow Application            │
//! ├─────────────┬─────────────┬─────────────┤
//! │   Mesh      │     I/O     │  Algorithms │
//! │  (half-edge)│  (.obj/.blend)│ (remesh)   │
//! ├─────────────┴─────────────┴─────────────┤
//! │         Renderer (wgpu + egui)          │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use topoflow::io::ObjLoader;
//! use topoflow::mesh::HalfEdgeMesh;
//!
//! let mesh = ObjLoader::from_file("sculpt.obj").unwrap();
//! println!("Loaded mesh: {} vertices, {} faces", mesh.vertex_count(), mesh.face_count());
//! ```

pub mod mesh;
pub mod io;
pub mod algorithms;
pub mod viewport;
pub mod ui;
pub mod utils;

pub use mesh::HalfEdgeMesh;
