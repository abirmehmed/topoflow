//! Orbital camera for mesh inspection

use nalgebra::{Point3, Vector3, Matrix4, Perspective3};

pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            eye: Point3::new(0.0, 0.0, 5.0),
            target: Point3::origin(),
            up: Vector3::y(),
            fov: 45.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            aspect: 1.0,
        }
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.eye, &self.target, &self.up)
    }

    pub fn projection_matrix(&self) -> Matrix4<f32> {
        Perspective3::new(self.aspect, self.fov, self.near, self.far).to_homogeneous()
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // TODO: Implement orbital rotation around target
    }

    pub fn zoom(&mut self, delta: f32) {
        let direction = self.target - self.eye;
        self.eye += direction * delta;
    }
}
