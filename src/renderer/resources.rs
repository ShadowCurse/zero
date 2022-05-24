use super::{context::Renderer, wgpu_imports::*};
use crate::{mesh::GpuMesh, texture::GpuTexture};
use std::collections::HashMap;

/// Trait for meshes vertices
pub trait Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a>;
}

/// Trait for types that create resources on the GPU (buffers, textures, etc..)
pub trait GpuResource {
    type ResourceType;
    fn build(&self, renderer: &Renderer) -> Self::ResourceType;
}

/// Trait for types that combine multiple GpuResources
pub trait ResourceHandle {
    type OriginalResource;
    type ResourceType;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self;
    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType);
    fn update(
        &self,
        _renderer: &Renderer,
        _storage: &RenderStorage,
        _original: &Self::OriginalResource,
    ) {
    }
}

/// Trait for the types that combine GpuResources into bind_groups
pub trait AssetBindGroup {
    type ResourceHandle;
    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout;
    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self;
}

/// Id assighed to any resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(usize);

impl ResourceId {
    pub const WINDOW_VIEW_ID: ResourceId = ResourceId(usize::MAX);
}

/// Strorage for resources
#[derive(Debug, Default)]
pub struct RenderStorage {
    // TODO use sparse arrays
    pub buffers: Vec<Buffer>,
    pub textures: Vec<GpuTexture>,
    pub meshes: Vec<GpuMesh>,
    pub bind_groups: Vec<BindGroup>,

    pub pipelines: Vec<RenderPipeline>,
    pub layouts: HashMap<&'static str, BindGroupLayout>,
}

impl RenderStorage {
    pub fn insert_pipeline(&mut self, pipeline: RenderPipeline) -> ResourceId {
        let id = ResourceId(self.pipelines.len());
        self.pipelines.push(pipeline);
        id
    }

    pub fn insert_buffer(&mut self, buffer: Buffer) -> ResourceId {
        let id = ResourceId(self.buffers.len());
        self.buffers.push(buffer);
        id
    }

    pub fn insert_texture(&mut self, texture: GpuTexture) -> ResourceId {
        let id = ResourceId(self.textures.len());
        self.textures.push(texture);
        id
    }

    pub fn insert_mesh(&mut self, mesh: GpuMesh) -> ResourceId {
        let id = ResourceId(self.meshes.len());
        self.meshes.push(mesh);
        id
    }

    pub fn insert_bind_group(&mut self, bind_group: BindGroup) -> ResourceId {
        let id = ResourceId(self.bind_groups.len());
        self.bind_groups.push(bind_group);
        id
    }

    pub fn replace_buffer(&mut self, buffer_id: ResourceId, buffer: Buffer) {
        self.buffers[buffer_id.0] = buffer;
    }

    pub fn replace_texture(&mut self, texture_id: ResourceId, texture: GpuTexture) {
        self.textures[texture_id.0] = texture;
    }

    pub fn replace_mesh(&mut self, mesh_id: ResourceId, mesh: GpuMesh) {
        self.meshes[mesh_id.0] = mesh;
    }

    pub fn register_bind_group_layout<A: AssetBindGroup>(&mut self, renderer: &Renderer) {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            self.layouts.insert(t_name, A::bind_group_layout(renderer));
        }
    }

    pub fn get_bind_group_layout<A: AssetBindGroup>(&self) -> &BindGroupLayout {
        let t_name = std::any::type_name::<A>();
        if !self.layouts.contains_key(t_name) {
            panic!("Trying to get a layout of an asset that was never built");
        }
        self.layouts.get(t_name).unwrap()
    }

    pub fn get_buffer(&self, id: ResourceId) -> &Buffer {
        self.buffers.get(id.0).unwrap()
    }

    pub fn get_texture(&self, id: ResourceId) -> &GpuTexture {
        self.textures.get(id.0).unwrap()
    }

    pub fn get_mesh(&self, id: ResourceId) -> &GpuMesh {
        self.meshes.get(id.0).unwrap()
    }

    pub fn get_bind_group(&self, id: ResourceId) -> &BindGroup {
        self.bind_groups.get(id.0).unwrap()
    }

    pub fn get_pipeline(&self, id: ResourceId) -> &RenderPipeline {
        self.pipelines.get(id.0).unwrap()
    }
}
