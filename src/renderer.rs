use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use winit::window::Window;

use crate::texture;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub trait RenderCommand<'a> {
    fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
    where
        'a: 'b;
}

/// Trait for resources located on the GPU
pub trait GpuResource {}

/// Trait for types that can be loaded to the GPU
pub trait GpuAsset {
    type GpuType: GpuResource;

    fn build(&self, renderer: &Renderer) -> Self::GpuType;
}

/// Trait for types that compose Gpu resources into bind group
pub trait RenderResource {
    fn bind_group(&self) -> &wgpu::BindGroup;
}

/// Trait for the types that can be converted to the RenderResource
pub trait RenderAsset {
    type RenderType: RenderResource;

    fn bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout;
    fn build(&self, renderer: &Renderer, layout: &wgpu::BindGroupLayout) -> Self::RenderType;
    fn update(&self, _renderer: &Renderer, _render_type: &Self::RenderType) {}
}

/// Builder for objects with the same bind_group_layout
#[derive(Debug)]
pub struct RenderAssetBuilder<T: RenderAsset> {
    pub bind_group_layout: wgpu::BindGroupLayout,
    _phantom: std::marker::PhantomData<fn() -> T>,
}

impl<T: RenderAsset> RenderAssetBuilder<T> {
    pub fn new(renderer: &Renderer) -> Self {
        Self {
            bind_group_layout: T::bind_group_layout(renderer),
            _phantom: std::marker::PhantomData::default(),
        }
    }

    pub fn build(&self, renderer: &Renderer, resource: &T) -> T::RenderType {
        resource.build(renderer, &self.bind_group_layout)
    }
}

#[derive(Debug)]
pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    pub fn resize(&mut self, new_size: Option<winit::dpi::PhysicalSize<u32>>) {
        let new_size = new_size.unwrap_or(self.size);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(
        &mut self,
        commands: &Vec<&dyn RenderCommand>,
        depth_texture: &texture::GpuTexture,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: true,
                    }),
                }),
            });

            for command in commands {
                command.execute(&mut render_pass);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn create_render_pipeline<P: AsRef<Path>>(
        &mut self,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader_path: P,
        write_depth: bool,
        mask: u32,
        comp: wgpu::CompareFunction,
    ) -> wgpu::RenderPipeline {
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_descriptor"),
                bind_group_layouts,
                push_constant_ranges: &[],
            });

        let mut contents = String::new();
        let mut file = File::open(shader_path.as_ref()).unwrap();
        file.read_to_string(&mut contents).unwrap();

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(contents.into()),
        };

        let shader = self.device.create_shader_module(&shader);

        let stencil_state = wgpu::StencilFaceState {
            compare: comp,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::IncrementClamp,
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: vertex_layouts,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: self.config.format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::DepthTexture::DEPTH_FORMAT,
                    depth_write_enabled: write_depth,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState {
                        front: stencil_state,
                        back: stencil_state,
                        read_mask: 0xff,
                        write_mask: mask,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}

#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    pub bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    pub vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    pub shader_path: String,
    pub stencil_compare: wgpu::CompareFunction,
    pub stencil_read_mask: u32,
    pub stencil_write_mask: u32,
    pub write_depth: bool,
}

impl<'a> std::default::Default for PipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            bind_group_layouts: Vec::new(),
            vertex_layouts: Vec::new(),
            shader_path: "".to_string(),
            stencil_compare: wgpu::CompareFunction::Always,
            stencil_read_mask: 0x00,
            stencil_write_mask: 0x00,
            write_depth: true,
        }
    }
}

impl<'a> PipelineBuilder<'a> {
    pub fn new<P: Into<String>>(
        bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
        vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
        shader_path: P,
    ) -> Self {
        Self {
            bind_group_layouts,
            vertex_layouts,
            shader_path: shader_path.into(),
            stencil_compare: wgpu::CompareFunction::Always,
            ..Default::default()
        }
    }

    pub fn stencil_compare(mut self, stencil_compare: wgpu::CompareFunction) -> Self {
        self.stencil_compare = stencil_compare;
        self
    }

    pub fn stencil_read_mask(mut self, stencil_read_mask: u32) -> Self {
        self.stencil_read_mask = stencil_read_mask;
        self
    }

    pub fn stencil_write_mask(mut self, stencil_write_mask: u32) -> Self {
        self.stencil_write_mask = stencil_write_mask;
        self
    }

    pub fn write_depth(mut self, write_depth: bool) -> Self {
        self.write_depth = write_depth;
        self
    }

    pub fn build(self, renderer: &Renderer) -> wgpu::RenderPipeline {
        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_descriptor"),
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let mut contents = String::new();
        let mut file = File::open(self.shader_path).unwrap();
        file.read_to_string(&mut contents).unwrap();

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(contents.into()),
        };

        let shader = renderer.device.create_shader_module(&shader);

        let stencil_state = wgpu::StencilFaceState {
            compare: self.stencil_compare,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::IncrementClamp,
        };

        renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &self.vertex_layouts,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: renderer.config.format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::DepthTexture::DEPTH_FORMAT,
                    depth_write_enabled: self.write_depth,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState {
                        front: stencil_state,
                        back: stencil_state,
                        read_mask: self.stencil_read_mask,
                        write_mask: self.stencil_write_mask,
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
