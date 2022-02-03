use wgpu::util::DeviceExt;

use crate::renderer;
use crate::texture;

pub struct RenderMaterial {
    pub diffuse_texture: texture::RenderTexture,
    pub normal_texture: texture::RenderTexture,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl renderer::GpuResource for RenderMaterial {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

pub struct RenderColorMaterial {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl renderer::GpuResource for RenderColorMaterial {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}


#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialPropertiesUniform {
    pub ambient: [f32; 3],
    pub _pad1: f32,
    pub diffuse: [f32; 3],
    pub _pad2: f32,
    pub specular: [f32; 3],
    pub _pad3: f32,
    pub shininess: f32,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}

impl renderer::RenderAsset for Material {
    type GpuType = RenderMaterial;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("material_bind_group_layout"),
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::GpuType {
        let diffuse_texture = self.diffuse_texture.build(renderer);
        let normal_texture = self.normal_texture.build(renderer);

        let properties = MaterialPropertiesUniform {
            ambient: self.ambient,
            diffuse: self.diffuse,
            specular: self.specular,
            shininess: self.shininess,
            ..Default::default()
        };

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material_params_buffer"),
                contents: bytemuck::cast_slice(&[properties]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffer.as_entire_binding(),
                    },
                ],
                label: None,
            });

        Self::GpuType {
            diffuse_texture,
            normal_texture,
            buffer,
            bind_group,
        }
    }
}

pub struct ColorMaterial {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}

impl renderer::RenderAsset for ColorMaterial {
    type GpuType = RenderColorMaterial;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("color_material_bind_group_layout"),
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::GpuType {
        let uniform = MaterialPropertiesUniform {
            ambient: self.ambient,
            diffuse: self.diffuse,
            specular: self.specular,
            shininess: self.shininess,
            ..Default::default()
        };

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("color_material_params_buffer"),
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
                label: None,
            });
        Self::GpuType { buffer, bind_group }
    }
}
