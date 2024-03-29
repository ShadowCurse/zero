use std::ops::Range;

use crate::cgmath_imports::*;
use crate::prelude::ConstVec;
use crate::render::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl From<([f32; 3], [f32; 2], [f32; 3])> for MeshVertex {
    fn from(data: ([f32; 3], [f32; 2], [f32; 3])) -> Self {
        Self {
            position: data.0,
            tex_coords: data.1,
            normal: data.2,
            ..Default::default()
        }
    }
}

impl MeshVertex {
    pub fn calc_tangents_and_bitangents(vertices: &mut [MeshVertex], indices: &[u32]) {
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

impl VertexLayout for MeshVertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
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

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl GpuResource for Mesh {
    type ResourceType = GpuMesh;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let vertex_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: BufferUsages::INDEX,
        });

        Self::ResourceType {
            vertex_buffer,
            index_buffer: Some(index_buffer),
            num_elements: self.indices.len() as u32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshRenderCommand {
    pub pipeline_id: ResourceId,
    pub mesh_id: ResourceId,
    pub index_slice: Option<Range<u64>>,
    pub vertex_slice: Option<Range<u64>>,
    pub scissor_rect: Option<[u32; 4]>,
    pub bind_groups: ConstVec<MAX_BIND_GROUPS, ResourceId>,
}

impl RenderCommand for MeshRenderCommand {
    fn execute<'a>(&self, render_pass: &mut RenderPass<'a>, storage: &'a CurrentFrameStorage) {
        render_pass.set_pipeline(storage.get_pipeline(self.pipeline_id));
        for (i, bg) in self.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(i as u32, storage.get_bind_group(*bg), &[]);
        }

        if let Some(scissor_rect) = self.scissor_rect {
            render_pass.set_scissor_rect(
                scissor_rect[0],
                scissor_rect[1],
                scissor_rect[2],
                scissor_rect[3],
            )
        }

        let mesh = storage.get_mesh(self.mesh_id);

        if let Some(vertex_slice) = &self.vertex_slice {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(vertex_slice.clone()));
        } else {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        }

        if let Some(index_buffer) = &mesh.index_buffer {
            if let Some(index_slice) = &self.index_slice {
                render_pass
                    .set_index_buffer(index_buffer.slice(index_slice.clone()), IndexFormat::Uint32);
                // slice is in bytes so we divide by size of u32 to get
                // number of actual indices
                let s = (index_slice.end - index_slice.start) as u32;
                let s = s / std::mem::size_of::<u32>() as u32;
                render_pass.draw_indexed(0..s, 0, 0..1);
            } else {
                render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
            }
        } else {
            render_pass.draw(0..mesh.num_elements, 0..1);
        }
    }
}
