use wgpu::util::DeviceExt;

use crate::renderer::{self, RenderResource};
use crate::texture;
use crate::light;
use crate::camera;

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
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
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

#[derive(Debug)]
pub struct RenderDefferedPass {
    pub position_texture: texture::GpuTexture,
    pub normal_texture: texture::GpuTexture,
    pub albedo_texture: texture::GpuTexture,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderDefferedPass {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct DefferedPassTextures<T>
where
    T: renderer::GpuAsset<GpuType = texture::GpuTexture>,
{
    pub position_texture: T,
    pub normal_texture: T,
    pub albedo_texture: T,
}

impl<T> renderer::RenderAsset for DefferedPassTextures<T>
where
    T: renderer::GpuAsset<GpuType = texture::GpuTexture>,
{
    type RenderType = RenderDefferedPass;

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
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("deffered_pass_bind_group_layout"),
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

        let position_texture = self.position_texture.build(renderer);
        let normal_texture = self.normal_texture.build(renderer);
        let albedo_texture = self.albedo_texture.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&position_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&position_texture.sampler),
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
                        resource: wgpu::BindingResource::TextureView(&albedo_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&albedo_texture.sampler),
                    },
                ],
                label: None,
            });

        Self::RenderType {
            position_texture,
            normal_texture,
            albedo_texture,
            vertex_buffer,
            index_buffer,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct DefferedPassRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub deffered_pass: &'a RenderDefferedPass,
    pub lights: &'a light::RenderLights,
    pub camera: &'a camera::RenderCamera,
}

impl<'a> renderer::RenderCommand<'a> for DefferedPassRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        render_pass.set_bind_group(0, &self.deffered_pass.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lights.bind_group(), &[]);
        render_pass.set_bind_group(2, &self.camera.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.deffered_pass.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.deffered_pass.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}
