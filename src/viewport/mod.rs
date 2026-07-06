//! 3D Viewport Renderer
//!
//! Uses wgpu for cross-platform GPU rendering
//! Features:
//! - Wireframe overlay
//! - Face shading (flat/smooth)
//! - Edge flow visualization
//! - Symmetry plane display
//! - Selection highlighting

pub mod renderer;
pub mod camera;
pub mod mesh_buffer;

pub use renderer::ViewportRenderer;
pub use camera::Camera;
