use super::{renderer::Renderer, wgpu_imports::*};
use log::info;
use std::{fs::File, io::Read, num::NonZeroU32};

pub struct PipelineBuilder<'a> {
    pub shader_path: &'a str,
    pub label: Option<&'a str>,
    pub layout_descriptor: Option<&'a PipelineLayoutDescriptor<'a>>,
    pub vertex_layouts: &'a [VertexBufferLayout<'a>],
    pub vertex_entry_point: &'a str,
    pub color_targets: Option<&'a [Option<ColorTargetState>]>,
    pub fragment_entry_point: &'a str,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub multiview: Option<NonZeroU32>,
}

impl<'a> PipelineBuilder<'a> {
    pub fn build(self, renderer: &Renderer) -> RenderPipeline {
        info!("Building pipilene: {}", self.shader_path);

        let layout = self
            .layout_descriptor
            .map(|d| renderer.device().create_pipeline_layout(d));

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

        let fragment = self.color_targets.map(|targets| FragmentState {
            module: &shader,
            entry_point: self.fragment_entry_point,
            targets,
        });

        renderer
            .device()
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: self.label,
                layout: layout.as_ref(),
                vertex: VertexState {
                    module: &shader,
                    entry_point: self.vertex_entry_point,
                    buffers: self.vertex_layouts,
                },
                fragment,
                primitive: self.primitive,
                depth_stencil: self.depth_stencil,
                multisample: self.multisample,
                multiview: self.multiview,
            })
    }
}
