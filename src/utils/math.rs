//! Math utilities for geometry processing

use nalgebra::{Point3, Vector3, Matrix3};

/// Compute triangle area using cross product
pub fn triangle_area(p0: &Point3<f32>, p1: &Point3<f32>, p2: &Point3<f32>) -> f32 {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    e1.cross(&e2).magnitude() * 0.5
}

/// Compute triangle normal (not normalized)
pub fn triangle_normal(p0: &Point3<f32>, p1: &Point3<f32>, p2: &Point3<f32>) -> Vector3<f32> {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    e1.cross(&e2)
}

/// Cotangent of angle at p0 in triangle (p0, p1, p2)
pub fn cotangent(p0: &Point3<f32>, p1: &Point3<f32>, p2: &Point3<f32>) -> f32 {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    let dot = e1.dot(&e2);
    let cross_mag = e1.cross(&e2).magnitude();
    if cross_mag < 1e-10 {
        0.0
    } else {
        dot / cross_mag
    }
}

/// Barycentric coordinates of point p in triangle (a, b, c)
pub fn barycentric_coords(
    p: &Point3<f32>,
    a: &Point3<f32>,
    b: &Point3<f32>,
    c: &Point3<f32>,
) -> [f32; 3] {
    let v0 = *b - *a;
    let v1 = *c - *a;
    let v2 = *p - *a;

    let d00 = v0.dot(&v0);
    let d01 = v0.dot(&v1);
    let d11 = v1.dot(&v1);
    let d20 = v2.dot(&v0);
    let d21 = v2.dot(&v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-10 {
        return [1.0/3.0, 1.0/3.0, 1.0/3.0];
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    [u, v, w]
}

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

impl AABB {
    pub fn new() -> Self {
        Self {
            min: Point3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Point3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn expand(&mut self, point: &Point3<f32>) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    pub fn center(&self) -> Point3<f32> {
        Point3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn diagonal(&self) -> f32 {
        (self.max - self.min).magnitude()
    }
}
