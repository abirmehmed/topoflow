//! Wavefront .obj file loader and exporter
//!
//! Format supports:
//! - v (vertex position)
//! - vn (vertex normal)
//! - vt (texture coordinate)
//! - f (face, supports triangles, quads, ngons)
//! - o/g (object/group names)
//! - usemtl (material reference)
//!
//! Note: .obj uses 1-based indexing, we convert to 0-based internally

use crate::mesh::halfedge::{HalfEdgeMesh, MeshError};
use nalgebra::Point3;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// OBJ file loader
pub struct ObjLoader;

impl ObjLoader {
    /// Load mesh from .obj file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<HalfEdgeMesh, MeshError> {
        let file = File::open(path).map_err(|e| MeshError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Load mesh from any buffered reader
    pub fn from_reader<R: BufRead>(reader: R) -> Result<HalfEdgeMesh, MeshError> {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut face_indices: Vec<Vec<u32>> = Vec::new();
        let mut face_normal_indices: Vec<Vec<u32>> = Vec::new();
        let mut face_uv_indices: Vec<Vec<u32>> = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| MeshError::IoError(e.to_string()))?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    // Vertex position: v x y z [w]
                    if parts.len() >= 4 {
                        positions.push([
                            parts[1].parse().map_err(|_| MeshError::IoError("Invalid vertex x".to_string()))?,
                            parts[2].parse().map_err(|_| MeshError::IoError("Invalid vertex y".to_string()))?,
                            parts[3].parse().map_err(|_| MeshError::IoError("Invalid vertex z".to_string()))?,
                        ]);
                    }
                }
                "vn" => {
                    // Vertex normal: vn nx ny nz
                    if parts.len() >= 4 {
                        normals.push([
                            parts[1].parse().map_err(|_| MeshError::IoError("Invalid normal x".to_string()))?,
                            parts[2].parse().map_err(|_| MeshError::IoError("Invalid normal y".to_string()))?,
                            parts[3].parse().map_err(|_| MeshError::IoError("Invalid normal z".to_string()))?,
                        ]);
                    }
                }
                "vt" => {
                    // Texture coordinate: vt u v [w]
                    if parts.len() >= 3 {
                        uvs.push([
                            parts[1].parse().map_err(|_| MeshError::IoError("Invalid uv u".to_string()))?,
                            parts[2].parse().map_err(|_| MeshError::IoError("Invalid uv v".to_string()))?,
                        ]);
                    }
                }
                "f" => {
                    // Face: f v1/vt1/vn1 v2/vt2/vn2 ...
                    let mut face_verts = Vec::new();
                    let mut face_norms = Vec::new();
                    let mut face_tex = Vec::new();

                    for i in 1..parts.len() {
                        let indices: Vec<&str> = parts[i].split('/').collect();

                        // Vertex index (required) - convert from 1-based to 0-based
                        let vi: u32 = indices[0].parse().map_err(|_| {
                            MeshError::IoError(format!("Invalid face vertex index: {}", indices[0]))
                        })?;
                        if vi == 0 {
                            return Err(MeshError::IoError("0 is not a valid 1-based index".to_string()));
                        }
                        face_verts.push(vi - 1);

                        // Texture index (optional)
                        if indices.len() > 1 && !indices[1].is_empty() {
                            let ti: u32 = indices[1].parse().map_err(|_| {
                                MeshError::IoError(format!("Invalid face texcoord index: {}", indices[1]))
                            })?;
                            face_tex.push(ti - 1);
                        }

                        // Normal index (optional)
                        if indices.len() > 2 && !indices[2].is_empty() {
                            let ni: u32 = indices[2].parse().map_err(|_| {
                                MeshError::IoError(format!("Invalid face normal index: {}", indices[2]))
                            })?;
                            face_norms.push(ni - 1);
                        }
                    }

                    if face_verts.len() >= 3 {
                        face_indices.push(face_verts);
                        if !face_norms.is_empty() {
                            face_normal_indices.push(face_norms);
                        }
                        if !face_tex.is_empty() {
                            face_uv_indices.push(face_tex);
                        }
                    }
                }
                "o" | "g" | "s" | "usemtl" | "mtllib" => {
                    // Object/group/smooth/material names - store for future use
                    // Currently ignored during mesh construction
                }
                _ => {
                    // Unknown command, skip
                }
            }
        }

        // Build per-vertex normals/uvs if face-indexed
        let vertex_normals = if !face_normal_indices.is_empty() {
            Some(Self::build_per_vertex_attribute(&face_indices, &face_normal_indices, &normals, [0.0, 0.0, 1.0])?)
        } else if !normals.is_empty() {
            Some(normals)
        } else {
            None
        };

        let vertex_uvs = if !face_uv_indices.is_empty() {
            Some(Self::build_per_vertex_attribute(&face_indices, &face_uv_indices, &uvs, [0.0, 0.0])?)
        } else if !uvs.is_empty() {
            Some(uvs)
        } else {
            None
        };

        // Convert normals to expected format
        let normals_ref = vertex_normals.as_ref().map(|n| {
            n.iter().map(|v| [v[0], v[1], v[2]]).collect::<Vec<_>>()
        });

        // Try to build mesh - if non-manifold, attempt to fix by removing duplicate faces
        match HalfEdgeMesh::from_indexed_faces(
            &positions,
            &face_indices,
            normals_ref.as_deref(),
            vertex_uvs.as_deref(),
        ) {
            Ok(mesh) => Ok(mesh),
            Err(MeshError::NonManifoldEdge(v0, v1)) => {
                log::warn!("Non-manifold edge detected between vertices {} and {}. Attempting to fix...", v0, v1);
                // Try removing duplicate faces and retry
                let cleaned_faces = Self::remove_duplicate_faces(&face_indices);
                HalfEdgeMesh::from_indexed_faces(
                    &positions,
                    &cleaned_faces,
                    normals_ref.as_deref(),
                    vertex_uvs.as_deref(),
                )
            }
            Err(e) => Err(e),
        }
    }

    /// Remove duplicate faces (same vertices, different order)
    fn remove_duplicate_faces(faces: &[Vec<u32>]) -> Vec<Vec<u32>> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for face in faces {
            // Create a canonical representation: sorted vertex indices
            let mut sorted = face.clone();
            sorted.sort();

            if !seen.contains(&sorted) {
                seen.insert(sorted);
                result.push(face.clone());
            }
        }

        result
    }

    /// Build per-vertex attributes from face-indexed data
    /// When faces reference different attributes per vertex, we need to duplicate vertices
    fn build_per_vertex_attribute<T: Copy + Default>(
        face_indices: &[Vec<u32>],
        face_attr_indices: &[Vec<u32>],
        attributes: &[T],
        default: T,
    ) -> Result<Vec<T>, MeshError> {
        // For simplicity, assume face_attr_indices aligns with face_indices
        // In a full implementation, this would handle vertex splitting for UV seams
        let mut result = vec![default; face_indices.iter().map(|f| f.len()).sum()];

        let mut vert_idx = 0;
        for (fi, face) in face_indices.iter().enumerate() {
            if let Some(attr_indices) = face_attr_indices.get(fi) {
                for (vi, &vidx) in face.iter().enumerate() {
                    if let Some(&attr_idx) = attr_indices.get(vi) {
                        if (attr_idx as usize) < attributes.len() {
                            result[vert_idx] = attributes[attr_idx as usize];
                        }
                    }
                    vert_idx += 1;
                }
            }
        }

        Ok(result)
    }
}

/// OBJ file exporter
pub struct ObjExporter;

impl ObjExporter {
    /// Export mesh to .obj file
    pub fn to_file<P: AsRef<Path>>(mesh: &HalfEdgeMesh, path: P) -> Result<(), MeshError> {
        let mut file = File::create(path).map_err(|e| MeshError::IoError(e.to_string()))?;
        Self::to_writer(mesh, &mut file)
    }

    /// Export mesh to any writer
    pub fn to_writer<W: Write>(mesh: &HalfEdgeMesh, writer: &mut W) -> Result<(), MeshError> {
        writeln!(writer, "# TopoFlow exported OBJ").map_err(|e| MeshError::IoError(e.to_string()))?;
        writeln!(writer, "# Vertices: {}", mesh.vertex_count()).map_err(|e| MeshError::IoError(e.to_string()))?;
        writeln!(writer, "# Faces: {}", mesh.face_count()).map_err(|e| MeshError::IoError(e.to_string()))?;
        writeln!(writer).map_err(|e| MeshError::IoError(e.to_string()))?;

        // Write vertices
        for (_, vertex) in mesh.vertices() {
            let p = &vertex.position;
            writeln!(writer, "v {} {} {}", p.x, p.y, p.z)
                .map_err(|e| MeshError::IoError(e.to_string()))?;
        }

        // Write normals
        let has_normals = mesh.vertices().any(|(_, v)| v.normal.is_some());
        if has_normals {
            writeln!(writer).map_err(|e| MeshError::IoError(e.to_string()))?;
            for (_, vertex) in mesh.vertices() {
                if let Some(n) = vertex.normal {
                    writeln!(writer, "vn {} {} {}", n.x, n.y, n.z)
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                } else {
                    writeln!(writer, "vn 0 0 1")
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                }
            }
        }

        // Write UVs
        let has_uvs = mesh.vertices().any(|(_, v)| v.uv.is_some());
        if has_uvs {
            writeln!(writer).map_err(|e| MeshError::IoError(e.to_string()))?;
            for (_, vertex) in mesh.vertices() {
                if let Some(uv) = vertex.uv {
                    writeln!(writer, "vt {} {}", uv[0], uv[1])
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                } else {
                    writeln!(writer, "vt 0 0")
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                }
            }
        }

        // Write faces
        writeln!(writer).map_err(|e| MeshError::IoError(e.to_string()))?;
        for (fid, _) in mesh.faces() {
            let verts = mesh.face_vertices(fid);

            write!(writer, "f").map_err(|e| MeshError::IoError(e.to_string()))?;
            for vidx in &verts {
                // OBJ uses 1-based indexing
                let idx = vidx.0 + 1;
                if has_normals && has_uvs {
                    write!(writer, " {}/{}/{}", idx, idx, idx)
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                } else if has_uvs {
                    write!(writer, " {}/{}", idx, idx)
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                } else if has_normals {
                    write!(writer, " {}//{}", idx, idx)
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                } else {
                    write!(writer, " {}", idx)
                        .map_err(|e| MeshError::IoError(e.to_string()))?;
                }
            }
            writeln!(writer).map_err(|e| MeshError::IoError(e.to_string()))?;
        }

        Ok(())
    }
}
