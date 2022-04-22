use crate::mesh::GpuMesh;
use crate::renderer::prelude::*;
use crate::texture::{GpuTexture, TextureVertex};

#[derive(Debug)]
pub struct GBufferTexture {
    pub format: TextureFormat,
}

impl GBufferTexture {
    pub fn new(format: TextureFormat) -> Self {
        Self { format }
    }
}

impl GpuAsset for GBufferTexture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::GpuType {
        let texture_size = Extent3d {
            width: renderer.config.width,
            height: renderer.config.height,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            label: Some("gbuffer_texture"),
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
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
    pub fn new(format: TextureFormat) -> Self {
        Self {
            position: GBufferTexture::new(format),
            normal: GBufferTexture::new(format),
            albedo: GBufferTexture::new(format),
        }
    }
}

impl RenderAsset for GBuffer {
    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("deffered_pass_bind_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
        let vertices: Vec<TextureVertex> = vec![
            ([-1.0, 1.0, 0.0], [0.0, 0.0]),
            ([-1.0, -1.0, 0.0], [0.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, -1.0, 0.0], [1.0, 1.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let vertex_buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let indices = vec![0, 1, 2, 2, 1, 3];

        let index_buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        let position = self.position.build(renderer);
        let normal = self.normal.build(renderer);
        let albedo = self.albedo.build(renderer);

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&position.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&position.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&normal.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&albedo.view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&albedo.sampler),
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
