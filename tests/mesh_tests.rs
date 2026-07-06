//! Tests for mesh data structures

use topoflow::mesh::HalfEdgeMesh;
use nalgebra::Point3;

#[test]
fn test_create_empty_mesh() {
    let mesh = HalfEdgeMesh::new();
    assert_eq!(mesh.vertex_count(), 0);
    assert_eq!(mesh.face_count(), 0);
}

#[test]
fn test_triangle_mesh() {
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.0, 0.0],
    ];
    let faces = vec![vec![0, 1, 2]];

    let mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create triangle mesh");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
    assert!(mesh.is_watertight());
    assert_eq!(mesh.euler_characteristic(), 1); // V - E + F = 3 - 3 + 1 = 1 (disk)
}

#[test]
fn test_quad_mesh() {
    let positions = vec![
        [0.0, 0.0, 0.0],  // 0
        [1.0, 0.0, 0.0],  // 1
        [1.0, 1.0, 0.0],  // 2
        [0.0, 1.0, 0.0],  // 3
    ];
    let faces = vec![vec![0, 1, 2, 3]];

    let mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create quad mesh");

    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.halfedge_count(), 4);
}

#[test]
fn test_cube_mesh() {
    // Simple cube: 8 vertices, 6 quads
    let positions = vec![
        [-1.0, -1.0, -1.0], // 0
        [ 1.0, -1.0, -1.0], // 1
        [ 1.0,  1.0, -1.0], // 2
        [-1.0,  1.0, -1.0], // 3
        [-1.0, -1.0,  1.0], // 4
        [ 1.0, -1.0,  1.0], // 5
        [ 1.0,  1.0,  1.0], // 6
        [-1.0,  1.0,  1.0], // 7
    ];

    let faces = vec![
        vec![0, 1, 2, 3], // Front
        vec![5, 4, 7, 6], // Back
        vec![4, 0, 3, 7], // Left
        vec![1, 5, 6, 2], // Right
        vec![3, 2, 6, 7], // Top
        vec![4, 5, 1, 0], // Bottom
    ];

    let mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create cube mesh");

    assert_eq!(mesh.vertex_count(), 8);
    assert_eq!(mesh.face_count(), 6);
    assert!(mesh.is_watertight());
    // Euler: V - E + F = 8 - 12 + 6 = 2 (sphere topology)
    assert_eq!(mesh.euler_characteristic(), 2);
}

#[test]
fn test_vertex_neighbors() {
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.0, 0.0],
    ];
    let faces = vec![vec![0, 1, 2]];

    let mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create mesh");

    let neighbors = mesh.vertex_neighbors(topoflow::mesh::VertexId(0));
    assert_eq!(neighbors.len(), 2); // Connected to vertices 1 and 2
}
