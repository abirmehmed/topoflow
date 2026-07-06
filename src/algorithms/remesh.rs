//! Voxel-based remeshing algorithm
//!
//! Converts any mesh to a uniform, quad-dominant mesh by:
//! 1. Voxelizing the mesh surface
//! 2. Extracting isosurface (Dual Contouring or Marching Cubes)
//! 3. Post-processing for quad quality
//!
//! Inspired by Blender's Remesh modifier and ZBrush's DynaMesh

use crate::mesh::halfedge::{HalfEdgeMesh, VertexId, FaceId, MeshError};
use nalgebra::{Point3, Vector3};

/// Remeshing options
#[derive(Debug, Clone)]
pub struct RemeshOptions {
    /// Target voxel size (world units)
    pub voxel_size: f32,
    /// Adaptivity: 0 = uniform, 1 = high detail in curved areas
    pub adaptivity: f32,
    /// Preserve sharp edges (crease angle in degrees)
    pub sharp_threshold: f32,
    /// Smooth iterations after remeshing
    pub smooth_iterations: u32,
    /// Target: quads vs triangles
    pub quad_dominant: bool,
}

impl Default for RemeshOptions {
    fn default() -> Self {
        Self {
            voxel_size: 0.1,
            adaptivity: 0.0,
            sharp_threshold: 30.0,
            smooth_iterations: 2,
            quad_dominant: true,
        }
    }
}

/// Voxel remeshing entry point
pub fn remesh_voxel(mesh: &HalfEdgeMesh, options: &RemeshOptions) -> Result<HalfEdgeMesh, MeshError> {
    // TODO: Implement voxelization + dual contouring
    // This is a placeholder that returns a simplified version

    // 1. Compute bounding box
    // 2. Create voxel grid
    // 3. Mark voxels intersecting mesh surface
    // 4. Extract surface using Dual Contouring (produces quads)
    // 5. Post-process: smooth, cleanup

    // For now, return a copy with basic smoothing
    let mut result = mesh.clone();
    laplacian_smooth(&mut result, options.smooth_iterations);
    Ok(result)
}

/// Laplacian smoothing: move each vertex to average of neighbors
/// Used for noise reduction and regularization
pub fn laplacian_smooth(mesh: &mut HalfEdgeMesh, iterations: u32) {
    for _ in 0..iterations {
        let mut new_positions: Vec<(VertexId, Point3<f32>)> = Vec::new();

        for (vid, _) in mesh.vertices() {
            let neighbors = mesh.vertex_neighbors(vid);
            if neighbors.is_empty() {
                continue;
            }

            let mut avg = Vector3::zeros();
            for nid in &neighbors {
                if let Some(v) = mesh.vertex(*nid) {
                    avg += v.position.coords;
                }
            }
            avg /= neighbors.len() as f32;

            new_positions.push((vid, Point3::from(avg)));
        }

        for (vid, pos) in new_positions {
            if let Some(v) = mesh.vertex_mut(vid) {
                v.position = pos;
            }
        }
    }
}

/// Taubin smoothing: preserves volume better than Laplacian
/// λ pass (shrink) followed by μ pass (expand)
pub fn taubin_smooth(mesh: &mut HalfEdgeMesh, iterations: u32, lambda: f32, mu: f32) {
    for _ in 0..iterations {
        // Shrink pass
        smooth_pass(mesh, lambda);
        // Expand pass
        smooth_pass(mesh, mu);
    }
}

fn smooth_pass(mesh: &mut HalfEdgeMesh, factor: f32) {
    let mut displacements: Vec<(VertexId, Vector3<f32>)> = Vec::new();

    for (vid, vertex) in mesh.vertices() {
        let neighbors = mesh.vertex_neighbors(vid);
        if neighbors.len() < 2 {
            continue;
        }

        let mut avg = Vector3::zeros();
        for nid in &neighbors {
            if let Some(v) = mesh.vertex(*nid) {
                avg += v.position.coords;
            }
        }
        avg /= neighbors.len() as f32;

        let displacement = factor * (avg - vertex.position.coords);
        displacements.push((vid, displacement));
    }

    for (vid, disp) in displacements {
        if let Some(v) = mesh.vertex_mut(vid) {
            v.position += disp;
        }
    }
}
