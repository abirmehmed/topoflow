//! Tests for file I/O

use topoflow::io::ObjLoader;
use std::io::Cursor;

#[test]
fn test_obj_load_triangle() {
    let obj_data = r#"
# Simple triangle
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.5 1.0 0.0
f 1 2 3
"#;

    let reader = Cursor::new(obj_data);
    let mesh = ObjLoader::from_reader(reader).expect("Should parse OBJ");

    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.face_count(), 1);
}

#[test]
fn test_obj_load_quad() {
    let obj_data = r#"
# Simple quad
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
f 1 2 3 4
"#;

    let reader = Cursor::new(obj_data);
    let mesh = ObjLoader::from_reader(reader).expect("Should parse OBJ quad");

    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.face_count(), 1);
}

#[test]
fn test_obj_with_normals_and_uvs() {
    let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.5 1.0 0.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.5 1.0
f 1/1/1 2/2/2 3/3/3
"#;

    let reader = Cursor::new(obj_data);
    let mesh = ObjLoader::from_reader(reader).expect("Should parse OBJ with attributes");

    assert_eq!(mesh.vertex_count(), 3);
}

#[test]
fn test_obj_export_import() {
    use topoflow::io::ObjExporter;

    let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.5 1.0 0.0
f 1 2 3
"#;

    let reader = Cursor::new(obj_data);
    let mesh = ObjLoader::from_reader(reader).unwrap();

    let mut output = Vec::new();
    ObjExporter::to_writer(&mesh, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("v 0 0 0"));
    assert!(output_str.contains("f 1 2 3"));
}
