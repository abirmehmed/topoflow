//! User Interface
//!
//! Built with egui for immediate-mode GUI
//! Panels:
//! - Toolbar (tools, modes)
//! - Properties (mesh stats, retopo settings)
//! - Outliner (objects, materials)
//! - Viewport controls

pub mod panels;
pub mod tools;

pub use panels::*;
