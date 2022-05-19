use crate::renderer::prelude::*;
use crate::texture;
use crate::mesh::GpuMesh;
use anyhow::{Ok, Result};
use texture::GpuTexture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxVertex {
    position: [f32; 3],
}

impl Vertex for SkyboxVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            }],
        }
    }
}

#[derive(Debug)]
pub struct Skybox {
    pub vertices: Vec<f32>,
    pub num_elements: u32,
    pub cube_map: texture::CubeMap,
}

impl Skybox {
    pub fn load<P: AsRef<std::path::Path>>(paths: [P; 6]) -> Result<Self> {
        let cube_map = texture::CubeMap::load(paths)?;

        let vertices: Vec<f32> = vec![
            -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0,
            -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        Ok(Self {
            vertices,
            num_elements: 36,
            cube_map,
        })
    }
}

pub struct SkyboxResources {
    texture: GpuTexture,
    mesh: GpuMesh,
}

impl GpuResource for Skybox {
    type ResourceType = SkyboxResources;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture = self.cube_map.build(renderer);

        let vertex_buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cube_map_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: BufferUsages::VERTEX,
        });
        let mesh = GpuMesh {
            vertex_buffer,
            index_buffer: None,
            num_elements: self.num_elements,
        };

        Self::ResourceType { texture, mesh }
    }
}

pub struct SkyboxHandle {
    pub texture_id: ResourceId,
    pub mesh_id: ResourceId,
}

impl ResourceHandle for SkyboxHandle {
    type OriginalResource = Skybox;
    type ResourceType = SkyboxResources;

    fn from_resource(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            texture_id: storage.insert_texture(resource.texture),
            mesh_id: storage.insert_mesh(resource.mesh),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.texture_id, resource.texture);
        storage.replace_mesh(self.mesh_id, resource.mesh);
    }
}

pub struct SkyboxBindGroup(pub ResourceId);

impl AssetBindGroup for SkyboxBindGroup {
    type ResourceHandle = SkyboxHandle;

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
                            view_dimension: TextureViewDimension::Cube,
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
                ],
                label: Some("skybox_bind_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resources: &Self::ResourceHandle,
    ) -> Self {
        storage.register_bind_group_layout::<Self>(renderer);
        let layout = storage.get_bind_group_layout::<Self>();
        let texture = storage.get_texture(resources.texture_id);

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: None,
        });

        Self(storage.insert_bind_group(bind_group))
    }
}
