use crate::renderer::{CurrentFrameContext, RenderCommand};
use crate::texture::GpuTexture;
use wgpu::CommandEncoder;

pub trait RenderPhase {
    fn color_attachments<'s, 'c>(
        &'s self,
        _frame_context: &'c CurrentFrameContext,
    ) -> Vec<wgpu::RenderPassColorAttachment<'c>>
    where
        's: 'c,
    {
        vec![]
    }

    fn depth_stencil_attachment(&self) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        None
    }

    fn commands(&self) -> &[&dyn RenderCommand];

    //fn clear(&mut self);
}

pub fn execute_phase(
    name: Option<&str>,
    encoder: &mut CommandEncoder,
    context: &dyn RenderPhase,
    frame_context: &CurrentFrameContext,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: name,
        color_attachments: &context.color_attachments(frame_context),
        depth_stencil_attachment: context.depth_stencil_attachment(),
    });

    for command in context.commands() {
        command.execute(&mut render_pass);
    }
}

pub struct OutputRenderPhaseContext<'a, 'b, C: RenderCommand<'b>>
where
    'a: 'b,
{
    depth: &'a GpuTexture,
    commands: Vec<C>,
    ref_commands: Vec<&'a dyn RenderCommand<'b>>,
    _phantom: std::marker::PhantomData<(&'a C, &'b C)>,
}

impl<'a, 'b, C: RenderCommand<'b>> OutputRenderPhaseContext<'a, 'b, C>
where
    'a: 'b,
{
    pub fn new(depth: &'a GpuTexture) -> Self {
        Self {
            depth,
            commands: Vec::new(),
            ref_commands: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn add_command(&'a mut self, command: C) {
        self.commands.push(command);
    }
}

impl<'a, 'b, C: RenderCommand<'b>> RenderPhase for OutputRenderPhaseContext<'a, 'b, C>
where
    'a: 'b,
{
    fn color_attachments<'s, 'c>(
        &'s self,
        frame_context: &'c CurrentFrameContext,
    ) -> Vec<wgpu::RenderPassColorAttachment<'c>>
    where
        's: 'c,
    {
        vec![wgpu::RenderPassColorAttachment {
            view: &frame_context.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: true,
            },
        }]
    }

    fn depth_stencil_attachment(&self) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        None
    }

    fn commands(&self) -> &[&'a dyn RenderCommand] {
        // TODO think about this self referencing
        // self.ref_commands = self.commands.iter().map(|c| c as &'a dyn RenderCommand<'a>).collect();
        &self.ref_commands
    }
}
