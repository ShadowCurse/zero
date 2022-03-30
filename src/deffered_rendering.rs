use crate::render_phase::IndexType;
use crate::render_phase::RenderResources;
use crate::renderer::{GpuAsset, RenderAsset, Renderer};
use crate::texture::{GpuTexture, TextureVertex};
use wgpu::util::DeviceExt;

// #[derive(Debug)]
// pub struct RenderGBuffer {
//     pub position: GpuTexture,
//     pub normal: GpuTexture,
//     pub albedo: GpuTexture,
//     vertex_buffer: wgpu::Buffer,
//     index_buffer: wgpu::Buffer,
//     bind_group: wgpu::BindGroup,
// }
//
// impl RenderGBuffer {
//     pub fn color_attachments(&self) -> Vec<wgpu::RenderPassColorAttachment> {
//         vec![
//             self.position.color_attachment(),
//             self.normal.color_attachment(),
//             self.albedo.color_attachment(),
//         ]
//     }
// }
//
// impl RenderResource for RenderGBuffer {
//     fn bind_group(&self) -> &wgpu::BindGroup {
//         &self.bind_group
//     }
// }

#[derive(Debug)]
pub struct GBufferTexture {
    pub format: wgpu::TextureFormat,
}

impl GBufferTexture {
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self { format }
    }
}

impl GpuAsset for GBufferTexture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::GpuType {
        let texture_size = wgpu::Extent3d {
            width: renderer.config.width,
            height: renderer.config.height,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("gbuffer_texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug)]
pub struct GBuffer {
    pub position: GBufferTexture,
    pub normal: GBufferTexture,
    pub albedo: GBufferTexture,
}

impl GBuffer {
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            position: GBufferTexture::new(format),
            normal: GBufferTexture::new(format),
            albedo: GBufferTexture::new(format),
        }
    }
}

impl RenderAsset for GBuffer {
    const ASSET_NAME: &'static str = "GBuffer";

    fn bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout {
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

    fn build(&self, renderer: &Renderer, layout: &wgpu::BindGroupLayout) -> RenderResources {
        let vertices: Vec<TextureVertex> = vec![
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

        let position = self.position.build(renderer);
        let normal = self.normal.build(renderer);
        let albedo = self.albedo.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&position.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&position.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&normal.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&albedo.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&albedo.sampler),
                    },
                ],
                label: None,
            });

        RenderResources {
            textures: vec![position, normal, albedo],
            vertex_buffer: Some(vertex_buffer),
            index_type: Some(IndexType::Buffer(index_buffer)),
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }
}

// #[derive(Debug)]
// pub struct DefferedPassRenderCommand<'a> {
//     pub pipeline: &'a wgpu::RenderPipeline,
//     pub g_buffer: &'a RenderGBuffer,
//     pub shadow_map: &'a shadow_map::RenderShadowMap,
//     pub lights: &'a light::RenderLights,
//     pub camera: &'a camera::RenderCamera,
// }
//
// impl<'a> RenderCommand<'a> for DefferedPassRenderCommand<'a> {
//     fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
//     where
//         'a: 'b,
//     {
//         render_pass.set_pipeline(self.pipeline);
//         render_pass.set_bind_group(0, self.g_buffer.bind_group(), &[]);
//         render_pass.set_bind_group(1, self.lights.bind_group(), &[]);
//         render_pass.set_bind_group(2, self.camera.bind_group(), &[]);
//         render_pass.set_bind_group(3, self.shadow_map.bind_group(), &[]);
//         render_pass.set_vertex_buffer(0, self.g_buffer.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(
//             self.g_buffer.index_buffer.slice(..),
//             wgpu::IndexFormat::Uint32,
//         );
//         render_pass.draw_indexed(0..6, 0, 0..1);
//     }
// }
