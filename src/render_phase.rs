use crate::renderer::{RenderAsset, Renderer};
use crate::texture::GpuTexture;
use std::borrow::Cow;
use std::collections::HashMap;
use wgpu::{
    BindGroup, Buffer, Color, CommandEncoder, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, TextureView,
};

pub fn execute_phase(
    name: Option<&str>,
    encoder: &mut CommandEncoder,
    phase: &RenderPhase,
    storage: &CurrentFrameStorage,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: name,
        color_attachments: &phase.color_attachments(storage),
        depth_stencil_attachment: phase.depth_stencil_attachment(storage),
    });

    for command in phase.commands() {
        command.execute(&mut render_pass);
    }
}

pub struct RenderCommand;

impl RenderCommand {
    fn execute<'a>(&self, render_pass: &mut wgpu::RenderPass<'a>) {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub usize);

impl ResourceId {
    const WINDOW_VIEW_ID: ResourceId = ResourceId(0);
}

pub struct ColorAttachment {
    view_id: ResourceId,
    ops: Operations<Color>,
}

pub struct DepthStencil {
    view_id: ResourceId,
    depth_ops: Option<Operations<f32>>,
    stencil_ops: Option<Operations<u32>>,
}

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
    ) -> Vec<RenderPassColorAttachment<'a>> {
        self.color_attachments
            .iter()
            .flat_map(|attachment| {
                storage
                    .get_views(attachment.view_id)
                    .into_iter()
                    .map(|view| wgpu::RenderPassColorAttachment {
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
            let views = storage.get_views(depth_stencil.view_id);
            if views.len() != 1 {
                panic!("Resource for depth_stencil has invalid number of textures: {}, but should be 1", views.len());
            }
            wgpu::RenderPassDepthStencilAttachment {
                view: views[0],
                depth_ops: depth_stencil.depth_ops,
                stencil_ops: depth_stencil.stencil_ops,
            }
        })
    }

    fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

#[derive(Debug, Default)]
pub struct RenderResources {
    pub buffers: Vec<Buffer>,
    pub textures: Vec<GpuTexture>,
    pub vertex_buffer: Option<Buffer>,
    pub index_type: Option<IndexType>,
    pub bind_group: Option<BindGroup>,
}

#[derive(Debug)]
pub enum IndexType {
    Buffer(Buffer),
    NumElements(u32),
}

pub struct RenderStorage {
    pub buffers: Vec<Vec<Buffer>>,
    pub textures: Vec<Vec<GpuTexture>>,
    pub vertex_buffers: Vec<Option<Buffer>>,
    pub index_type: Vec<Option<IndexType>>,
    pub bind_groups: Vec<Option<BindGroup>>,
    // ....
    pub layouts: HashMap<&'static str, wgpu::BindGroupLayout>,
}

impl RenderStorage {
    pub fn build<A: RenderAsset>(&mut self, renderer: &Renderer, item: A) -> ResourceId {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            self.layouts.insert(t_name, A::bind_group_layout(renderer));
        }
        let layout = self.layouts.get(t_name).unwrap();
        let resources = item.build(renderer, layout);
        self.insert_resources(resources)
    }

    fn insert_resources(&mut self, resources: RenderResources) -> ResourceId {
        let id = self.buffers.len();
        self.buffers.push(resources.buffers);
        self.textures.push(resources.textures);
        self.vertex_buffers.push(resources.vertex_buffer);
        self.index_type.push(resources.index_type);
        self.bind_groups.push(resources.bind_group);
        ResourceId(id)
    }

    pub fn get_buffers(&self, id: ResourceId) -> &[Buffer] {
        self.buffers.get(id.0).unwrap()
    }

    pub fn get_textures(&self, id: ResourceId) -> &[GpuTexture] {
        self.textures.get(id.0).unwrap()
    }
}

pub struct CurrentFrameStorage<'a> {
    pub storage: &'a RenderStorage,
    pub current_frame_view: &'a TextureView,
}

impl<'a> CurrentFrameStorage<'a> {
    pub fn get_views(&self, id: ResourceId) -> Vec<&TextureView> {
        if id == ResourceId::WINDOW_VIEW_ID {
            vec![self.current_frame_view]
        } else {
            self.storage
                .get_textures(id)
                .iter()
                .map(|texture| &texture.view)
                .collect()
        }
    }
}

#[derive(Default)]
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

    pub fn run(
        &mut self,
        renderer: &Renderer,
        storage: &RenderStorage,
    ) -> Result<(), wgpu::SurfaceError> {
        let current_frame = renderer.current_frame()?;
        let mut encoder = renderer.create_encoder();

        let frame_storage = CurrentFrameStorage {
            storage,
            current_frame_view: &current_frame.view,
        };

        for p in self.order.iter() {
            let phase = self.phases.get_mut(p).unwrap();
            execute_phase(Some(p), &mut encoder, phase, &frame_storage);
            phase.commands.clear();
        }

        renderer.submit(std::iter::once(encoder.finish()));
        current_frame.output.present();
        Ok(())
    }
}
