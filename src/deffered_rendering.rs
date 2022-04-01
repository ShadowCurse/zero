use crate::model::GpuMesh;
use crate::render_phase::RenderResources;
use crate::renderer::{GpuAsset, RenderAsset, Renderer};
use crate::texture::{GpuTexture, TextureVertex};
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct GBufferTexture {
    pub format: wgpu::TextureFormat,
}

impl GBufferTexture {
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self { format }
    }
}

impl GpuAsset for GBufferTexture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::GpuType {
        let texture_size = wgpu::Extent3d {
            width: renderer.config.width,
            height: renderer.config.height,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("gbuffer_texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug)]
pub struct GBuffer {
    pub position: GBufferTexture,
    pub normal: GBufferTexture,
    pub albedo: GBufferTexture,
}

impl GBuffer {
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            position: GBufferTexture::new(format),
            normal: GBufferTexture::new(format),
            albedo: GBufferTexture::new(format),
        }
    }
}

impl RenderAsset for GBuffer {
    const ASSET_NAME: &'static str = "GBuffer";

    fn bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout {
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("deffered_pass_bind_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &wgpu::BindGroupLayout) -> RenderResources {
        let vertices: Vec<TextureVertex> = vec![
            ([-1.0, 1.0, 0.0], [0.0, 0.0]),
            ([-1.0, -1.0, 0.0], [0.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, -1.0, 0.0], [1.0, 1.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let indices = vec![0, 1, 2, 2, 1, 3];

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let position = self.position.build(renderer);
        let normal = self.normal.build(renderer);
        let albedo = self.albedo.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&position.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&position.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&normal.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&albedo.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&albedo.sampler),
                    },
                ],
                label: None,
            });

        let mesh = GpuMesh {
            vertex_buffer,
            index_buffer: Some(index_buffer),
            num_elements: 6,
        };

        RenderResources {
            textures: vec![position, normal, albedo],
            meshes: vec![mesh],
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }
}
