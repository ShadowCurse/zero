use wgpu::util::DeviceExt;

use crate::renderer;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    rotate: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct RenderTransform {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl RenderTransform {
    pub fn update(
        &mut self,
        renderer: &renderer::Renderer,
        transform: &impl renderer::RenderAsset,
    ) {
        renderer.queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[transform.to_uniform()]),
        );
    }
}

impl renderer::RenderResource for RenderTransform {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct Transform {
    pub translation: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,
}

impl renderer::RenderAsset for Transform {
    type RenderType = RenderTransform;
    type UniformType = TransformUniform;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("transform_bind_group_layout"),
            })
    }

    fn to_uniform(&self) -> Self::UniformType {
        let rotate = cgmath::Matrix4::from(self.rotation);
        Self::UniformType {
            transform: (cgmath::Matrix4::from_translation(self.translation)
                * rotate
                * cgmath::Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z))
            .into(),
            rotate: rotate.into(),
            ..Default::default()
        }
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::RenderType {
        let uniform = self.to_uniform();
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("transform_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }
}
