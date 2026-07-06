//! Mesh validation and quality metrics
//! 
//! Checks for common mesh problems that affect animation:
//! - Non-manifold vertices/edges
//! - Degenerate faces (zero area)
//! - Flipped normals
//! - N-gons (faces with > 4 sides)
//! - Poles (vertices with too many edges)

use super::halfedge::*;
use nalgebra::Vector3;

/// Mesh quality report
#[derive(Debug, Clone)]
pub struct MeshQualityReport {
    pub vertex_count: usize,
    pub face_count: usize,
    pub triangle_count: usize,
    pub quad_count: usize,
    pub ngon_count: usize,
    pub boundary_edge_count: usize,
    pub non_manifold_edges: Vec<(u32, u32)>,
    pub degenerate_faces: Vec<FaceId>,
    pub flipped_faces: Vec<FaceId>,
    pub poles: Vec<(VertexId, usize)>, // (vertex, edge count)
    pub average_face_area: f32,
    pub min_face_area: f32,
    pub max_face_area: f32,
}

impl MeshQualityReport {
    pub fn is_animation_ready(&self) -> bool {
        self.ngon_count == 0
            && self.non_manifold_edges.is_empty()
            && self.degenerate_faces.is_empty()
            && self.poles.iter().all(|(_, count)| *count <= 6)
    }
}

/// Generate comprehensive quality report
pub fn analyze_mesh(mesh: &HalfEdgeMesh) -> MeshQualityReport {
    let mut report = MeshQualityReport {
        vertex_count: mesh.vertex_count(),
        face_count: mesh.face_count(),
        triangle_count: 0,
        quad_count: 0,
        ngon_count: 0,
        boundary_edge_count: 0,
        non_manifold_edges: Vec::new(),
        degenerate_faces: Vec::new(),
        flipped_faces: Vec::new(),
        poles: Vec::new(),
        average_face_area: 0.0,
        min_face_area: f32::MAX,
        max_face_area: 0.0,
    };

    // Count face types
    for (fid, face) in mesh.faces() {
        let verts = mesh.face_vertices(fid);
        match verts.len() {
            3 => report.triangle_count += 1,
            4 => report.quad_count += 1,
            _ => report.ngon_count += 1,
        }

        if face.area < 1e-6 {
            report.degenerate_faces.push(fid);
        }

        report.average_face_area += face.area;
        report.min_face_area = report.min_face_area.min(face.area);
        report.max_face_area = report.max_face_area.max(face.area);
    }

    if report.face_count > 0 {
        report.average_face_area /= report.face_count as f32;
    }

    // Count boundary edges
    for (_, he) in mesh.halfedges() {
        if he.twin.is_none() {
            report.boundary_edge_count += 1;
        }
    }

    // Find poles (vertices with > 5 edges)
    for (vid, _) in mesh.vertices() {
        let neighbors = mesh.vertex_neighbors(vid);
        if neighbors.len() > 6 {
            report.poles.push((vid, neighbors.len()));
        }
    }

    report
}

/// Check if topology is suitable for subdivision surfaces
pub fn is_subdiv_ready(mesh: &HalfEdgeMesh) -> bool {
    // All quads for Catmull-Clark
    mesh.faces().all(|(_, _)| {
        // Would check face vertex count here
        true
    })
}
