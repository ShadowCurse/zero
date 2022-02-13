use wgpu::util::DeviceExt;

use crate::renderer;
use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl From<([f32; 3], [f32; 2])> for Vertex {
    fn from(data: ([f32; 3], [f32; 2])) -> Self {
        Self {
            position: data.0,
            tex_coords: data.1,
        }
    }
}

impl renderer::Vertex for Vertex {
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
            ],
        }
    }
}

#[derive(Debug)]
pub struct RenderPresentTexture {
    pub texture: texture::GpuTexture,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderPresentTexture {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct PresentTexture<T>
where
    T: renderer::GpuAsset<GpuType = texture::GpuTexture>,
{
    pub texture: T,
}

impl<T> renderer::RenderAsset for PresentTexture<T>
where
    T: renderer::GpuAsset<GpuType = texture::GpuTexture>,
{
    type RenderType = RenderPresentTexture;

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
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
                ],
                label: Some("screen_quad_bind_group_layout"),
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::RenderType {
        let vertices: Vec<Vertex> = vec![
            ([-1.0, 1.0, 0.0], [0.0, 0.0]),
            ([-1.0, -1.0, 0.0], [0.0, 1.0]),
            ([1.0, 1.0, 0.0], [1.0, 0.0]),
            ([1.0, -1.0, 0.0], [1.0, 1.0]),
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let indices = vec![0, 1, 2, 2, 1, 3];

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let texture = self.texture.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: None,
            });

        Self::RenderType {
            texture,
            vertex_buffer,
            index_buffer,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct PresentTextureRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub screen_quad: &'a RenderPresentTexture,
}

impl<'a> renderer::RenderCommand<'a> for PresentTextureRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        render_pass.set_bind_group(0, &self.screen_quad.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.screen_quad.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.screen_quad.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}
