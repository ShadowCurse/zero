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

#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
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
