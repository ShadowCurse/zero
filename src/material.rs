use crate::impl_simple_buffer;
use crate::prelude::GpuTexture;
use crate::render::prelude::*;
use crate::texture::ImageTexture;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialPropertiesUniform {
    ambient: [f32; 3],
    _pad1: f32,
    diffuse: [f32; 3],
    _pad2: f32,
    specular: [f32; 3],
    _pad3: f32,
    shininess: f32,
    _pad4: f32,
    _pad5: f32,
    _pad6: f32,
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: ImageTexture,
    pub normal_texture: ImageTexture,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}

impl Material {
    fn to_uniform(&self) -> MaterialPropertiesUniform {
        MaterialPropertiesUniform {
            ambient: self.ambient,
            diffuse: self.diffuse,
            specular: self.specular,
            shininess: self.shininess,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct MaterialResource {
    buffer: Buffer,
    diffuse_texture: GpuTexture,
    normal_texture: GpuTexture,
}

impl GpuResource for Material {
    type ResourceType = MaterialResource;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let diffuse_texture = self.diffuse_texture.build(renderer);
        let normal_texture = self.normal_texture.build(renderer);

        let properties = self.to_uniform();

        let buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("material_params_buffer"),
            contents: bytemuck::cast_slice(&[properties]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self::ResourceType {
            buffer,
            diffuse_texture,
            normal_texture,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialHandle {
    pub buffer_id: ResourceId,
    pub diffuse_texture_id: ResourceId,
    pub normal_texture_id: ResourceId,
}

impl ResourceHandle for MaterialHandle {
    type OriginalResource<'a> = Material;
    type ResourceType = MaterialResource;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            buffer_id: storage.insert_buffer(resource.buffer),
            diffuse_texture_id: storage.insert_texture(resource.diffuse_texture),
            normal_texture_id: storage.insert_texture(resource.normal_texture),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_buffer(self.buffer_id, resource.buffer);
        storage.replace_texture(self.diffuse_texture_id, resource.diffuse_texture);
        storage.replace_texture(self.normal_texture_id, resource.normal_texture);
    }

    fn update(
        &self,
        renderer: &Renderer,
        storage: &RenderStorage,
        original: &Self::OriginalResource<'_>,
    ) {
        renderer.queue().write_buffer(
            storage.get_buffer(self.buffer_id),
            0,
            bytemuck::cast_slice(&[original.to_uniform()]),
        );
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct MaterialBindGroup(pub ResourceId);

impl AssetBindGroup for MaterialBindGroup {
    type ResourceHandle = MaterialHandle;

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
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("material_bind_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self {
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resource.buffer_id);
        let diffuse_texture = storage.get_texture(resource.diffuse_texture_id);
        let normal_texture = storage.get_texture(resource.normal_texture_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal_texture.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&normal_texture.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        Self(storage.insert_bind_group(bind_group))
    }

    fn replace(
        &self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) {
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resource.buffer_id);
        let diffuse_texture = storage.get_texture(resource.diffuse_texture_id);
        let normal_texture = storage.get_texture(resource.normal_texture_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal_texture.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&normal_texture.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        storage.replace_bind_group(self.0, bind_group);
    }
}

impl From<&ColorMaterial> for MaterialPropertiesUniform {
    fn from(value: &ColorMaterial) -> Self {
        Self {
            ambient: value.ambient,
            diffuse: value.diffuse,
            specular: value.specular,
            shininess: value.shininess,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct ColorMaterial {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}

impl_simple_buffer!(
    ColorMaterial,
    MaterialPropertiesUniform,
    ColorMaterialResources,
    ColorMaterialHandle,
    ColorMaterialBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);
