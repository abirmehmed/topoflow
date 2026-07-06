//! Retopology algorithms module
//!
//! Core algorithms for converting high-poly sculpts to animation-ready meshes:
//! - Quad-dominant remeshing (Instant Meshes inspired)
//! - Voxel-based remeshing
//! - Decimation with topology preservation
//! - Edge flow optimization

pub mod remesh;
pub mod decimation;
pub mod quad_dominant;
pub mod smoothing;

pub use remesh::{RemeshOptions, remesh_voxel};
pub use decimation::{DecimationOptions, decimate};
