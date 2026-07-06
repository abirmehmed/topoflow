//! Topology operations for retopology

use super::halfedge::*;
use nalgebra::{Point3, Vector3};
use std::collections::HashSet;

pub type TopologyResult<T> = Result<T, TopologyError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopologyError {
    #[error("Edge collapse would create non-manifold topology")]
    NonManifoldCollapse,
    #[error("Edge flip not possible (not shared by two triangles)")]
    InvalidFlip,
    #[error("Operation would create degenerate face")]
    DegenerateFace,
    #[error("Boundary edge operation not allowed")]
    BoundaryEdge,
    #[error("Invalid edge: {0}")]
    InvalidEdge(String),
    #[error("Invalid topology: {0}")]
    InvalidTopology(String),
    #[error("{0}")]
    MeshError(#[from] MeshError),
}

/// Collapse an edge by merging its two vertices.
pub fn collapse_edge(
    mesh: &mut HalfEdgeMesh,
    v0: VertexId,
    v1: VertexId,
    target_position: Point3<f32>,
) -> TopologyResult<VertexId> {
    let (he_01, he_10) = find_edge_halfedges(mesh, v0, v1)
        .ok_or_else(|| TopologyError::InvalidEdge(
            format!("No edge between vertices {} and {}", v0.0, v1.0)
        ))?;

    // Check if this is a boundary edge
    let is_boundary = mesh.halfedge(he_01).map(|h| h.twin.is_none()).unwrap_or(false)
        || mesh.halfedge(he_10).map(|h| h.twin.is_none()).unwrap_or(false);

    // Get the faces adjacent to this edge
    let face_01 = mesh.halfedge(he_01).unwrap().face;
    let face_10 = mesh.halfedge(he_10).unwrap().face;

    // Link condition check: count shared faces between v0 and v1
    // They should share exactly 2 faces (the ones on either side of the edge)
    let v0_faces: HashSet<u32> = mesh.vertex_faces(v0).iter().map(|f| f.0).collect();
    let v1_faces: HashSet<u32> = mesh.vertex_faces(v1).iter().map(|f| f.0).collect();
    let shared_faces: Vec<FaceId> = v0_faces.intersection(&v1_faces)
        .map(|&f| FaceId(f))
        .collect();

    // For interior edge: should share exactly 2 faces
    // For boundary edge: should share exactly 1 face
    let expected_shared = if is_boundary { 1 } else { 2 };
    if shared_faces.len() != expected_shared {
        return Err(TopologyError::NonManifoldCollapse);
    }

    // Check for face inversion
    if would_cause_inversion(mesh, v0, v1, target_position) {
        return Err(TopologyError::NonManifoldCollapse);
    }

    // Collect all half-edges originating from v1
    let v1_outgoing: Vec<HalfEdgeId> = collect_outgoing_halfedges(mesh, v1);

    // Update half-edge origins from v1 to v0
    for &heid in &v1_outgoing {
        if heid == he_10 {
            continue;
        }
        mesh.set_halfedge_origin(heid, v0);
    }

    // Patch faces to bypass the collapsed edge
    if let Some(f01) = face_01 {
        patch_face_after_collapse(mesh, f01, he_01)?;
    }
    if let Some(f10) = face_10 {
        patch_face_after_collapse(mesh, f10, he_10)?;
    }

    // Update twin relationships
    let he_01_next = mesh.halfedge(he_01).unwrap().next;
    let he_10_next = mesh.halfedge(he_10).unwrap().next;
    let he_01_prev = mesh.halfedge(he_01).unwrap().prev;
    let he_10_prev = mesh.halfedge(he_10).unwrap().prev;

    if let Some(twin_prev) = mesh.halfedge(he_01_prev).and_then(|h| h.twin) {
        mesh.set_halfedge_twin(twin_prev, Some(he_10_next));
        mesh.set_halfedge_twin(he_10_next, Some(twin_prev));
    }
    if let Some(twin_prev) = mesh.halfedge(he_10_prev).and_then(|h| h.twin) {
        mesh.set_halfedge_twin(twin_prev, Some(he_01_next));
        mesh.set_halfedge_twin(he_01_next, Some(twin_prev));
    }

    // Update vertex halfedge reference
    let new_v0_he = if mesh.halfedge(he_01_next).is_some() {
        he_01_next
    } else {
        he_10_next
    };
    mesh.set_vertex_halfedge(v0, Some(new_v0_he));

    // Move v0 to target position
    mesh.set_vertex_position(v0, target_position);

    // Remove deleted elements
    if let Some(f01) = face_01 {
        mesh.remove_face(f01);
    }
    if let Some(f10) = face_10 {
        mesh.remove_face(f10);
    }

    mesh.remove_halfedge(he_01);
    mesh.remove_halfedge(he_10);
    mesh.remove_vertex(v1);

    mesh.update_topology();

    Ok(v0)
}

fn find_edge_halfedges(
    mesh: &HalfEdgeMesh,
    v0: VertexId,
    v1: VertexId,
) -> Option<(HalfEdgeId, HalfEdgeId)> {
    let start_he = mesh.vertex(v0)?.halfedge?;
    let mut curr = start_he;

    loop {
        let he = mesh.halfedge(curr)?;
        let next_he = mesh.halfedge(he.next)?;
        if next_he.origin == v1 {
            let he_01 = curr;
            let he_10 = he.twin?;
            return Some((he_01, he_10));
        }

        if let Some(twin) = he.twin {
            curr = mesh.halfedge(twin)?.next;
        } else {
            break;
        }

        if curr == start_he {
            break;
        }
    }

    None
}

fn would_cause_inversion(
    mesh: &HalfEdgeMesh,
    v0: VertexId,
    v1: VertexId,
    target: Point3<f32>,
) -> bool {
    let mut affected_faces = Vec::new();

    for fid in mesh.vertex_faces(v0) {
        let verts = mesh.face_vertices(fid);
        if !verts.contains(&v1) {
            affected_faces.push(fid);
        }
    }

    for fid in mesh.vertex_faces(v1) {
        let verts = mesh.face_vertices(fid);
        if !verts.contains(&v0) && !affected_faces.contains(&fid) {
            affected_faces.push(fid);
        }
    }

    for fid in affected_faces {
        let verts = mesh.face_vertices(fid);
        if verts.len() < 3 {
            continue;
        }

        let p0 = mesh.vertex(verts[0]).unwrap().position;
        let p1 = mesh.vertex(verts[1]).unwrap().position;
        let p2 = mesh.vertex(verts[2]).unwrap().position;
        let original_normal = (p1 - p0).cross(&(p2 - p0));

        if original_normal.magnitude() < 1e-10 {
            continue;
        }

        let new_positions: Vec<Point3<f32>> = verts.iter()
            .map(|&v| {
                if v == v0 || v == v1 {
                    target
                } else {
                    mesh.vertex(v).unwrap().position
                }
            })
            .collect();

        if new_positions.len() >= 3 {
            let new_normal = (new_positions[1] - new_positions[0])
                .cross(&(new_positions[2] - new_positions[0]));

            if original_normal.dot(&new_normal) < 0.0 {
                return true;
            }
        }
    }

    false
}

fn collect_outgoing_halfedges(mesh: &HalfEdgeMesh, vid: VertexId) -> Vec<HalfEdgeId> {
    let mut result = Vec::new();
    let start_he = match mesh.vertex(vid).and_then(|v| v.halfedge) {
        Some(he) => he,
        None => return result,
    };

    let mut curr = start_he;
    loop {
        result.push(curr);

        let he = match mesh.halfedge(curr) {
            Some(h) => h,
            None => break,
        };

        if let Some(twin) = he.twin {
            curr = match mesh.halfedge(twin) {
                Some(t) => t.next,
                None => break,
            };
        } else {
            break;
        }

        if curr == start_he {
            break;
        }
    }

    result
}

fn patch_face_after_collapse(
    mesh: &mut HalfEdgeMesh,
    fid: FaceId,
    collapsed_he: HalfEdgeId,
) -> TopologyResult<()> {
    let he = mesh.halfedge(collapsed_he)
        .ok_or_else(|| TopologyError::InvalidEdge("Collapsed half-edge not found".to_string()))?;

    let prev_he = he.prev;
    let next_he = he.next;

    mesh.set_halfedge_next(prev_he, next_he);
    mesh.set_halfedge_prev(next_he, prev_he);
    mesh.set_face_halfedge(fid, next_he);

    Ok(())
}

pub fn split_edge(
    _mesh: &mut HalfEdgeMesh,
    _v0: VertexId,
    _v1: VertexId,
    _position: Point3<f32>,
) -> TopologyResult<VertexId> {
    unimplemented!("Edge split")
}

pub fn flip_edge(mesh: &mut HalfEdgeMesh, v0: VertexId, v1: VertexId) -> TopologyResult<()> {
    let (he_01, he_10) = find_edge_halfedges(mesh, v0, v1)
        .ok_or_else(|| TopologyError::InvalidEdge("Edge not found".to_string()))?;

    let he_01_face = mesh.halfedge(he_01).unwrap().face;
    let he_10_face = mesh.halfedge(he_10).unwrap().face;

    if he_01_face.is_none() || he_10_face.is_none() {
        return Err(TopologyError::BoundaryEdge);
    }

    let he_01_next = mesh.halfedge(he_01).unwrap().next;
    let he_10_next = mesh.halfedge(he_10).unwrap().next;
    let v2 = mesh.halfedge(he_01_next).unwrap().origin;
    let v3 = mesh.halfedge(he_10_next).unwrap().origin;

    let f0_verts = mesh.face_vertices(he_01_face.unwrap());
    let f1_verts = mesh.face_vertices(he_10_face.unwrap());
    if f0_verts.len() != 3 || f1_verts.len() != 3 {
        return Err(TopologyError::InvalidFlip);
    }

    let he_01_prev = mesh.halfedge(he_01).unwrap().prev;
    let he_10_prev = mesh.halfedge(he_10).unwrap().prev;
    let he_12 = he_01_next;
    let he_23 = mesh.halfedge(he_12).unwrap().next;
    let he_30 = he_10_next;
    let he_03 = mesh.halfedge(he_30).unwrap().next;

    if mesh.halfedge(he_23).unwrap().next != he_01 {
        return Err(TopologyError::InvalidTopology("Unexpected face structure".to_string()));
    }
    if mesh.halfedge(he_03).unwrap().next != he_10 {
        return Err(TopologyError::InvalidTopology("Unexpected face structure".to_string()));
    }

    mesh.set_halfedge_next(he_01, he_30);
    mesh.set_halfedge_prev(he_30, he_01);
    mesh.set_halfedge_next(he_30, he_23);
    mesh.set_halfedge_prev(he_23, he_30);
    mesh.set_halfedge_next(he_23, he_01);
    mesh.set_halfedge_prev(he_01, he_23);
    mesh.set_halfedge_face(he_01, he_01_face);
    mesh.set_halfedge_face(he_30, he_01_face);
    mesh.set_halfedge_face(he_23, he_01_face);
    mesh.set_face_halfedge(he_01_face.unwrap(), he_01);

    mesh.set_halfedge_next(he_10, he_03);
    mesh.set_halfedge_prev(he_03, he_10);
    mesh.set_halfedge_next(he_03, he_12);
    mesh.set_halfedge_prev(he_12, he_03);
    mesh.set_halfedge_next(he_12, he_10);
    mesh.set_halfedge_prev(he_10, he_12);
    mesh.set_halfedge_face(he_10, he_10_face);
    mesh.set_halfedge_face(he_03, he_10_face);
    mesh.set_halfedge_face(he_12, he_10_face);
    mesh.set_face_halfedge(he_10_face.unwrap(), he_10);

    mesh.set_halfedge_origin(he_01, v2);
    mesh.set_halfedge_origin(he_10, v3);

    if mesh.vertex(v0).unwrap().halfedge == Some(he_01) {
        mesh.set_vertex_halfedge(v0, Some(he_03));
    }
    if mesh.vertex(v1).unwrap().halfedge == Some(he_10) {
        mesh.set_vertex_halfedge(v1, Some(he_12));
    }

    mesh.update_topology();

    Ok(())
}

pub fn quad_quality(p0: &Point3<f32>, p1: &Point3<f32>, p2: &Point3<f32>, p3: &Point3<f32>) -> f32 {
    let e0 = p1 - p0;
    let e1 = p2 - p1;
    let e2 = p3 - p2;
    let e3 = p0 - p3;

    let normal = e0.cross(&e1).normalize();
    let normal2 = e2.cross(&e3).normalize();
    let planarity = normal.dot(&normal2).abs();

    let angles = [
        e0.dot(&e3).abs() / (e0.magnitude() * e3.magnitude()),
        e0.dot(&e1).abs() / (e0.magnitude() * e1.magnitude()),
        e1.dot(&e2).abs() / (e1.magnitude() * e2.magnitude()),
        e2.dot(&e3).abs() / (e2.magnitude() * e3.magnitude()),
    ];

    let angle_quality = angles.iter().map(|a| 1.0 - a).sum::<f32>() / 4.0;

    planarity * angle_quality
}

pub fn is_valid_quad(mesh: &HalfEdgeMesh, face: FaceId) -> bool {
    let verts = mesh.face_vertices(face);
    verts.len() == 4
}

pub fn optimal_collapse_position(
    mesh: &HalfEdgeMesh,
    v0: VertexId,
    v1: VertexId,
) -> Point3<f32> {
    let p0 = mesh.vertex(v0).unwrap().position;
    let p1 = mesh.vertex(v1).unwrap().position;
    Point3::from((p0.coords + p1.coords) * 0.5)
}
