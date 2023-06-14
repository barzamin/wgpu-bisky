use bytemuck::{Pod, Zeroable};
use std::ops::Range;
use ultraviolet::{projection::orthographic_wgpu_dx, Mat4, Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct CameraUniforms {
    pub(crate) view: Mat4,
    pub(crate) project: Mat4,
}

impl CameraUniforms {
    pub fn new() -> Self {
        // Iᵀ = I
        Self {
            view: Mat4::identity(),
            project: Mat4::identity(),
        }
    }
}

pub(crate) trait ComputeCameraUniforms {
    fn compute_camera_uniforms(&self) -> CameraUniforms;
}

pub(crate) struct OrthoCamera {
    pub(crate) position: Vec3,
    pub(crate) aimdir: Vec3,
    pub(crate) up: Vec3,

    pub(crate) aspect: f32,
    pub(crate) zrange: Range<f32>,
}

impl OrthoCamera {
    pub fn new(position: Vec3, aimdir: Vec3, up: Vec3, aspect: f32, zrange: Range<f32>) -> Self {
        Self {
            position,
            aimdir,
            up,
            aspect,
            zrange,
        }
    }
}

impl ComputeCameraUniforms for OrthoCamera {
    fn compute_camera_uniforms(&self) -> CameraUniforms {
        let fwd = self.aimdir.normalized();
        let right = fwd.cross(self.up).normalized();
        let up = right.cross(fwd);
        let view = Mat4::new(
            Vec4::new(right.x, up.x, -fwd.x, 0.0),
            Vec4::new(right.y, up.y, -fwd.y, 0.0),
            Vec4::new(right.z, up.z, -fwd.z, 0.0),
            Vec4::new(
                -right.dot(self.position),
                -up.dot(self.position),
                fwd.dot(self.position),
                1.0,
            ),
        );

        let project = orthographic_wgpu_dx(
            -1.,
            1.,
            -self.aspect,
            self.aspect,
            self.zrange.start,
            self.zrange.end,
        );

        CameraUniforms { view, project }
    }
}
