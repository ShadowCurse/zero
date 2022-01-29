use anyhow::{Ok, Result};
use wgpu::util::DeviceExt;

use crate::camera;
use crate::model::Vertex;
use crate::renderer;
use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxVertex {
    position: [f32; 3],
}

impl Vertex for SkyboxVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

pub struct Skybox {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub cube_map: texture::Texture,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Skybox {
    pub fn load<P: AsRef<std::path::Path>>(
        renderer: &renderer::Renderer,
        paths: [P; 6],
    ) -> Result<Self> {
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
                                view_dimension: wgpu::TextureViewDimension::Cube,
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
                    label: Some("skybox_bind_group_layout"),
                });

        let cube_map = texture::Texture::load_cube_map(&renderer.device, &renderer.queue, paths)?;

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&cube_map.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&cube_map.sampler),
                    },
                ],
                label: None,
            });

        let skybox_vertices: Vec<f32> = vec![
            -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0,
            -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube_map_vertex_buffer"),
                contents: bytemuck::cast_slice(&skybox_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube_map_index_buffer"),
                contents: bytemuck::cast_slice(&(0..37).collect::<Vec<_>>()),
                usage: wgpu::BufferUsages::INDEX,
            });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            num_elements: 36,
            cube_map,
            bind_group,
            bind_group_layout,
        })
    }
}

pub struct SkyboxRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub skybox: &'a Skybox,
    pub camera: &'a camera::RenderCamera,
}

impl<'a> renderer::RenderCommand<'a> for SkyboxRenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b,
    {
        render_pass.set_pipeline(self.pipeline);
        render_pass.draw_skybox(self.skybox, self.camera);
    }
}

pub trait DrawSkybox<'a> {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera: &'a camera::RenderCamera);
}

impl<'a, 'b> DrawSkybox<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera: &'a camera::RenderCamera) {
        self.set_vertex_buffer(0, skybox.vertex_buffer.slice(..));
        self.set_index_buffer(skybox.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &skybox.bind_group, &[]);
        self.set_bind_group(1, &camera.bind_group, &[]);
        self.draw_indexed(0..skybox.num_elements, 0, 0..1);
    }
}
