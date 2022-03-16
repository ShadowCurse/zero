use std::fs::File;
use std::io::prelude::*;
use winit::window::Window;

use crate::{deffered_rendering, texture};

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

    pub fn forward_render(
        &mut self,
        commands: &[&dyn RenderCommand],
        post_commands: Option<&[&dyn RenderCommand]>,
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for command in commands {
                command.execute(&mut render_pass);
            }
        }

        if let Some(commands) = post_commands {
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("render_pass"),
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.3,
                                g: 0.3,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                for command in commands {
                    command.execute(&mut render_pass);
                }
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn deferred_render(
        &mut self,
        geometry_pass_commands: &[&dyn RenderCommand],
        lighting_pass_commands: &[&dyn RenderCommand],
        forward_pass_commands: Option<&[&dyn RenderCommand]>,
        g_buffer: &deffered_rendering::RenderGBuffer,
        depth_texture: &texture::GpuTexture,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // geometry_pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("geometry_render_pass"),
                color_attachments: &g_buffer.color_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for command in geometry_pass_commands {
                command.execute(&mut render_pass);
            }
        }

        // lighting pass
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lighting_render_pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.3,
                            g: 0.3,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            for command in lighting_pass_commands {
                command.execute(&mut render_pass);
            }
        }

        // forward pass
        if let Some(commands) = forward_pass_commands {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("forward_render_pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
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
}

#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    shader_path: String,
    primitive_topology: wgpu::PrimitiveTopology,
    depth_enabled: bool,
    stencil_enabled: bool,
    stencil_compare: wgpu::CompareFunction,
    stencil_read_mask: u32,
    stencil_write_mask: u32,
    write_depth: bool,
    color_targets: Option<Vec<wgpu::TextureFormat>>,
}

impl<'a> std::default::Default for PipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            bind_group_layouts: Vec::new(),
            vertex_layouts: Vec::new(),
            shader_path: "".to_string(),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_enabled: true,
            stencil_enabled: false,
            stencil_compare: wgpu::CompareFunction::Always,
            stencil_read_mask: 0x00,
            stencil_write_mask: 0x00,
            write_depth: true,
            color_targets: None,
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

    pub fn primitive_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.primitive_topology = topology;
        self
    }

    pub fn depth_enabled(mut self, enabled: bool) -> Self {
        self.depth_enabled = enabled;
        self
    }

    pub fn stencil_enabled(mut self, enabled: bool) -> Self {
        self.stencil_enabled = enabled;
        self
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

    pub fn color_targets(mut self, color_targets: Vec<wgpu::TextureFormat>) -> Self {
        self.color_targets = Some(color_targets);
        self
    }

    pub fn build(self, renderer: &Renderer) -> wgpu::RenderPipeline {
        println!("building pipilene: {}", self.shader_path);
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

        let targets = if let Some(color_targets) = self.color_targets {
            color_targets
                .into_iter()
                .map(|ct| wgpu::ColorTargetState {
                    format: ct,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
                .collect()
        } else {
            vec![wgpu::ColorTargetState {
                format: renderer.config.format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }]
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
                    targets: &targets,
                }),
                primitive: wgpu::PrimitiveState {
                    topology: self.primitive_topology,
                    strip_index_format: match self.primitive_topology {
                        wgpu::PrimitiveTopology::TriangleList => None,
                        wgpu::PrimitiveTopology::TriangleStrip => Some(wgpu::IndexFormat::Uint32),
                        _ => unimplemented!(),
                    },
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: if self.depth_enabled {
                    Some(wgpu::DepthStencilState {
                        format: texture::DepthTexture::DEPTH_FORMAT,
                        depth_write_enabled: self.write_depth,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: if self.stencil_enabled {
                            let stencil_state = wgpu::StencilFaceState {
                                compare: self.stencil_compare,
                                fail_op: wgpu::StencilOperation::Keep,
                                depth_fail_op: wgpu::StencilOperation::Keep,
                                pass_op: wgpu::StencilOperation::IncrementClamp,
                            };

                            wgpu::StencilState {
                                front: stencil_state,
                                back: stencil_state,
                                read_mask: self.stencil_read_mask,
                                write_mask: self.stencil_write_mask,
                            }
                        } else {
                            wgpu::StencilState::default()
                        },
                        bias: wgpu::DepthBiasState::default(),
                    })
                } else {
                    None
                },
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
