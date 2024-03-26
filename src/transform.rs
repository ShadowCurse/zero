use cgmath::Zero;

use crate::render::prelude::*;
use crate::{cgmath_imports::*, impl_simple_buffer};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    rotate: [[f32; 4]; 4],
}

impl From<&Transform> for TransformUniform {
    fn from(value: &Transform) -> Self {
        let rotate = Matrix4::from(value.rotation);
        Self {
            transform: (Matrix4::from_translation(value.translation)
                * rotate
                * Matrix4::from_nonuniform_scale(value.scale.x, value.scale.y, value.scale.z))
            .into(),
            rotate: rotate.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl From<&Transform> for Matrix4<f32> {
    fn from(value: &Transform) -> Self {
        Matrix4::from_translation(value.translation)
            * Matrix4::from(value.rotation)
            * Matrix4::from_nonuniform_scale(value.scale.x, value.scale.y, value.scale.z)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::zero(),
            rotation: Quaternion::zero(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl_simple_buffer!(
    Transform,
    TransformUniform,
    TransformResources,
    TransformHandle,
    TransformBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX },
    { BufferBindingType::Uniform }
);
