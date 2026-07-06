//! Mesh module - Core data structures for topology representation
//! 
//! Uses half-edge data structure for efficient topology queries:
//! - O(1) edge traversal
//! - O(1) face/vertex adjacency
//! - Efficient edge collapse, split, flip operations

pub mod halfedge;
pub mod topology;
pub mod attributes;
pub mod validation;

pub use halfedge::{HalfEdgeMesh, VertexId, EdgeId, FaceId, HalfEdgeId, MeshError};
pub use topology::{TopologyError, collapse_edge, flip_edge, optimal_collapse_position};
