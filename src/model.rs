use anyhow::{Context, Ok, Result};
use cgmath::InnerSpace;
use wgpu::util::DeviceExt;

use crate::camera;
use crate::light;
use crate::renderer;
use crate::texture;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
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

pub struct Transform {
    pub translation: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,
}

impl Transform {
    pub fn to_uniform(&self) -> TransformUniform {
        let rotate = cgmath::Matrix4::from(self.rotation);
        TransformUniform {
            transform: (cgmath::Matrix4::from_translation(self.translation)
                * rotate
                * cgmath::Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z))
            .into(),
            rotate: rotate.into(),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    rotate: [[f32; 4]; 4],
}

impl TransformUniform {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct RenderTransform {
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl RenderTransform {
    pub fn new(renderer: &renderer::Renderer, transform: &Transform) -> Self {
        let uniform = transform.to_uniform();
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("texture_bind_group_layout"),
                });
        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("transform_bind_group"),
            });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn update(&mut self, renderer: &renderer::Renderer, transform: &Transform) {
        renderer.queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[transform.to_uniform()]),
        );
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub struct ColorMaterial {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl ColorMaterial {
    pub fn new(
        renderer: &renderer::Renderer,
        ambient: [f32; 3],
        diffuse: [f32; 3],
        specular: [f32; 3],
        shininess: f32,
    ) -> Self {
        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("texture_bind_group_layout"),
                });

        let properties = MaterialPropertiesUniform {
            ambient,
            diffuse,
            specular,
            shininess,
            ..Default::default()
        };

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material_params_buffer"),
                contents: bytemuck::cast_slice(&[properties]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: None,
            });

        Self {
            ambient,
            diffuse,
            specular,
            shininess,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialPropertiesUniform {
    pub ambient: [f32; 3],
    _pad1: f32,
    pub diffuse: [f32; 3],
    _pad2: f32,
    pub specular: [f32; 3],
    _pad3: f32,
    pub shininess: f32,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Model {
    pub fn load<P: AsRef<std::path::Path>>(renderer: &renderer::Renderer, path: P) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let bind_group_layout =
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
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let obj_materials = obj_materials?;

        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = containing_folder.join(mat.diffuse_texture);
            let diffuse_texture = texture::Texture::load(
                &renderer.device,
                &renderer.queue,
                diffuse_path,
                texture::TextureType::Diffuse,
            )?;

            let normal_path = containing_folder.join(mat.normal_texture);
            let normal_texture = texture::Texture::load(
                &renderer.device,
                &renderer.queue,
                normal_path,
                texture::TextureType::Normal,
            )?;

            let properties = MaterialPropertiesUniform {
                ambient: mat.ambient,
                diffuse: mat.diffuse,
                specular: mat.specular,
                shininess: mat.shininess,
                ..Default::default()
            };

            let buffer = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("material_params_buffer"),
                    contents: bytemuck::cast_slice(&[properties]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

            let bind_group = renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: buffer.as_entire_binding(),
                        },
                    ],
                    label: None,
                });

            materials.push(Material {
                name: mat.name,
                diffuse_texture,
                normal_texture,
                ambient: mat.ambient,
                diffuse: mat.diffuse,
                specular: mat.specular,
                shininess: mat.shininess,
                buffer,
                bind_group,
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

            for c in m.mesh.indices.chunks(3) {
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

            for v in &mut vertices {
                v.tangent = cgmath::Vector3::from(v.tangent).normalize().into();
                v.bitangent = cgmath::Vector3::from(v.bitangent).normalize().into();
            }

            let vertex_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            let index_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                        contents: bytemuck::cast_slice(&m.mesh.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self {
            meshes,
            materials,
            bind_group_layout,
        })
    }
}

pub struct ModelRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub models: Vec<&'a Model>,
    pub transforms: Vec<&'a RenderTransform>,
    pub camera: &'a camera::RenderCamera,
    pub light: &'a light::RenderLight,
}

impl<'a> renderer::RenderCommand<'a> for ModelRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        for (i, model) in self.models.iter().enumerate() {
            render_pass.draw_model(model, self.transforms[i], self.camera, self.light);
        }
    }
}

pub struct MeshRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub mesh: &'a Mesh,
    pub material: &'a ColorMaterial,
    pub transform: &'a RenderTransform,
    pub camera: &'a camera::RenderCamera,
    pub light: &'a light::RenderLight,
}

impl<'a> renderer::RenderCommand<'a> for MeshRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        // render_pass.draw_mesh(
        //     self.mesh,
        //     self.material,
        //     self.transform,
        //     self.camera,
        //     self.light,
        // );
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.material.bind_group, &[]);
        render_pass.set_bind_group(1, &self.transform.bind_group, &[]);
        render_pass.set_bind_group(2, &self.camera.bind_group, &[]);
        render_pass.set_bind_group(3, &self.light.bind_group, &[]);
        render_pass.draw_indexed(0..self.mesh.num_elements, 0, 0..1);
    }
}

pub trait DrawModel<'a> {
    fn draw_model(
        &mut self,
        model: &'a Model,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
    );

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
        instances: std::ops::Range<u32>,
    );

    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
        instances: std::ops::Range<u32>,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_model(
        &mut self,
        model: &'a Model,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
    ) {
        self.draw_model_instanced(model, transform, camera, light, 0..1);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
        instances: std::ops::Range<u32>,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, transform, camera, light, instances.clone());
        }
    }

    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
    ) {
        self.draw_mesh_instanced(mesh, material, transform, camera, light, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        transform: &'a RenderTransform,
        camera: &'a camera::RenderCamera,
        light: &'a light::RenderLight,
        instances: std::ops::Range<u32>,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &transform.bind_group, &[]);
        self.set_bind_group(2, &camera.bind_group, &[]);
        self.set_bind_group(3, &light.bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
