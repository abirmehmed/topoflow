//! Interactive retopology tools
//!
//! These are the artist-facing tools for manual retopology:
//! - Polystrips: Draw strips of quads
//! - Contours: Extract edge loops from sculpt
//! - Quad draw: Place individual quads
//! - Relax: Smooth vertex positions
//! - Slide: Move vertices along surface

use crate::mesh::halfedge::{HalfEdgeMesh, VertexId};
use nalgebra::Point3;

/// Draw a strip of quads along a curve on the surface
pub fn draw_polystrip(
    mesh: &mut HalfEdgeMesh,
    curve: &[Point3<f32>],
    width: f32,
) -> Result<Vec<VertexId>, String> {
    // TODO: Project curve to surface, create quad strip
    Ok(vec![])
}

/// Extract contour edges from high-poly mesh
/// Used for defining edge loops (mouth, eyes, joints)
pub fn extract_contours(
    mesh: &HalfEdgeMesh,
    angle_threshold: f32,
) -> Vec<Vec<VertexId>> {
    // TODO: Find sharp edges, trace continuous loops
    vec![]
}

/// Slide vertex along surface while preserving topology
pub fn slide_vertex(
    mesh: &mut HalfEdgeMesh,
    vertex: VertexId,
    direction: Point3<f32>,
    distance: f32,
) {
    // TODO: Move vertex along tangent plane
}
