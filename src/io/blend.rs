//! Blender .blend file loader
//!
//! Strategy: Use Blender's Python API via subprocess or FFI
//! 
//! The .blend format is a complex binary format that changes between Blender versions.
//! Rather than parsing it natively (which requires constant updates), we:
//! 1. Launch a headless Blender process
//! 2. Run a Python script to export the mesh to .obj
//! 3. Load the .obj using our native loader
//!
//! Future: Native binary parser for .blend files

use crate::mesh::halfedge::{HalfEdgeMesh, MeshError};
use std::path::Path;
use std::process::Command;

/// Blender file loader using Python bridge
pub struct BlendLoader;

impl BlendLoader {
    /// Load mesh from .blend file
    /// Requires Blender 3.0+ installed and in PATH
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<HalfEdgeMesh, MeshError> {
        let blend_path = path.as_ref();

        // Create temporary OBJ file
        let temp_obj = std::env::temp_dir().join("topoflow_blend_export.obj");
        let temp_obj_str = temp_obj.to_string_lossy();
        let blend_path_str = blend_path.to_string_lossy();

        // Python script to export mesh from Blender
        let python_script = format!(r#"
import bpy
import sys

# Load blend file
bpy.ops.wm.open_mainfile(filepath="{}")

# Export all mesh objects
for obj in bpy.context.scene.objects:
    if obj.type == 'MESH':
        # Select only this object
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj

        # Export to OBJ
        bpy.ops.wm.obj_export(
            filepath="{}",
            export_selected_objects=True,
            apply_modifiers=True
        )
        break  # Export first mesh only for now

# Clean up
bpy.ops.wm.quit_blender()
"#, blend_path_str, temp_obj_str);

        // Run Blender headless
        let output = Command::new("blender")
            .args(&["--background", "--python-expr", &python_script])
            .output()
            .map_err(|e| MeshError::IoError(
                format!("Failed to run Blender. Is it installed and in PATH? Error: {}", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MeshError::IoError(
                format!("Blender export failed: {}", stderr)
            ));
        }

        // Load the exported OBJ
        if temp_obj.exists() {
            let mesh = super::obj::ObjLoader::from_file(&temp_obj)?;
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_obj);
            Ok(mesh)
        } else {
            Err(MeshError::IoError(
                "Blender export did not create output file".to_string()
            ))
        }
    }

    /// Check if Blender is available
    pub fn is_blender_available() -> bool {
        Command::new("blender")
            .args(&["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
