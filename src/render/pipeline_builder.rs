use super::{renderer::Renderer, wgpu_imports::*};
use crate::texture::DepthTexture;
use log::info;
use std::{fs::File, io::Read};

/// Builder for the pipelines
#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    pub bind_group_layouts: Vec<&'a BindGroupLayout>,
    pub vertex_layouts: Vec<VertexBufferLayout<'a>>,
    pub shader_path: &'a str,
    pub primitive_topology: PrimitiveTopology,
    pub depth_enabled: bool,
    pub stencil_enabled: bool,
    pub stencil_compare: CompareFunction,
    pub stencil_read_mask: u32,
    pub stencil_write_mask: u32,
    pub write_depth: bool,
    pub color_targets: Option<Vec<TextureFormat>>,
    pub cull_mode: Face,
}

impl<'a> std::default::Default for PipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            bind_group_layouts: vec![],
            vertex_layouts: vec![],
            shader_path: "",
            primitive_topology: PrimitiveTopology::TriangleList,
            depth_enabled: true,
            stencil_enabled: false,
            stencil_compare: CompareFunction::Always,
            stencil_read_mask: 0x00,
            stencil_write_mask: 0x00,
            write_depth: true,
            color_targets: None,
            cull_mode: Face::Back,
        }
    }
}

impl<'a> PipelineBuilder<'a> {
    pub fn build(self, renderer: &Renderer) -> RenderPipeline {
        info!("Building pipilene: {}", self.shader_path);
        let layout = renderer
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("render_pipeline_descriptor"),
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let mut contents = String::new();
        {
            let mut file = File::open(self.shader_path).unwrap();
            file.read_to_string(&mut contents).unwrap();
        }

        let shader_label = format!("shader: {}", self.shader_path);
        let shader = ShaderModuleDescriptor {
            label: Some(&shader_label),
            source: ShaderSource::Wgsl(contents.into()),
        };
        let shader = renderer.device().create_shader_module(shader);

        let targets = match self.color_targets {
            Some(ct) => ct
                .into_iter()
                .map(|ct| {
                    Some(ColorTargetState {
                        format: ct,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })
                })
                .collect(),
            None => vec![Some(ColorTargetState {
                format: renderer.surface_format(),
                blend: Some(BlendState {
                    alpha: BlendComponent::REPLACE,
                    color: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            })],
        };

        let strip_index_format = match self.primitive_topology {
            PrimitiveTopology::TriangleList => None,
            PrimitiveTopology::TriangleStrip => Some(IndexFormat::Uint32),
            _ => unimplemented!(),
        };

        let depth_stencil = if self.depth_enabled {
            let stencil = if self.stencil_enabled {
                let stencil_state = StencilFaceState {
                    compare: self.stencil_compare,
                    fail_op: StencilOperation::Keep,
                    depth_fail_op: StencilOperation::Keep,
                    pass_op: StencilOperation::IncrementClamp,
                };

                StencilState {
                    front: stencil_state,
                    back: stencil_state,
                    read_mask: self.stencil_read_mask,
                    write_mask: self.stencil_write_mask,
                }
            } else {
                StencilState::default()
            };

            Some(DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: self.write_depth,
                depth_compare: CompareFunction::LessEqual,
                stencil,
                bias: DepthBiasState::default(),
            })
        } else {
            None
        };

        renderer
            .device()
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(self.shader_path),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &self.vertex_layouts,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &targets,
                }),
                primitive: PrimitiveState {
                    topology: self.primitive_topology,
                    strip_index_format,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(self.cull_mode),
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
