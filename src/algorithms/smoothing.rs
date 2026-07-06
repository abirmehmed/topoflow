//! Mesh smoothing and fairing algorithms
//!
//! Used for:
//! - Removing noise from scanned meshes
//! - Regularizing topology for animation
//! - Preparing meshes for retopology

use crate::mesh::halfedge::{HalfEdgeMesh, VertexId};
use nalgebra::{Point3, Vector3};

/// Umbrella operator: simple Laplacian
pub fn umbrella(mesh: &mut HalfEdgeMesh, iterations: u32, strength: f32) {
    for _ in 0..iterations {
        let mut updates = Vec::new();

        for (vid, vertex) in mesh.vertices() {
            let neighbors = mesh.vertex_neighbors(vid);
            if neighbors.len() < 2 {
                continue;
            }

            let mut centroid = Vector3::zeros();
            for nid in &neighbors {
                if let Some(v) = mesh.vertex(*nid) {
                    centroid += v.position.coords;
                }
            }
            centroid /= neighbors.len() as f32;

            let delta = centroid - vertex.position.coords;
            updates.push((vid, vertex.position + strength * delta));
        }

        for (vid, new_pos) in updates {
            if let Some(v) = mesh.vertex_mut(vid) {
                v.position = new_pos;
            }
        }
    }
}

/// Mean curvature flow: smooths while preserving features
pub fn mean_curvature_flow(mesh: &mut HalfEdgeMesh, iterations: u32, dt: f32) {
    // Uses cotangent weights for anisotropic smoothing
    // Preserves sharp features better than umbrella

    for _ in 0..iterations {
        let mut updates = Vec::new();

        for (vid, vertex) in mesh.vertices() {
            let neighbors = mesh.vertex_neighbors(vid);
            if neighbors.len() < 2 {
                continue;
            }

            // TODO: Compute cotangent weights for each neighbor
            // This requires edge lengths and angles

            let mut displacement = Vector3::zeros();
            for nid in &neighbors {
                if let Some(v) = mesh.vertex(*nid) {
                    displacement += v.position.coords - vertex.position.coords;
                }
            }
            displacement /= neighbors.len() as f32;

            updates.push((vid, vertex.position + dt * displacement));
        }

        for (vid, new_pos) in updates {
            if let Some(v) = mesh.vertex_mut(vid) {
                v.position = new_pos;
            }
        }
    }
}
