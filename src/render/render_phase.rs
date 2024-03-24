use super::renderer::MAX_COLOR_ATTACHMENTS;
use super::storage::CurrentFrameStorage;
use super::{storage::ResourceId, wgpu_imports::*};
use crate::utils::ConstVec;

#[derive(Debug)]
pub struct ColorAttachment {
    pub view_id: ResourceId,
    pub ops: Operations<Color>,
}

#[derive(Debug)]
pub struct DepthStencil {
    pub view_id: ResourceId,
    pub depth_ops: Option<Operations<f32>>,
    pub stencil_ops: Option<Operations<u32>>,
}

#[derive(Debug, Default)]
pub struct RenderPhase {
    color_attachments: ConstVec<MAX_COLOR_ATTACHMENTS, ColorAttachment>,
    depth_stencil: Option<DepthStencil>,
}

impl RenderPhase {
    pub fn new(
        color_attachments: ConstVec<MAX_COLOR_ATTACHMENTS, ColorAttachment>,
        depth_stencil: Option<DepthStencil>,
    ) -> Self {
        Self {
            color_attachments,
            depth_stencil,
        }
    }

    pub fn render_pass<'a>(
        &self,
        encoder: &'a mut CommandEncoder,
        current_frame_storage: &'a CurrentFrameStorage,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &self.color_attachments(current_frame_storage),
            depth_stencil_attachment: self.depth_stencil_attachment(current_frame_storage),
            ..Default::default()
        })
    }
}

impl RenderPhase {
    fn color_attachments<'a>(
        &self,
        storage: &'a CurrentFrameStorage,
    ) -> Vec<Option<RenderPassColorAttachment<'a>>> {
        self.color_attachments
            .iter()
            .map(|attachment| {
                let view = storage.get_view(attachment.view_id);
                Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: attachment.ops,
                })
            })
            .collect()
    }

    fn depth_stencil_attachment<'a>(
        &self,
        storage: &'a CurrentFrameStorage,
    ) -> Option<RenderPassDepthStencilAttachment<'a>> {
        self.depth_stencil.as_ref().map(|depth_stencil| {
            let view = storage.get_view(depth_stencil.view_id);
            RenderPassDepthStencilAttachment {
                view,
                depth_ops: depth_stencil.depth_ops,
                stencil_ops: depth_stencil.stencil_ops,
            }
        })
    }
}
