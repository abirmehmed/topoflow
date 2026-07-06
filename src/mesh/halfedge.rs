//! Half-Edge Mesh Data Structure
//! 
//! The half-edge (or doubly-connected edge list) is the industry standard
//! for mesh topology manipulation. Each edge is split into two directed "half-edges"
//! that point to each other (twin), forming a circular linked list around each face.
//!
//! Memory layout optimized for cache locality:
//! - All vertices stored in contiguous Vec
//! - All half-edges stored in contiguous Vec  
//! - All faces stored in contiguous Vec
//! - IDs are array indices (u32 for compactness)

use nalgebra::{Vector3, Point3};
use std::collections::HashSet;

/// Compact ID types using array indices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HalfEdgeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u32);  // Logical edge = pair of half-edges

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceId(pub u32);

/// Invalid/Null ID sentinel
pub const INVALID_ID: u32 = u32::MAX;

/// A vertex in the mesh
#[derive(Debug, Clone)]
pub struct Vertex {
    /// Position in 3D space
    pub position: Point3<f32>,
    /// One outgoing half-edge (arbitrary choice)
    pub halfedge: Option<HalfEdgeId>,
    /// Normal (computed or loaded)
    pub normal: Option<Vector3<f32>>,
    /// UV coordinate
    pub uv: Option<[f32; 2]>,
    /// Is this vertex on a boundary?
    pub is_boundary: bool,
    /// User-defined flags
    pub flags: u32,
}

/// A half-edge (directed edge)
#[derive(Debug, Clone)]
pub struct HalfEdge {
    /// Origin vertex
    pub origin: VertexId,
    /// Twin half-edge (opposite direction)
    pub twin: Option<HalfEdgeId>,
    /// Next half-edge around face (CCW)
    pub next: HalfEdgeId,
    /// Previous half-edge around face
    pub prev: HalfEdgeId,
    /// Incident face (None = boundary)
    pub face: Option<FaceId>,
    /// Is this edge sharp/creased?
    pub is_sharp: bool,
}

/// A face (polygon, typically quad or triangle)
#[derive(Debug, Clone)]
pub struct Face {
    /// One half-edge on this face
    pub halfedge: HalfEdgeId,
    /// Normal vector
    pub normal: Vector3<f32>,
    /// Face area (cached)
    pub area: f32,
    /// Material ID
    pub material: u32,
}

/// The main mesh container
/// 
/// Invariants maintained:
/// - Every half-edge has a valid origin vertex
/// - Every half-edge has valid next/prev pointers
/// - Twin relationships are symmetric
/// - Boundary edges have no face
/// - Manifold: each edge has at most 2 incident faces
#[derive(Debug, Clone)]
pub struct HalfEdgeMesh {
    vertices: Vec<Vertex>,
    halfedges: Vec<HalfEdge>,
    faces: Vec<Face>,
    /// Next available ID (for stable IDs after deletion)
    next_vertex_id: u32,
    next_halfedge_id: u32,
    next_face_id: u32,
    /// Deleted slots (for recycling)
    free_vertices: Vec<u32>,
    free_halfedges: Vec<u32>,
    free_faces: Vec<u32>,
    /// Bounding box (cached, invalidated on vertex move)
    bbox_min: Point3<f32>,
    bbox_max: Point3<f32>,
    bbox_valid: bool,
}

impl HalfEdgeMesh {
    /// Create empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            halfedges: Vec::new(),
            faces: Vec::new(),
            next_vertex_id: 0,
            next_halfedge_id: 0,
            next_face_id: 0,
            free_vertices: Vec::new(),
            free_halfedges: Vec::new(),
            free_faces: Vec::new(),
            bbox_min: Point3::origin(),
            bbox_max: Point3::origin(),
            bbox_valid: false,
        }
    }

    /// Build mesh from indexed face set (typical .obj format)
    /// 
    /// # Arguments
    /// * `positions` - Vertex positions [[x,y,z], ...]
    /// * `face_indices` - Face vertex indices (triangles or quads)
    /// * `normals` - Optional per-vertex normals
    /// * `uvs` - Optional per-vertex UVs
    ///
    /// # Returns
    /// Valid half-edge mesh or error if input is non-manifold
    pub fn from_indexed_faces(
        positions: &[[f32; 3]],
        face_indices: &[Vec<u32>],
        normals: Option<&[[f32; 3]]>,
        uvs: Option<&[[f32; 2]]>,
    ) -> Result<Self, MeshError> {
        let mut mesh = Self::new();

        // Create vertices
        for (i, pos) in positions.iter().enumerate() {
            let vid = mesh.add_vertex(Point3::new(pos[0], pos[1], pos[2]));
            assert_eq!(vid.0 as usize, i, "Vertex IDs must be sequential");

            if let Some(n) = normals {
                mesh.vertices[i].normal = Some(Vector3::new(n[i][0], n[i][1], n[i][2]));
            }
            if let Some(u) = uvs {
                mesh.vertices[i].uv = Some(u[i]);
            }
        }

        // Build edge map to find twins
        // Key: (min_vid, max_vid) -> halfedge_id
        let mut edge_map: std::collections::HashMap<(u32, u32), HalfEdgeId> = 
            std::collections::HashMap::new();

        for face_verts in face_indices {
            let n = face_verts.len();
            if n < 3 {
                return Err(MeshError::DegenerateFace);
            }

            // Create half-edges for this face
            let mut face_halfedges = Vec::with_capacity(n);

            for i in 0..n {
                let v0 = face_verts[i];
                let v1 = face_verts[(i + 1) % n];

                let he = mesh.add_halfedge(VertexId(v0));
                face_halfedges.push(he);

                // Check for twin
                let key = (v0.min(v1), v0.max(v1));
                if let Some(&twin) = edge_map.get(&key) {
                    // Found twin - link them
                    let twin_he = &mesh.halfedges[twin.0 as usize];
                    let twin_origin = twin_he.origin.0;

                    // Verify direction is opposite
                    if twin_origin == v1 {
                        mesh.halfedges[he.0 as usize].twin = Some(twin);
                        mesh.halfedges[twin.0 as usize].twin = Some(he);
                    } else {
                        return Err(MeshError::NonManifoldEdge(v0, v1));
                    }
                } else {
                    edge_map.insert(key, he);
                }
            }

            // Link next/prev around face
            for i in 0..n {
                let curr = face_halfedges[i];
                let next = face_halfedges[(i + 1) % n];
                let prev = face_halfedges[(i + n - 1) % n];

                mesh.halfedges[curr.0 as usize].next = next;
                mesh.halfedges[curr.0 as usize].prev = prev;
            }

            // Create face
            let fid = mesh.add_face(face_halfedges[0]);
            for &he in &face_halfedges {
                mesh.halfedges[he.0 as usize].face = Some(fid);
            }
        }

        // Validate boundary consistency
        mesh.validate()?;
        mesh.update_boundary_flags();
        mesh.compute_face_normals();

        Ok(mesh)
    }

    /// Add a vertex, returns its ID
    pub fn add_vertex(&mut self, position: Point3<f32>) -> VertexId {
        let id = if let Some(recycled) = self.free_vertices.pop() {
            recycled
        } else {
            let id = self.next_vertex_id;
            self.next_vertex_id += 1;
            id
        };

        // Ensure vec is large enough
        if id as usize >= self.vertices.len() {
            self.vertices.resize_with(id as usize + 1, || Vertex {
                position: Point3::origin(),
                halfedge: None,
                normal: None,
                uv: None,
                is_boundary: false,
                flags: 0,
            });
        }

        self.vertices[id as usize] = Vertex {
            position,
            halfedge: None,
            normal: None,
            uv: None,
            is_boundary: false,
            flags: 0,
        };

        VertexId(id)
    }

    fn add_halfedge(&mut self, origin: VertexId) -> HalfEdgeId {
        let id = if let Some(recycled) = self.free_halfedges.pop() {
            recycled
        } else {
            let id = self.next_halfedge_id;
            self.next_halfedge_id += 1;
            id
        };

        if id as usize >= self.halfedges.len() {
            self.halfedges.resize_with(id as usize + 1, || HalfEdge {
                origin: VertexId(INVALID_ID),
                twin: None,
                next: HalfEdgeId(INVALID_ID),
                prev: HalfEdgeId(INVALID_ID),
                face: None,
                is_sharp: false,
            });
        }

        self.halfedges[id as usize] = HalfEdge {
            origin,
            twin: None,
            next: HalfEdgeId(INVALID_ID),
            prev: HalfEdgeId(INVALID_ID),
            face: None,
            is_sharp: false,
        };

        // Update vertex reference
        if self.vertices[origin.0 as usize].halfedge.is_none() {
            self.vertices[origin.0 as usize].halfedge = Some(HalfEdgeId(id));
        }

        HalfEdgeId(id)
    }

    fn add_face(&mut self, halfedge: HalfEdgeId) -> FaceId {
        let id = if let Some(recycled) = self.free_faces.pop() {
            recycled
        } else {
            let id = self.next_face_id;
            self.next_face_id += 1;
            id
        };

        if id as usize >= self.faces.len() {
            self.faces.resize_with(id as usize + 1, || Face {
                halfedge: HalfEdgeId(INVALID_ID),
                normal: Vector3::zeros(),
                area: 0.0,
                material: 0,
            });
        }

        self.faces[id as usize] = Face {
            halfedge,
            normal: Vector3::zeros(),
            area: 0.0,
            material: 0,
        };

        FaceId(id)
    }

    // === Element Removal (for edge collapse) ===

    /// Mark a vertex as deleted (soft delete for stable IDs)
    pub(crate) fn remove_vertex(&mut self, vid: VertexId) {
        let idx = vid.0 as usize;
        if idx < self.vertices.len() && !self.free_vertices.contains(&vid.0) {
            self.vertices[idx].halfedge = None;
            self.free_vertices.push(vid.0);
        }
    }

    /// Mark a half-edge as deleted
    pub(crate) fn remove_halfedge(&mut self, heid: HalfEdgeId) {
        let idx = heid.0 as usize;
        if idx < self.halfedges.len() && !self.free_halfedges.contains(&heid.0) {
            self.halfedges[idx].origin = VertexId(INVALID_ID);
            self.halfedges[idx].twin = None;
            self.halfedges[idx].face = None;
            self.free_halfedges.push(heid.0);
        }
    }

    /// Mark a face as deleted
    pub(crate) fn remove_face(&mut self, fid: FaceId) {
        let idx = fid.0 as usize;
        if idx < self.faces.len() && !self.free_faces.contains(&fid.0) {
            self.faces[idx].halfedge = HalfEdgeId(INVALID_ID);
            self.free_faces.push(fid.0);
        }
    }

    /// Update vertex position
    pub fn set_vertex_position(&mut self, vid: VertexId, position: Point3<f32>) {
        if let Some(v) = self.vertex_mut(vid) {
            v.position = position;
            self.bbox_valid = false;
        }
    }

    /// Update vertex halfedge reference
    pub(crate) fn set_vertex_halfedge(&mut self, vid: VertexId, he: Option<HalfEdgeId>) {
        if let Some(v) = self.vertex_mut(vid) {
            v.halfedge = he;
        }
    }

    /// Update half-edge origin
    pub(crate) fn set_halfedge_origin(&mut self, heid: HalfEdgeId, origin: VertexId) {
        if let Some(he) = self.halfedges.get_mut(heid.0 as usize) {
            he.origin = origin;
        }
    }

    /// Update half-edge twin
    pub(crate) fn set_halfedge_twin(&mut self, heid: HalfEdgeId, twin: Option<HalfEdgeId>) {
        if let Some(he) = self.halfedges.get_mut(heid.0 as usize) {
            he.twin = twin;
        }
    }

    /// Update half-edge next
    pub(crate) fn set_halfedge_next(&mut self, heid: HalfEdgeId, next: HalfEdgeId) {
        if let Some(he) = self.halfedges.get_mut(heid.0 as usize) {
            he.next = next;
        }
    }

    /// Update half-edge prev
    pub(crate) fn set_halfedge_prev(&mut self, heid: HalfEdgeId, prev: HalfEdgeId) {
        if let Some(he) = self.halfedges.get_mut(heid.0 as usize) {
            he.prev = prev;
        }
    }

    /// Update half-edge face
    pub(crate) fn set_halfedge_face(&mut self, heid: HalfEdgeId, face: Option<FaceId>) {
        if let Some(he) = self.halfedges.get_mut(heid.0 as usize) {
            he.face = face;
        }
    }

    /// Update face halfedge
    pub(crate) fn set_face_halfedge(&mut self, fid: FaceId, he: HalfEdgeId) {
        if let Some(f) = self.faces.get_mut(fid.0 as usize) {
            f.halfedge = he;
        }
    }

    // === Topology Queries ===

    /// Get vertex by ID
    pub fn vertex(&self, id: VertexId) -> Option<&Vertex> {
        if self.free_vertices.contains(&id.0) {
            return None;
        }
        self.vertices.get(id.0 as usize)
    }

    pub fn vertex_mut(&mut self, id: VertexId) -> Option<&mut Vertex> {
        if self.free_vertices.contains(&id.0) {
            return None;
        }
        self.vertices.get_mut(id.0 as usize)
    }

    /// Get half-edge by ID
    pub fn halfedge(&self, id: HalfEdgeId) -> Option<&HalfEdge> {
        if self.free_halfedges.contains(&id.0) {
            return None;
        }
        self.halfedges.get(id.0 as usize)
    }

    pub fn halfedge_mut(&mut self, id: HalfEdgeId) -> Option<&mut HalfEdge> {
        if self.free_halfedges.contains(&id.0) {
            return None;
        }
        self.halfedges.get_mut(id.0 as usize)
    }

    /// Get face by ID
    pub fn face(&self, id: FaceId) -> Option<&Face> {
        if self.free_faces.contains(&id.0) {
            return None;
        }
        self.faces.get(id.0 as usize)
    }

    /// Count of valid vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() - self.free_vertices.len()
    }

    pub fn halfedge_count(&self) -> usize {
        self.halfedges.len() - self.free_halfedges.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len() - self.free_faces.len()
    }

    /// Iterator over all valid vertices
    pub fn vertices(&self) -> impl Iterator<Item = (VertexId, &Vertex)> {
        self.vertices.iter()
            .enumerate()
            .filter(|(i, _)| !self.free_vertices.contains(&(*i as u32)))
            .map(|(i, v)| (VertexId(i as u32), v))
    }

    /// Iterator over all valid faces
    pub fn faces(&self) -> impl Iterator<Item = (FaceId, &Face)> {
        self.faces.iter()
            .enumerate()
            .filter(|(i, _)| !self.free_faces.contains(&(*i as u32)))
            .map(|(i, f)| (FaceId(i as u32), f))
    }

    /// Iterator over all valid half-edges
    pub fn halfedges(&self) -> impl Iterator<Item = (HalfEdgeId, &HalfEdge)> {
        self.halfedges.iter()
            .enumerate()
            .filter(|(i, _)| !self.free_halfedges.contains(&(*i as u32)))
            .map(|(i, h)| (HalfEdgeId(i as u32), h))
    }

    /// Get one-ring neighbors of a vertex (adjacent vertices)
    pub fn vertex_neighbors(&self, vid: VertexId) -> Vec<VertexId> {
        let mut neighbors = Vec::new();
        let vertex = match self.vertex(vid) {
            Some(v) => v,
            None => return neighbors,
        };

        let start_he = match vertex.halfedge {
            Some(he) => he,
            None => return neighbors,
        };

        let mut curr = start_he;
        loop {
            let he = match self.halfedge(curr) {
                Some(h) => h,
                None => break,
            };
            // The target vertex of this half-edge
            let next_he = match self.halfedge(he.next) {
                Some(h) => h,
                None => break,
            };
            neighbors.push(next_he.origin);

            // Move to next half-edge around vertex
            if let Some(twin) = he.twin {
                curr = match self.halfedge(twin) {
                    Some(t) => t.next,
                    None => break,
                };
            } else {
                break; // Boundary
            }

            if curr == start_he {
                break;
            }
        }

        neighbors
    }

    /// Get faces adjacent to a vertex
    pub fn vertex_faces(&self, vid: VertexId) -> Vec<FaceId> {
        let mut faces = Vec::new();
        let vertex = match self.vertex(vid) {
            Some(v) => v,
            None => return faces,
        };

        let start_he = match vertex.halfedge {
            Some(he) => he,
            None => return faces,
        };

        let mut curr = start_he;
        loop {
            let he = match self.halfedge(curr) {
                Some(h) => h,
                None => break,
            };
            if let Some(fid) = he.face {
                faces.push(fid);
            }

            if let Some(twin) = he.twin {
                curr = match self.halfedge(twin) {
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

        faces
    }

    /// Get vertices of a face (in CCW order)
    pub fn face_vertices(&self, fid: FaceId) -> Vec<VertexId> {
        let mut verts = Vec::new();
        let face = match self.face(fid) {
            Some(f) => f,
            None => return verts,
        };

        let start_he = face.halfedge;
        let mut curr = start_he;

        loop {
            let he = match self.halfedge(curr) {
                Some(h) => h,
                None => break,
            };
            verts.push(he.origin);
            curr = he.next;
            if curr == start_he {
                break;
            }
        }

        verts
    }

    /// Get edges (as half-edge pairs) of a face
    pub fn face_edges(&self, fid: FaceId) -> Vec<HalfEdgeId> {
        let mut edges = Vec::new();
        let face = match self.face(fid) {
            Some(f) => f,
            None => return edges,
        };

        let start_he = face.halfedge;
        let mut curr = start_he;

        loop {
            edges.push(curr);
            let he = match self.halfedge(curr) {
                Some(h) => h,
                None => break,
            };
            curr = he.next;
            if curr == start_he {
                break;
            }
        }

        edges
    }

    /// Get edge endpoints (v0, v1) for a half-edge
    pub fn edge_endpoints(&self, heid: HalfEdgeId) -> Option<(VertexId, VertexId)> {
        let he = self.halfedge(heid)?;
        let v0 = he.origin;
        let v1 = self.halfedge(he.next)?.origin;
        Some((v0, v1))
    }

    /// Is the mesh watertight? (no boundary edges)
    pub fn is_watertight(&self) -> bool {
        self.halfedges.iter()
            .enumerate()
            .filter(|(i, _)| !self.free_halfedges.contains(&(*i as u32)))
            .all(|(_, he)| he.twin.is_some())
    }

    /// Euler characteristic: V - E + F
    pub fn euler_characteristic(&self) -> i32 {
        let v = self.vertex_count() as i32;
        let e = (self.halfedge_count() / 2) as i32;
        let f = self.face_count() as i32;
        v - e + f
    }

    /// Get all edges as (v0, v1) pairs
    pub fn edges(&self) -> Vec<(VertexId, VertexId)> {
        let mut edges = Vec::new();
        let mut seen = HashSet::new();

        for (_heid, he) in self.halfedges() {
            let v0 = he.origin.0;
            let v1 = match self.halfedge(he.next) {
                Some(h) => h.origin.0,
                None => continue,
            };

            let key = (v0.min(v1), v0.max(v1));
            if !seen.contains(&key) {
                seen.insert(key);
                edges.push((he.origin, VertexId(v1)));
            }
        }

        edges
    }

    // === Internal Methods ===

    fn update_boundary_flags(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.is_boundary = false;
        }

        for (i, he) in self.halfedges.iter().enumerate() {
            if self.free_halfedges.contains(&(i as u32)) {
                continue;
            }
            if he.twin.is_none() {
                self.vertices[he.origin.0 as usize].is_boundary = true;
            }
        }
    }

    fn compute_face_normals(&mut self) {
        // Collect data first to avoid borrow issues
        let mut updates = Vec::new();

        for (fid, _) in self.faces.iter().enumerate() {
            if self.free_faces.contains(&(fid as u32)) {
                continue;
            }

            let verts = self.face_vertices(FaceId(fid as u32));
            if verts.len() >= 3 {
                let p0 = self.vertices[verts[0].0 as usize].position;
                let p1 = self.vertices[verts[1].0 as usize].position;
                let p2 = self.vertices[verts[2].0 as usize].position;

                let e1 = p1 - p0;
                let e2 = p2 - p0;
                let cross = e1.cross(&e2);
                let mag = cross.magnitude();

                if mag > 1e-10 {
                    updates.push((fid, cross / mag, mag * 0.5));
                }
            }
        }

        for (fid, normal, area) in updates {
            if let Some(face) = self.faces.get_mut(fid) {
                face.normal = normal;
                face.area = area;
            }
        }
    }

    pub fn validate(&self) -> Result<(), MeshError> {
        // Check all half-edges have valid next/prev
        for (i, he) in self.halfedges.iter().enumerate() {
            if self.free_halfedges.contains(&(i as u32)) {
                continue;
            }

            if he.next.0 == INVALID_ID || he.prev.0 == INVALID_ID {
                return Err(MeshError::InvalidTopology(
                    format!("HalfEdge {} has invalid next/prev", i)
                ));
            }

            // Verify next/prev consistency
            let next_he = &self.halfedges[he.next.0 as usize];
            if next_he.prev != HalfEdgeId(i as u32) {
                return Err(MeshError::InvalidTopology(
                    format!("HalfEdge {} next/prev mismatch", i)
                ));
            }

            // Verify twin consistency
            if let Some(twin) = he.twin {
                let twin_he = &self.halfedges[twin.0 as usize];
                if twin_he.twin != Some(HalfEdgeId(i as u32)) {
                    return Err(MeshError::InvalidTopology(
                        format!("HalfEdge {} twin mismatch", i)
                    ));
                }
            }
        }

        Ok(())
    }

    /// Recompute all derived data after topology changes
    pub fn update_topology(&mut self) {
        self.update_boundary_flags();
        self.compute_face_normals();
    }
}

impl Default for HalfEdgeMesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh construction and validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MeshError {
    #[error("Degenerate face with less than 3 vertices")]
    DegenerateFace,

    #[error("Non-manifold edge between vertices {0} and {1}")]
    NonManifoldEdge(u32, u32),

    #[error("Invalid topology: {0}")]
    InvalidTopology(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
