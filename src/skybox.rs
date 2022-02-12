use anyhow::{Ok, Result};
use wgpu::util::DeviceExt;

use crate::camera;
use crate::renderer::{self, GpuAsset, RenderResource};
use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyboxVertex {
    position: [f32; 3],
}

impl renderer::Vertex for SkyboxVertex {
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

#[derive(Debug)]
pub struct RenderSkybox {
    vertex_buffer: wgpu::Buffer,
    num_elements: u32,
    cube_map: texture::GpuTexture,
    bind_group: wgpu::BindGroup,
}

impl renderer::RenderResource for RenderSkybox {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct Skybox {
    pub vertices: Vec<f32>,
    pub num_elements: u32,
    pub cube_map: texture::CubeMap,
}

impl Skybox {
    pub fn load<P: AsRef<std::path::Path>>(paths: [P; 6]) -> Result<Self> {
        let cube_map = texture::CubeMap::load(paths)?;

        let vertices: Vec<f32> = vec![
            -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0,
            -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0,
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0,
            -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        Ok(Self {
            vertices,
            num_elements: 36,
            cube_map,
        })
    }
}

impl renderer::RenderAsset for Skybox {
    type RenderType = RenderSkybox;

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
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> Self::RenderType {
        let cube_map = self.cube_map.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
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

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube_map_vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self::RenderType {
            vertex_buffer,
            num_elements: self.num_elements,
            cube_map,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct SkyboxRenderCommand<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub skybox: &'a RenderSkybox,
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
    fn draw_skybox(&mut self, skybox: &'a RenderSkybox, camera: &'a camera::RenderCamera);
}

impl<'a, 'b> DrawSkybox<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_skybox(&mut self, skybox: &'a RenderSkybox, camera: &'a camera::RenderCamera) {
        self.set_vertex_buffer(0, skybox.vertex_buffer.slice(..));
        self.set_bind_group(0, &skybox.bind_group(), &[]);
        self.set_bind_group(1, &camera.bind_group(), &[]);
        self.draw(0..skybox.num_elements, 0..1);
    }
}
