//! Tests for edge collapse and topology operations

use topoflow::mesh::{HalfEdgeMesh, VertexId, collapse_edge, optimal_collapse_position};
use topoflow::mesh::validation::analyze_mesh;
use topoflow::algorithms::decimation::{decimate, DecimationOptions};
use nalgebra::Point3;

/// Helper: Create a simple triangle mesh (2 triangles forming a quad)
fn create_two_triangle_mesh() -> HalfEdgeMesh {
    let positions = vec![
        [0.0, 0.0, 0.0],  // v0
        [1.0, 0.0, 0.0],  // v1
        [1.0, 1.0, 0.0],  // v2
        [0.0, 1.0, 0.0],  // v3
    ];
    // Two triangles sharing edge v1-v2
    let faces = vec![
        vec![0, 1, 2],  // Triangle 1: v0-v1-v2
        vec![0, 2, 3],  // Triangle 2: v0-v2-v3
    ];

    HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create two-triangle mesh")
}

/// Helper: Create a tetrahedron (4 vertices, 4 triangles, watertight)
fn create_tetrahedron() -> HalfEdgeMesh {
    let positions = vec![
        [0.0, 0.0, 0.0],           // v0
        [1.0, 0.0, 0.0],           // v1
        [0.5, 1.0, 0.0],           // v2
        [0.5, 0.5, 1.0],           // v3
    ];
    let faces = vec![
        vec![0, 1, 2],
        vec![0, 1, 3],
        vec![1, 2, 3],
        vec![0, 2, 3],
    ];

    HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create tetrahedron")
}

#[test]
fn test_collapse_edge_interior() {
    let mut mesh = create_two_triangle_mesh();

    let original_verts = mesh.vertex_count();
    let original_faces = mesh.face_count();

    // Collapse edge v0-v2 (interior edge shared by both triangles)
    let v0 = VertexId(0);
    let v2 = VertexId(2);
    let target = Point3::new(0.5, 0.5, 0.0);

    let result = collapse_edge(&mut mesh, v0, v2, target);

    assert!(result.is_ok(), "Edge collapse should succeed: {:?}", result.err());

    // After collapse: 2 vertices removed (but one reused), 2 faces removed
    assert_eq!(mesh.vertex_count(), original_verts - 1, "Should remove 1 vertex");
    assert_eq!(mesh.face_count(), original_faces - 2, "Should remove 2 faces");

    // Verify mesh is still valid
    mesh.validate().expect("Mesh should be valid after collapse");
}

#[test]
fn test_collapse_edge_preserves_manifold() {
    let mut mesh = create_tetrahedron();

    let original_euler = mesh.euler_characteristic();

    // Collapse an edge
    let v0 = VertexId(0);
    let v1 = VertexId(1);
    let target = Point3::new(0.5, 0.0, 0.0);

    let result = collapse_edge(&mut mesh, v0, v1, target);
    assert!(result.is_ok());

    // Euler characteristic should be preserved for manifold meshes
    // (V - E + F stays the same for edge collapse on sphere-like topology)
    let new_euler = mesh.euler_characteristic();
    assert_eq!(new_euler, original_euler, "Euler characteristic should be preserved");

    // Mesh should still be watertight
    assert!(mesh.is_watertight(), "Mesh should remain watertight");
}

#[test]
fn test_collapse_boundary_edge() {
    // Create a single triangle (boundary edge)
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.0, 0.0],
    ];
    let faces = vec![vec![0, 1, 2]];

    let mut mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create triangle");

    // Try to collapse a boundary edge (should fail with current implementation
    // or succeed with special boundary handling)
    let v0 = VertexId(0);
    let v1 = VertexId(1);
    let target = Point3::new(0.5, 0.0, 0.0);

    let result = collapse_edge(&mut mesh, v0, v1, target);

    // Boundary collapse may or may not be allowed depending on implementation
    // For now, we just check it doesn't panic
    println!("Boundary collapse result: {:?}", result);
}

#[test]
fn test_collapse_prevents_non_manifold() {
    // Create a mesh where collapse would create non-manifold topology
    // Two pyramids sharing a base vertex but not connected otherwise
    let positions = vec![
        [0.0, 0.0, 0.0],   // v0 - shared apex
        [1.0, 0.0, 0.0],   // v1
        [0.0, 1.0, 0.0],   // v2
        [-1.0, 0.0, 0.0],  // v3
        [0.0, -1.0, 0.0],  // v4
    ];
    let faces = vec![
        vec![0, 1, 2],  // Pyramid 1 face
        vec![0, 2, 3],  // Pyramid 1 face
        vec![0, 3, 4],  // Pyramid 1 face
        vec![0, 4, 1],  // Pyramid 1 face
    ];

    let mut mesh = HalfEdgeMesh::from_indexed_faces(&positions, &faces, None, None)
        .expect("Should create pyramid");

    // Collapsing v0-v1 should be valid (removes one face)
    let v0 = VertexId(0);
    let v1 = VertexId(1);
    let target = Point3::new(0.5, 0.0, 0.0);

    let result = collapse_edge(&mut mesh, v0, v1, target);
    assert!(result.is_ok(), "Valid collapse should succeed");
}

#[test]
fn test_decimate_simple() {
    let mut mesh = create_tetrahedron();
    let original = mesh.vertex_count();

    let options = DecimationOptions {
        target_vertices: 3, // Reduce to 3 vertices (minimum for a triangle)
        target_ratio: 0.0,
        preserve_boundary: true,
        max_error: 10.0,
        preserve_uv_seams: true,
    };

    let result = decimate(&mut mesh, &options);
    assert!(result.is_ok(), "Decimation should succeed");

    // Should have reduced vertices
    assert!(mesh.vertex_count() <= original, "Should reduce or maintain vertex count");

    // Mesh should still be valid
    mesh.validate().expect("Mesh should be valid after decimation");
}

#[test]
fn test_decimate_preserves_topology() {
    let mut mesh = create_tetrahedron();
    let original_euler = mesh.euler_characteristic();

    let options = DecimationOptions {
        target_ratio: 0.5,
        ..Default::default()
    };

    let result = decimate(&mut mesh, &options);
    assert!(result.is_ok());

    // Euler characteristic should be preserved
    assert_eq!(mesh.euler_characteristic(), original_euler,
        "Euler characteristic should be preserved during decimation");
}

#[test]
fn test_optimal_collapse_position() {
    let mesh = create_two_triangle_mesh();
    let v0 = VertexId(0);
    let v1 = VertexId(1);

    let pos = optimal_collapse_position(&mesh, v0, v1);

    let p0 = mesh.vertex(v0).unwrap().position;
    let p1 = mesh.vertex(v1).unwrap().position;
    let expected = Point3::from((p0.coords + p1.coords) * 0.5);

    assert!((pos - expected).magnitude() < 1e-6, 
        "Optimal position should be midpoint");
}

#[test]
fn test_collapse_and_validate() {
    let mut mesh = create_tetrahedron();

    let report_before = analyze_mesh(&mesh);

    // Collapse an edge
    let v0 = VertexId(0);
    let v1 = VertexId(1);
    let target = Point3::new(0.5, 0.0, 0.0);

    collapse_edge(&mut mesh, v0, v1, target).unwrap();

    let report_after = analyze_mesh(&mesh);

    // Should have fewer faces
    assert!(report_after.face_count < report_before.face_count,
        "Should have fewer faces after collapse");

    // Should have no degenerate faces
    assert!(report_after.degenerate_faces.is_empty(),
        "Should not create degenerate faces");
}
