//! GPU mesh buffer management

use crate::mesh::halfedge::HalfEdgeMesh;

pub struct MeshBuffer {
    // TODO: wgpu buffers for vertices, indices, normals
}

impl MeshBuffer {
    pub fn from_mesh(mesh: &HalfEdgeMesh) -> Self {
        // TODO: Upload mesh to GPU
        Self {}
    }
}
