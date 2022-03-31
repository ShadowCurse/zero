use crate::model::GpuMesh;
use crate::renderer::{GpuAsset, PipelineBuilder, RenderAsset, Renderer};
use crate::texture::GpuTexture;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Color, CommandEncoder, IndexFormat, Operations,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPipeline, TextureView,
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
        command.execute(&mut render_pass, storage);
    }
}

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
    fn execute<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        storage: &'a CurrentFrameStorage,
    ) {
        let meshes = storage.get_meshes(self.mesh_id);
        let bind_groups: Vec<_> = self
            .bind_groups
            .iter()
            .map(|meta| {
                (
                    meta.index,
                    storage.get_bind_group(meta.bind_group_id).unwrap(),
                )
            })
            .collect();
        let pipeline = storage.get_pipeline(self.pipeline_id);
        render_pass.set_pipeline(pipeline);
        for m in meshes.iter() {
            for bg in bind_groups.iter() {
                render_pass.set_bind_group(bg.0, bg.1, &[]);
            }
            render_pass.set_vertex_buffer(0, m.vertex_buffer.slice(..));
            if m.index_buffer.is_some() {
                render_pass.set_index_buffer(
                    m.index_buffer.as_ref().unwrap().slice(..),
                    IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..m.num_elements, 0, 0..1);
            } else {
                render_pass.draw(0..m.num_elements, 0..1);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub usize);

impl ResourceId {
    pub const WINDOW_VIEW_ID: ResourceId = ResourceId(usize::MAX);
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
    pub meshes: Vec<GpuMesh>,
    pub bind_group: Option<BindGroup>,
}

#[derive(Debug)]
pub struct IndexBuffer {
    pub buffer: Option<Buffer>,
    pub num_elements: u32,
}

#[derive(Debug, Default)]
pub struct RenderStorage {
    pub buffers: Vec<Vec<Buffer>>,
    pub textures: Vec<Vec<GpuTexture>>,
    pub meshes: Vec<Vec<GpuMesh>>,
    pub bind_groups: Vec<Option<BindGroup>>,
    // ....
    pub pipelines: Vec<RenderPipeline>,
    pub layouts: HashMap<&'static str, wgpu::BindGroupLayout>,
}

impl RenderStorage {
    pub fn build_asset<A: RenderAsset>(&mut self, renderer: &Renderer, item: &A) -> ResourceId {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            self.layouts.insert(t_name, A::bind_group_layout(renderer));
        }
        let layout = self.layouts.get(t_name).unwrap();
        let resources = item.build(renderer, layout);
        self.insert_resources(resources)
    }

    pub fn build_texture<A: GpuAsset<GpuType = GpuTexture>>(
        &mut self,
        renderer: &Renderer,
        texture: &A,
    ) -> ResourceId {
        let texture = texture.build(renderer);
        self.insert_resources(RenderResources {
            textures: vec![texture],
            ..Default::default()
        })
    }

    pub fn build_mesh<A: GpuAsset<GpuType = GpuMesh>>(
        &mut self,
        renderer: &Renderer,
        mesh: &A,
    ) -> ResourceId {
        let mesh = mesh.build(renderer);
        self.insert_resources(RenderResources {
            meshes: vec![mesh],
            ..Default::default()
        })
    }

    pub fn add_pipeline(&mut self, pipeline: RenderPipeline) -> ResourceId {
        let id = self.pipelines.len();
        self.pipelines.push(pipeline);
        ResourceId(id)
    }

    fn insert_resources(&mut self, resources: RenderResources) -> ResourceId {
        let id = self.buffers.len();
        self.buffers.push(resources.buffers);
        self.textures.push(resources.textures);
        self.meshes.push(resources.meshes);
        self.bind_groups.push(resources.bind_group);
        ResourceId(id)
    }

    pub fn rebuild_asset<A: RenderAsset>(&mut self, renderer: &Renderer, item: &A, id: ResourceId) {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            panic!("Rebuilding asset that was never built");
        }
        let layout = self.layouts.get(t_name).unwrap();
        let resources = item.build(renderer, layout);
        self.insert_resources_at(resources, id);
    }

    pub fn rebuild_texture<A: GpuAsset<GpuType = GpuTexture>>(
        &mut self,
        renderer: &Renderer,
        texture: &A,
        id: ResourceId,
    ) {
        let texture = texture.build(renderer);
        self.insert_resources_at(
            RenderResources {
                textures: vec![texture],
                ..Default::default()
            },
            id,
        );
    }

    pub fn rebuild_mesh<A: GpuAsset<GpuType = GpuMesh>>(
        &mut self,
        renderer: &Renderer,
        mesh: &A,
        id: ResourceId,
    ) {
        let mesh = mesh.build(renderer);
        self.insert_resources_at(
            RenderResources {
                meshes: vec![mesh],
                ..Default::default()
            },
            id,
        );
    }

    fn insert_resources_at(&mut self, resources: RenderResources, id: ResourceId) {
        self.buffers[id.0] = resources.buffers;
        self.textures[id.0] = resources.textures;
        self.meshes[id.0] = resources.meshes;
        self.bind_groups[id.0] = resources.bind_group;
    }

    pub fn get_bind_group_layout<A: RenderAsset>(&self) -> &BindGroupLayout {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            panic!("Trying to get a layout of an asset that was never built");
        }
        self.layouts.get(t_name).unwrap()
    }

    pub fn get_buffers(&self, id: ResourceId) -> &[Buffer] {
        self.buffers.get(id.0).unwrap()
    }

    pub fn get_textures(&self, id: ResourceId) -> &[GpuTexture] {
        self.textures.get(id.0).unwrap()
    }

    pub fn get_meshes(&self, id: ResourceId) -> &[GpuMesh] {
        self.meshes.get(id.0).unwrap()
    }

    pub fn get_bind_group(&self, id: ResourceId) -> Option<&BindGroup> {
        self.bind_groups.get(id.0).unwrap().as_ref()
    }

    pub fn get_pipeline(&self, id: ResourceId) -> &RenderPipeline {
        self.pipelines.get(id.0).unwrap()
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
            // TODO enable clear later
            // phase.commands.clear();
        }

        renderer.submit(std::iter::once(encoder.finish()));
        current_frame.output.present();
        Ok(())
    }
}
