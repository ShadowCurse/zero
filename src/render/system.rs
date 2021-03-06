use super::renderer::{CurrentFrameContext, Renderer};
use super::{
    storage::{RenderStorage, ResourceId},
    wgpu_imports::*,
};
use std::{borrow::Cow, collections::HashMap, ops::Deref};

#[derive(Debug)]
pub struct BindGroupMeta {
    pub index: u32,
    pub bind_group_id: ResourceId,
}

#[derive(Debug)]
pub struct RenderCommand {
    pub pipeline_id: ResourceId,
    pub mesh_id: ResourceId,
    pub bind_groups: Vec<BindGroupMeta>,
}

impl RenderCommand {
    pub fn new(pipeline_id: ResourceId, mesh_id: ResourceId, bind_groups: Vec<ResourceId>) -> Self {
        let bind_groups = bind_groups
            .into_iter()
            .enumerate()
            .map(|(i, bind_group_id)| BindGroupMeta {
                index: i as u32,
                bind_group_id,
            })
            .collect();
        Self {
            pipeline_id,
            mesh_id,
            bind_groups,
        }
    }
    fn execute<'a>(&self, render_pass: &mut RenderPass<'a>, storage: &'a CurrentFrameStorage) {
        let mesh = storage.get_mesh(self.mesh_id);
        let bind_groups: Vec<_> = self
            .bind_groups
            .iter()
            .map(|meta| (meta.index, storage.get_bind_group(meta.bind_group_id)))
            .collect();
        let pipeline = storage.get_pipeline(self.pipeline_id);
        render_pass.set_pipeline(pipeline);
        for bg in bind_groups.iter() {
            render_pass.set_bind_group(bg.0, bg.1, &[]);
        }
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        if mesh.index_buffer.is_some() {
            render_pass.set_index_buffer(
                mesh.index_buffer.as_ref().unwrap().slice(..),
                IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        } else {
            render_pass.draw(0..mesh.num_elements, 0..1);
        }
    }
}

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

#[derive(Debug)]
pub struct RenderPhase {
    color_attachments: Vec<ColorAttachment>,
    depth_stencil: Option<DepthStencil>,
    commands: Vec<RenderCommand>,
}

impl RenderPhase {
    pub fn new(
        color_attachments: Vec<ColorAttachment>,
        depth_stencil: Option<DepthStencil>,
    ) -> Self {
        Self {
            color_attachments,
            depth_stencil,
            commands: Vec::new(),
        }
    }

    pub fn add_command(&mut self, command: RenderCommand) {
        self.commands.push(command);
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

    fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

pub struct CurrentFrameStorage<'a> {
    pub storage: &'a RenderStorage,
    pub current_frame_view: &'a TextureView,
}

impl<'a> CurrentFrameStorage<'a> {
    pub fn get_view(&self, id: ResourceId) -> &TextureView {
        if id == ResourceId::WINDOW_VIEW_ID {
            self.current_frame_view
        } else {
            &self.storage.get_texture(id).view
        }
    }
}

impl<'a> Deref for CurrentFrameStorage<'a> {
    type Target = RenderStorage;
    fn deref(&self) -> &Self::Target {
        self.storage
    }
}

#[derive(Debug, Default)]
pub struct RenderSystem {
    pub phases: HashMap<Cow<'static, str>, RenderPhase>,
    pub order: Vec<Cow<'static, str>>,
}

impl RenderSystem {
    pub fn add_phase(&mut self, name: impl Into<Cow<'static, str>>, phase: RenderPhase) {
        let name = name.into();
        self.order.push(name.clone());
        self.phases.insert(name, phase);
    }

    pub fn add_phase_commands(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        commands: Vec<RenderCommand>,
    ) {
        let name = name.into();
        self.phases
            .get_mut(&name)
            .unwrap_or_else(|| panic!("Setting commands for non existed phase: {name}"))
            .commands
            .extend(commands);
    }

    #[cfg(not(feature = "headless"))]
    pub fn run(
        &mut self,
        renderer: &Renderer,
        storage: &RenderStorage,
    ) -> Result<(), SurfaceError> {
        let current_frame = renderer.current_frame()?;
        self.run_system(renderer, storage, &current_frame);
        current_frame.present();
        Ok(())
    }

    #[cfg(feature = "headless")]
    pub fn run(&mut self, renderer: &Renderer, storage: &RenderStorage) {
        let current_frame = renderer.current_frame();
        self.run_system(renderer, storage, &current_frame);
    }

    fn run_system(
        &mut self,
        renderer: &Renderer,
        storage: &RenderStorage,
        current_frame: &CurrentFrameContext,
    ) {
        let mut encoder = renderer.create_encoder();

        let frame_storage = CurrentFrameStorage {
            storage,
            current_frame_view: current_frame.view(),
        };

        for p in self.order.iter() {
            let phase = self.phases.get_mut(p).unwrap();
            Self::execute_phase(Some(p), &mut encoder, phase, &frame_storage);
            phase.commands.clear();
        }

        renderer.submit(std::iter::once(encoder.finish()));
    }

    fn execute_phase(
        name: Option<&str>,
        encoder: &mut CommandEncoder,
        phase: &RenderPhase,
        storage: &CurrentFrameStorage,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: name,
            color_attachments: &phase.color_attachments(storage),
            depth_stencil_attachment: phase.depth_stencil_attachment(storage),
        });

        for command in phase.commands() {
            command.execute(&mut render_pass, storage);
        }
    }
}
