use crate::material::Material;
use crate::mesh::{Mesh, MeshVertex};
use crate::renderer::prelude::*;
use crate::texture::{ImageTexture, TextureType};
use anyhow::{Context, Ok, Result};

#[derive(Debug)]
pub struct ModelMesh {
    pub material_id: usize,
    pub mesh: Mesh,
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<ModelMesh>,
    pub materials: Vec<Material>,
}

#[derive(Debug)]
pub struct MeshId {
    pub id: ResourceId,
    pub material_id: ResourceId,
}

#[derive(Debug)]
pub struct ModelIds {
    pub meshes: Vec<MeshId>,
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
            let diffuse_texture = ImageTexture::load(diffuse_path, TextureType::Diffuse)?;

            let normal_path = containing_folder.join(mat.normal_texture);
            let normal_texture = ImageTexture::load(normal_path, TextureType::Normal)?;

            materials.push(Material {
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
                vertices.push(MeshVertex {
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

            MeshVertex::calc_tangents_and_bitangents(&mut vertices, &m.mesh.indices);

            meshes.push(ModelMesh {
                material_id: m.mesh.material_id.unwrap_or(0),
                mesh: Mesh {
                    name: m.name,
                    vertices,
                    indices: m.mesh.indices,
                },
            });
        }

        Ok(Self { meshes, materials })
    }

    pub fn build(&self, renderer: &Renderer, storage: &mut RenderStorage) -> ModelIds {
        let materials: Vec<_> = self
            .materials
            .iter()
            .map(|m| storage.build_asset(renderer, m))
            .collect();
        let meshes = self
            .meshes
            .iter()
            .map(|m| MeshId {
                id: storage.build_mesh(renderer, &m.mesh),
                material_id: materials[m.material_id],
            })
            .collect();
        ModelIds { meshes }
    }
}
