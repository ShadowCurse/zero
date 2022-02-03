use cgmath::Vector3;
use wgpu::util::DeviceExt;

use crate::renderer;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    _pad1: u32,
    pub color: [f32; 3],
    _pad2: u32,
}

pub struct RenderLight {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderLight {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

pub struct Light {
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
}

impl Light {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(position: P, color: C) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
        }
    }

    pub fn to_uniform(&self) -> LightUniform {
        LightUniform {
            position: self.position.into(),
            color: self.color.into(),
            ..Default::default()
        }
    }
}

impl renderer::RenderAsset for Light {
    type RenderType = RenderLight;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("light_binding_group_layout"),
            })
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
                label: Some("light_uniform"),
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
                label: Some("light_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }
}

impl RenderLight {
    pub fn update(&mut self, renderer: &renderer::Renderer, light: &Light) {
        renderer
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[light.to_uniform()]));
    }
}
