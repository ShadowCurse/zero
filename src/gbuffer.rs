use crate::mesh::GpuMesh;
use crate::render::prelude::*;
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

impl GpuResource for GBufferTexture {
    type ResourceType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture_size = Extent3d {
            width: renderer.size().width,
            height: renderer.size().height,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device().create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            label: Some("gbuffer_texture"),
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = renderer.device().create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self::ResourceType {
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

#[derive(Debug)]
pub struct GBufferResource {
    position_texture: GpuTexture,
    normal_texture: GpuTexture,
    albedo_texture: GpuTexture,
    mesh: GpuMesh,
}

impl GpuResource for GBuffer {
    type ResourceType = GBufferResource;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let vertices: Vec<TextureVertex> = vec![
            ([-1.0, 1.0, 0.0], [0.0, 0.0]),
            ([-1.0, -1.0, 0.0], [0.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, -1.0, 0.0], [1.0, 1.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let vertex_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("gbuffer_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let indices = vec![0, 1, 2, 2, 1, 3];

        let index_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("gbuffer_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        let position_texture = self.position.build(renderer);
        let normal_texture = self.normal.build(renderer);
        let albedo_texture = self.albedo.build(renderer);

        let mesh = GpuMesh {
            vertex_buffer,
            index_buffer: Some(index_buffer),
            num_elements: 6,
        };

        Self::ResourceType {
            position_texture,
            normal_texture,
            albedo_texture,
            mesh,
        }
    }
}

#[derive(Debug)]
pub struct GBufferHandle {
    pub position_texture_id: ResourceId,
    pub normal_texture_id: ResourceId,
    pub albedo_texture_id: ResourceId,
    pub mesh_id: ResourceId,
}

impl ResourceHandle for GBufferHandle {
    type OriginalResource = GBuffer;
    type ResourceType = GBufferResource;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            position_texture_id: storage.insert_texture(resource.position_texture),
            normal_texture_id: storage.insert_texture(resource.normal_texture),
            albedo_texture_id: storage.insert_texture(resource.albedo_texture),
            mesh_id: storage.insert_mesh(resource.mesh),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.position_texture_id, resource.position_texture);
        storage.replace_texture(self.normal_texture_id, resource.normal_texture);
        storage.replace_texture(self.albedo_texture_id, resource.albedo_texture);
        storage.replace_mesh(self.mesh_id, resource.mesh);
    }
}

#[derive(Debug)]
pub struct GBufferBindGroup(pub ResourceId);

impl AssetBindGroup for GBufferBindGroup {
    type ResourceHandle = GBufferHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
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
                label: Some("gbuffer_bind_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resources: &Self::ResourceHandle,
    ) -> Self {
        storage.register_bind_group_layout::<Self>(renderer);
        storage.register_bind_group_layout::<Self>(renderer);
        let layout = storage.get_bind_group_layout::<Self>();
        let position = storage.get_texture(resources.position_texture_id);
        let normal = storage.get_texture(resources.normal_texture_id);
        let albedo = storage.get_texture(resources.albedo_texture_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
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

        Self(storage.insert_bind_group(bind_group))
    }
}
