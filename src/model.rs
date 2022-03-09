use anyhow::{Context, Ok, Result};
use cgmath::InnerSpace;
use wgpu::util::DeviceExt;

use crate::camera;
use crate::material;
use crate::renderer::{self, GpuAsset, RenderResource};
use crate::texture;
use crate::transform;

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

            let pos0: cgmath::Vector3<_> = v0.position.into();
            let pos1: cgmath::Vector3<_> = v1.position.into();
            let pos2: cgmath::Vector3<_> = v2.position.into();

            let uv0: cgmath::Vector2<_> = v0.tex_coords.into();
            let uv1: cgmath::Vector2<_> = v1.tex_coords.into();
            let uv2: cgmath::Vector2<_> = v2.tex_coords.into();

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

            vertices[c[0] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
            vertices[c[1] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
            vertices[c[2] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
            vertices[c[0] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
            vertices[c[1] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
            vertices[c[2] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();
        }

        for v in vertices.iter_mut() {
            v.tangent = cgmath::Vector3::from(v.tangent).normalize().into();
            v.bitangent = cgmath::Vector3::from(v.bitangent).normalize().into();
        }
    }
}

impl renderer::Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_elements: u32,
    material: usize,
}

impl renderer::GpuResource for GpuMesh {}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub material: usize,
}

impl renderer::GpuAsset for Mesh {
    type GpuType = GpuMesh;

    fn build(&self, renderer: &renderer::Renderer) -> Self::GpuType {
        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self::GpuType {
            vertex_buffer,
            index_buffer,
            num_elements: self.indices.len() as u32,
            material: self.material,
        }
    }
}

#[derive(Debug)]
pub struct RenderModel {
    meshes: Vec<GpuMesh>,
    materials: Vec<material::RenderMaterial>,
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
                texture::Texture::load(diffuse_path, texture::TextureType::Diffuse)?;

            let normal_path = containing_folder.join(mat.normal_texture);
            let normal_texture = texture::Texture::load(normal_path, texture::TextureType::Normal)?;

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
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self { meshes, materials })
    }

    pub fn build(
        &self,
        renderer: &renderer::Renderer,
        material_builder: &renderer::RenderAssetBuilder<material::Material>,
    ) -> RenderModel {
        let meshes = self
            .meshes
            .iter()
            .map(|mesh| mesh.build(renderer))
            .collect();

        let materials = self
            .materials
            .iter()
            .map(|material| material_builder.build(renderer, material))
            .collect();

        RenderModel { meshes, materials }
    }
}

#[derive(Debug)]
pub struct ModelRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub models: Vec<&'a RenderModel>,
    pub transforms: Vec<&'a transform::RenderTransform>,
    pub camera: &'a camera::RenderCamera,
}

impl<'a> renderer::RenderCommand<'a> for ModelRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        for (i, model) in self.models.iter().enumerate() {
            render_pass.draw_model(model, self.transforms[i], self.camera);
        }
    }
}

#[derive(Debug)]
pub struct ModelOutlineRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub models: Vec<&'a RenderModel>,
    pub transforms: Vec<&'a transform::RenderTransform>,
    pub camera: &'a camera::RenderCamera,
}

impl<'a> renderer::RenderCommand<'a> for ModelOutlineRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_stencil_reference(1);
        render_pass.set_pipeline(self.pipeline);
        for (i, model) in self.models.iter().enumerate() {
            render_pass.set_bind_group(0, self.transforms[i].bind_group(), &[]);
            render_pass.set_bind_group(1, self.camera.bind_group(), &[]);
            for mesh in &model.meshes {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
            }
        }
    }
}

#[derive(Debug)]
pub struct MeshRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub mesh: &'a GpuMesh,
    pub material: &'a material::RenderColorMaterial,
    pub transform: &'a transform::RenderTransform,
    pub camera: &'a camera::RenderCamera,
}

impl<'a> renderer::RenderCommand<'a> for MeshRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        render_pass.set_bind_group(0, self.material.bind_group(), &[]);
        render_pass.set_bind_group(1, self.transform.bind_group(), &[]);
        render_pass.set_bind_group(2, self.camera.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.mesh.num_elements, 0, 0..1);
    }
}

pub trait DrawModel<'a> {
    fn draw_model(
        &mut self,
        model: &'a RenderModel,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
    );

    fn draw_model_instanced(
        &mut self,
        model: &'a RenderModel,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
        instances: std::ops::Range<u32>,
    );

    fn draw_mesh(
        &mut self,
        mesh: &'a GpuMesh,
        material: &'a material::RenderMaterial,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a GpuMesh,
        material: &'a material::RenderMaterial,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
        instances: std::ops::Range<u32>,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_model(
        &mut self,
        model: &'a RenderModel,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
    ) {
        self.draw_model_instanced(model, transform, camera, 0..1);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'a RenderModel,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
        instances: std::ops::Range<u32>,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, transform, camera, instances.clone());
        }
    }

    fn draw_mesh(
        &mut self,
        mesh: &'a GpuMesh,
        material: &'a material::RenderMaterial,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
    ) {
        self.draw_mesh_instanced(mesh, material, transform, camera, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a GpuMesh,
        material: &'a material::RenderMaterial,
        transform: &'a transform::RenderTransform,
        camera: &'a camera::RenderCamera,
        instances: std::ops::Range<u32>,
    ) {
        self.set_bind_group(0, material.bind_group(), &[]);
        self.set_bind_group(1, transform.bind_group(), &[]);
        self.set_bind_group(2, camera.bind_group(), &[]);
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
