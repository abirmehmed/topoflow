//! I/O module - File format loaders and exporters
//!
//! Supported formats:
//! - .obj (Wavefront) - Import/Export
//! - .blend (Blender) - Import via Python bridge (future: native binary)
//! - .ply (Stanford) - Future
//! - .fbx (Autodesk) - Future
//! - .gltf (Khronos) - Future

pub mod obj;
pub mod blend;

pub use obj::{ObjLoader, ObjExporter};
pub use blend::BlendLoader;
