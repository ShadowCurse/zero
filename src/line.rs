use crate::mesh::GpuMesh;
use crate::prelude::ConstVec;
use crate::render::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position_a: [f32; 3],
    pub position_b: [f32; 3],
    pub color_a: [f32; 4],
    pub color_b: [f32; 4],
}

impl VertexLayout for LineVertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 10]>() as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct Line {
    pub vertices: Vec<LineVertex>,
}

impl GpuResource for Line {
    type ResourceType = GpuMesh;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let vertex_buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: BufferUsages::VERTEX,
        });

        Self::ResourceType {
            vertex_buffer,
            index_buffer: None,
            num_elements: self.vertices.len() as u32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineRenderCommand {
    pub pipeline_id: ResourceId,
    pub mesh_id: ResourceId,
    pub bind_groups: ConstVec<MAX_BIND_GROUPS, ResourceId>,
}

impl RenderCommand for LineRenderCommand {
    fn execute<'a>(&self, render_pass: &mut RenderPass<'a>, storage: &'a CurrentFrameStorage) {
        render_pass.set_pipeline(storage.get_pipeline(self.pipeline_id));
        for (i, bg) in self.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(i as u32, storage.get_bind_group(*bg), &[]);
        }

        let mesh = storage.get_mesh(self.mesh_id);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        render_pass.draw(0..6, 0..mesh.num_elements);
    }
}
