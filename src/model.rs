use crate::material;
use crate::renderer::*;
use crate::texture;
use anyhow::{Context, Ok, Result};
use cgmath::{InnerSpace, Vector2, Vector3};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

impl From<([f32; 3], [f32; 2], [f32; 3])> for ModelVertex {
    fn from(data: ([f32; 3], [f32; 2], [f32; 3])) -> Self {
        Self {
            position: data.0,
            tex_coords: data.1,
            normal: data.2,
            ..Default::default()
        }
    }
}

impl ModelVertex {
    pub fn calc_tangents_and_bitangents(vertices: &mut Vec<ModelVertex>, indices: &[u32]) {
        for c in indices.chunks(3) {
            let v0 = vertices[c[0] as usize];
            let v1 = vertices[c[1] as usize];
            let v2 = vertices[c[2] as usize];

            let pos0: Vector3<_> = v0.position.into();
            let pos1: Vector3<_> = v1.position.into();
            let pos2: Vector3<_> = v2.position.into();

            let uv0: Vector2<_> = v0.tex_coords.into();
            let uv1: Vector2<_> = v1.tex_coords.into();
            let uv2: Vector2<_> = v2.tex_coords.into();

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

            vertices[c[0] as usize].tangent =
                (tangent + Vector3::from(vertices[c[0] as usize].tangent)).into();
            vertices[c[1] as usize].tangent =
                (tangent + Vector3::from(vertices[c[1] as usize].tangent)).into();
            vertices[c[2] as usize].tangent =
                (tangent + Vector3::from(vertices[c[2] as usize].tangent)).into();
            vertices[c[0] as usize].bitangent =
                (bitangent + Vector3::from(vertices[c[0] as usize].bitangent)).into();
            vertices[c[1] as usize].bitangent =
                (bitangent + Vector3::from(vertices[c[1] as usize].bitangent)).into();
            vertices[c[2] as usize].bitangent =
                (bitangent + Vector3::from(vertices[c[2] as usize].bitangent)).into();
        }

        for v in vertices.iter_mut() {
            v.tangent = Vector3::from(v.tangent).normalize().into();
            v.bitangent = Vector3::from(v.bitangent).normalize().into();
        }
    }
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 11]>() as BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Option<Buffer>,
    pub num_elements: u32,
}

impl GpuResource for GpuMesh {}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
}

impl GpuAsset for Mesh {
    type GpuType = GpuMesh;

    fn build(&self, renderer: &Renderer) -> Self::GpuType {
        let vertex_buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: BufferUsages::INDEX,
        });

        Self::GpuType {
            vertex_buffer,
            index_buffer: Some(index_buffer),
            num_elements: self.indices.len() as u32,
        }
    }
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<material::Material>,
}

impl Model {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let obj_materials = obj_materials?;

        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = containing_folder.join(mat.diffuse_texture);
            let diffuse_texture =
                texture::ImageTexture::load(diffuse_path, texture::TextureType::Diffuse)?;

            let normal_path = containing_folder.join(mat.normal_texture);
            let normal_texture =
                texture::ImageTexture::load(normal_path, texture::TextureType::Normal)?;

            materials.push(material::Material {
                name: mat.name,
                diffuse_texture,
                normal_texture,
                ambient: mat.ambient,
                diffuse: mat.diffuse,
                specular: mat.specular,
                shininess: mat.shininess,
            });
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                });
            }

            ModelVertex::calc_tangents_and_bitangents(&mut vertices, &m.mesh.indices);

            meshes.push(Mesh {
                name: m.name,
                vertices,
                indices: m.mesh.indices,
                // material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self { meshes, materials })
    }

    // TODO
    // pub fn build(
    //     &self,
    //     renderer: &Renderer,
    //     material_builder: &RenderAssetBuilder<material::Material>,
    // ) -> RenderModel {
    //     let meshes = self
    //         .meshes
    //         .iter()
    //         .map(|mesh| mesh.build(renderer))
    //         .collect();
    //
    //     let materials = self
    //         .materials
    //         .iter()
    //         .map(|material| material_builder.build(renderer, material))
    //         .collect();
    //
    //     RenderModel { meshes, materials }
    // }
}
