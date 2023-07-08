use crate::mesh::GpuMesh;
use crate::render::prelude::*;
use crate::{impl_simple_texture_bind_group, texture};
use image::ImageError;
use texture::GpuTexture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxVertex {
    position: [f32; 3],
}

impl VertexLayout for SkyboxVertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
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
    pub fn load<P: AsRef<std::path::Path>>(paths: [P; 6]) -> Result<Self, ImageError> {
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

        let vertex_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
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

#[derive(Debug, Clone, Copy)]
pub struct SkyboxHandle {
    pub texture_id: ResourceId,
    pub mesh_id: ResourceId,
}

impl ResourceHandle for SkyboxHandle {
    type OriginalResource<'a> = Skybox;
    type ResourceType = SkyboxResources;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
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

impl_simple_texture_bind_group!(
    SkyboxHandle,
    SkyboxBindGroup,
    { TextureViewDimension::Cube },
    { TextureSampleType::Float { filterable: true } }
);
