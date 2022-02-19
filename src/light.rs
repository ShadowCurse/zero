use cgmath::Vector3;
use wgpu::util::DeviceExt;

use crate::renderer;

#[derive(Debug)]
pub struct RenderLights {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderLights {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct RenderLight {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderLight {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectionalLightUniform {
    direction: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
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

    fn to_uniform(&self) -> DirectionalLightUniform {
        DirectionalLightUniform {
            direction: self.direction.into(),
            color: self.color.into(),
            ..Default::default()
        }
    }
}

impl renderer::RenderAsset for DirectionalLight {
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
                label: Some("directional_light_binding_group_layout"),
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

    fn update(&self, renderer: &renderer::Renderer, render_type: &Self::RenderType) {
        renderer.queue.write_buffer(
            &render_type.buffer,
            0,
            bytemuck::cast_slice(&[self.to_uniform()]),
        );
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightUniform {
    position: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
    _pad2: u32,
    constant: f32,
    linear: f32,
    quadratic: f32,
    _pad3: u32,
}

#[derive(Debug, Clone)]
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

    fn to_uniform(&self) -> PointLightUniform {
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
                label: Some("point_light_binding_group_layout"),
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
                label: Some("point_light_uniform"),
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
                label: Some("point_light_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }

    fn update(&self, renderer: &renderer::Renderer, render_type: &Self::RenderType) {
        renderer.queue.write_buffer(
            &render_type.buffer,
            0,
            bytemuck::cast_slice(&[self.to_uniform()]),
        );
    }
}

#[derive(Debug, Clone)]
pub struct PointLights {
    pub lights: Vec<PointLight>,
}

impl PointLights {
    fn to_uniform(&self) -> Vec<PointLightUniform> {
        self.lights.iter().map(|light| light.to_uniform()).collect()
    }
}

impl renderer::RenderAsset for PointLights {
    type RenderType = RenderLights;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("point_lights_binding_group_layout"),
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::RenderType {
        let uniforms = self.to_uniform();

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("point_lights_uniform"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("point_lights_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }

    fn update(&self, renderer: &renderer::Renderer, render_type: &Self::RenderType) {
        renderer.queue.write_buffer(
            &render_type.buffer,
            0,
            bytemuck::cast_slice(&self.to_uniform()),
        );
    }
}


#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpotLightUniform {
    position: [f32; 3],
    _pad1: u32,
    direction: [f32; 3],
    _pad2: u32,
    color: [f32; 3],
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

    fn to_uniform(&self) -> SpotLightUniform {
        SpotLightUniform {
            position: self.position.into(),
            direction: self.direction.into(),
            color: self.color.into(),
            ..Default::default()
        }
    }
}

impl renderer::RenderAsset for SpotLight {
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
                label: Some("spot_light_binding_group_layout"),
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
                label: Some("spot_light_uniform"),
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
                label: Some("spot_light_bind_group"),
            });

        Self::RenderType { buffer, bind_group }
    }

    fn update(&self, renderer: &renderer::Renderer, render_type: &Self::RenderType) {
        renderer.queue.write_buffer(
            &render_type.buffer,
            0,
            bytemuck::cast_slice(&[self.to_uniform()]),
        );
    }
}
