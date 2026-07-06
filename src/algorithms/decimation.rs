//! Mesh decimation (simplification) algorithm
//!
//! Reduces polygon count while preserving shape and topology.
//! Uses edge collapse with quadric error metrics (QEM) for optimal vertex placement.

use crate::mesh::halfedge::{HalfEdgeMesh, VertexId, MeshError};
use crate::mesh::topology::{collapse_edge, optimal_collapse_position};
use nalgebra::{Point3, Vector3, Matrix4};
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

/// Decimation options
#[derive(Debug, Clone)]
pub struct DecimationOptions {
    /// Target vertex count (0 = use ratio)
    pub target_vertices: usize,
    /// Target ratio of original vertices (0.5 = 50% reduction)
    pub target_ratio: f32,
    /// Preserve boundary edges
    pub preserve_boundary: bool,
    /// Maximum error threshold
    pub max_error: f32,
    /// Preserve UV seams
    pub preserve_uv_seams: bool,
}

impl Default for DecimationOptions {
    fn default() -> Self {
        Self {
            target_vertices: 0,
            target_ratio: 0.5,
            preserve_boundary: true,
            max_error: 1.0,
            preserve_uv_seams: true,
        }
    }
}

/// Quadric matrix for a plane: ax + by + cz + d = 0
#[derive(Debug, Clone)]
struct Quadric {
    matrix: Matrix4<f32>,
}

impl Quadric {
    fn zero() -> Self {
        Self { matrix: Matrix4::zeros() }
    }

    fn from_plane(normal: &Vector3<f32>, point: &Point3<f32>) -> Self {
        let a = normal.x;
        let b = normal.y;
        let c = normal.z;
        let d = -(a * point.x + b * point.y + c * point.z);

        let m = Matrix4::new(
            a*a, a*b, a*c, a*d,
            a*b, b*b, b*c, b*d,
            a*c, b*c, c*c, c*d,
            a*d, b*d, c*d, d*d,
        );

        Self { matrix: m }
    }

    fn add(&self, other: &Quadric) -> Self {
        Self {
            matrix: self.matrix + other.matrix,
        }
    }

    fn evaluate(&self, point: &Point3<f32>) -> f32 {
        let x = point.x;
        let y = point.y;
        let z = point.z;
        let m = &self.matrix;

        x*x*m[(0,0)] + 2.0*x*y*m[(0,1)] + 2.0*x*z*m[(0,2)] + 2.0*x*m[(0,3)] +
        y*y*m[(1,1)] + 2.0*y*z*m[(1,2)] + 2.0*y*m[(1,3)] +
        z*z*m[(2,2)] + 2.0*z*m[(2,3)] +
        m[(3,3)]
    }

    fn optimal_position(&self) -> Option<Point3<f32>> {
        let a = self.matrix[(0,0)];
        let b = self.matrix[(0,1)];
        let c = self.matrix[(0,2)];
        let d = self.matrix[(1,1)];
        let e = self.matrix[(1,2)];
        let f = self.matrix[(2,2)];

        let det = a*(d*f - e*e) - b*(b*f - c*e) + c*(b*e - c*d);

        if det.abs() < 1e-10 {
            return None;
        }

        let g1 = -self.matrix[(0,3)];
        let g2 = -self.matrix[(1,3)];
        let g3 = -self.matrix[(2,3)];

        let x = (g1*(d*f - e*e) - b*(g2*f - g3*e) + c*(g2*e - g3*d)) / det;
        let y = (a*(g2*f - g3*e) - g1*(b*f - c*e) + c*(b*g3 - c*g2)) / det;
        let z = (a*(d*g3 - e*g2) - b*(b*g3 - c*g2) + g1*(b*e - c*d)) / det;

        Some(Point3::new(x, y, z))
    }
}

#[derive(Debug, Clone)]
struct CollapseCandidate {
    v0: VertexId,
    v1: VertexId,
    error: f32,
    position: Point3<f32>,
}

impl PartialEq for CollapseCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.error == other.error
    }
}

impl Eq for CollapseCandidate {}

impl PartialOrd for CollapseCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.error.partial_cmp(&self.error)
    }
}

impl Ord for CollapseCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Main decimation entry point
pub fn decimate(mesh: &mut HalfEdgeMesh, options: &DecimationOptions) -> Result<(), MeshError> {
    let target = if options.target_vertices > 0 {
        options.target_vertices
    } else {
        (mesh.vertex_count() as f32 * options.target_ratio) as usize
    };

    if target >= mesh.vertex_count() {
        return Ok(());
    }

    log::info!("Decimation: {} -> {} vertices", mesh.vertex_count(), target);

    // Step 1: Compute quadrics for each face
    let mut vertex_quadrics: Vec<Quadric> = Vec::new();
    let vert_count = mesh.vertices().count();
    vertex_quadrics.resize_with(vert_count, || Quadric::zero());

    for (fid, face) in mesh.faces() {
        let verts = mesh.face_vertices(fid);
        if verts.len() < 3 {
            continue;
        }

        let p0 = mesh.vertex(verts[0]).unwrap().position;
        let p1 = mesh.vertex(verts[1]).unwrap().position;
        let p2 = mesh.vertex(verts[2]).unwrap().position;

        let normal = face.normal;
        let quadric = Quadric::from_plane(&normal, &p0);

        for v in &verts {
            let idx = v.0 as usize;
            if idx < vertex_quadrics.len() {
                let q = vertex_quadrics[idx].add(&quadric);
                vertex_quadrics[idx] = q;
            }
        }
    }

    // Step 2: Build priority queue of collapse candidates
    let mut heap: BinaryHeap<CollapseCandidate> = BinaryHeap::new();
    let edges = mesh.edges();
    let mut valid_candidates = 0;
    let mut rejected_boundary = 0;
    let mut rejected_error = 0;

    for (v0, v1) in edges {
        if options.preserve_boundary {
            let v0_is_boundary = mesh.vertex(v0).map(|v| v.is_boundary).unwrap_or(false);
            let v1_is_boundary = mesh.vertex(v1).map(|v| v.is_boundary).unwrap_or(false);
            if v0_is_boundary && v1_is_boundary {
                rejected_boundary += 1;
                continue;
            }
        }

        let q0 = &vertex_quadrics[v0.0 as usize];
        let q1 = &vertex_quadrics[v1.0 as usize];
        let combined = q0.add(q1);

        let position = combined.optimal_position()
            .unwrap_or_else(|| optimal_collapse_position(mesh, v0, v1));

        let error = combined.evaluate(&position);

        if error > options.max_error {
            rejected_error += 1;
            continue;
        }

        valid_candidates += 1;
        heap.push(CollapseCandidate {
            v0,
            v1,
            error,
            position,
        });
    }

    log::info!("Decimation candidates: {} valid, {} boundary-rejected, {} error-rejected", 
        valid_candidates, rejected_boundary, rejected_error);

    if valid_candidates == 0 {
        log::warn!("No valid collapse candidates found!");
        return Ok(());
    }

    // Step 3: Iteratively collapse edges
    let mut collapsed_count = 0;
    let mut failed_count = 0;
    let mut processed_edges: HashSet<(u32, u32)> = HashSet::new();
    let max_iterations = valid_candidates * 2; // Safety limit to prevent infinite loops
    let mut iterations = 0;

    while let Some(candidate) = heap.pop() {
        iterations += 1;
        if iterations > max_iterations {
            log::warn!("Decimation stopped: exceeded max iterations");
            break;
        }

        if mesh.vertex_count() <= target {
            break;
        }

        // Skip if vertices no longer exist
        if mesh.vertex(candidate.v0).is_none() || mesh.vertex(candidate.v1).is_none() {
            continue;
        }

        // Skip if this edge was already processed
        let edge_key = (candidate.v0.0.min(candidate.v1.0), candidate.v0.0.max(candidate.v1.0));
        if processed_edges.contains(&edge_key) {
            continue;
        }
        processed_edges.insert(edge_key);

        match collapse_edge(mesh, candidate.v0, candidate.v1, candidate.position) {
            Ok(merged_vertex) => {
                collapsed_count += 1;

                // Update quadric for merged vertex
                let q0 = &vertex_quadrics[candidate.v0.0 as usize];
                let q1 = &vertex_quadrics[candidate.v1.0 as usize];
                let merged_idx = merged_vertex.0 as usize;
                if merged_idx < vertex_quadrics.len() {
                    vertex_quadrics[merged_idx] = q0.add(q1);
                }

                // Add new candidates for edges connected to merged vertex
                // But limit how many we add to prevent heap explosion
                let neighbors = mesh.vertex_neighbors(merged_vertex);
                for &neighbor in &neighbors {
                    let nv0 = merged_vertex;
                    let nv1 = neighbor;

                    if mesh.vertex(nv0).is_none() || mesh.vertex(nv1).is_none() {
                        continue;
                    }

                    let new_edge_key = (nv0.0.min(nv1.0), nv0.0.max(nv1.0));
                    if processed_edges.contains(&new_edge_key) {
                        continue;
                    }

                    let nq0 = &vertex_quadrics[nv0.0 as usize];
                    let nq1 = &vertex_quadrics[nv1.0 as usize];
                    let n_combined = nq0.add(nq1);
                    let n_pos = n_combined.optimal_position()
                        .unwrap_or_else(|| optimal_collapse_position(mesh, nv0, nv1));
                    let n_error = n_combined.evaluate(&n_pos);

                    if n_error <= options.max_error {
                        heap.push(CollapseCandidate {
                            v0: nv0,
                            v1: nv1,
                            error: n_error,
                            position: n_pos,
                        });
                    }
                }
            }
            Err(e) => {
                failed_count += 1;
                log::debug!("Collapse failed: {}", e);
                continue;
            }
        }
    }

    log::info!("Decimation complete: {} collapsed, {} failed, {} vertices remaining", 
        collapsed_count, failed_count, mesh.vertex_count());

    Ok(())
}
