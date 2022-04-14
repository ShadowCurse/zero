use crate::material;
use crate::mesh::{Mesh, MeshVertex};
use crate::texture;
use anyhow::{Context, Ok, Result};

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
