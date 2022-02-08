use cgmath::Vector3;
use wgpu::util::DeviceExt;

use crate::renderer;

#[derive(Debug)]
pub struct RenderLight {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderLight {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl RenderLight {
    pub fn update(&mut self, renderer: &renderer::Renderer, light: &impl renderer::RenderAsset) {
        renderer
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[light.to_uniform()]));
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectionalLightUniform {
    pub direction: [f32; 3],
    _pad1: u32,
    pub color: [f32; 3],
    _pad2: u32,
}

#[derive(Debug)]
pub struct DirectionalLight {
    pub direction: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
}

impl DirectionalLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(direction: P, color: C) -> Self {
        Self {
            direction: direction.into(),
            color: color.into(),
        }
    }
}

impl renderer::RenderAsset for DirectionalLight {
    type RenderType = RenderLight;
    type UniformType = DirectionalLightUniform;

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
                label: Some("directional_light_binding_group_layout"),
            })
    }

    fn to_uniform(&self) -> Self::UniformType {
        Self::UniformType {
            direction: self.direction.into(),
            color: self.color.into(),
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
                label: Some("directional_light_uniform"),
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
                label: Some("directional_light_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightUniform {
    pub position: [f32; 3],
    _pad1: u32,
    pub color: [f32; 3],
    _pad2: u32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
    _pad3: u32,
}

#[derive(Debug)]
pub struct PointLight {
    pub position: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(
        position: P,
        color: C,
        constant: f32,
        linear: f32,
        quadratic: f32,
    ) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
            constant,
            linear,
            quadratic,
        }
    }

    pub fn to_uniform(&self) -> PointLightUniform {
        PointLightUniform {
            position: self.position.into(),
            color: self.color.into(),
            constant: self.constant,
            linear: self.linear,
            quadratic: self.quadratic,
            ..Default::default()
        }
    }
}

impl renderer::RenderAsset for PointLight {
    type RenderType = RenderLight;
    type UniformType = PointLightUniform;

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

    fn to_uniform(&self) -> Self::UniformType {
        Self::UniformType {
            position: self.position.into(),
            color: self.color.into(),
            constant: self.constant,
            linear: self.linear,
            quadratic: self.quadratic,
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

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpotLightUniform {
    pub position: [f32; 3],
    _pad1: u32,
    pub direction: [f32; 3],
    _pad2: u32,
    pub color: [f32; 3],
    _pad3: u32,
}

#[derive(Debug)]
pub struct SpotLight {
    pub position: cgmath::Vector3<f32>,
    pub direction: cgmath::Vector3<f32>,
    pub color: cgmath::Vector3<f32>,
}

impl SpotLight {
    pub fn new<P: Into<Vector3<f32>>, D: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(
        position: P,
        direction: D,
        color: C,
    ) -> Self {
        Self {
            position: position.into(),
            direction: direction.into(),
            color: color.into(),
        }
    }
}

impl renderer::RenderAsset for SpotLight {
    type RenderType = RenderLight;
    type UniformType = SpotLightUniform;

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

    fn to_uniform(&self) -> SpotLightUniform {
        SpotLightUniform {
            position: self.position.into(),
            direction: self.direction.into(),
            color: self.color.into(),
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
