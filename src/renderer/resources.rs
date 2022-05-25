use super::{context::Renderer, wgpu_imports::*};
use crate::{mesh::GpuMesh, texture::GpuTexture, utils::sparse_set::SparseSet};
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
#[derive(Debug)]
pub struct RenderStorage {
    buffers: SparseSet<Buffer>,
    textures: SparseSet<GpuTexture>,
    meshes: SparseSet<GpuMesh>,
    bind_groups: SparseSet<BindGroup>,

    pipelines: Vec<RenderPipeline>,
    layouts: HashMap<&'static str, BindGroupLayout>,
}

impl Default for RenderStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderStorage {
    pub fn new() -> Self {
        Self {
            buffers: SparseSet::new(),
            textures: SparseSet::new(),
            meshes: SparseSet::new(),
            bind_groups: SparseSet::new(),
            pipelines: Vec::new(),
            layouts: HashMap::new(),
        }
    }

    pub fn insert_pipeline(&mut self, pipeline: RenderPipeline) -> ResourceId {
        let id = ResourceId(self.pipelines.len());
        self.pipelines.push(pipeline);
        id
    }

    pub fn insert_buffer(&mut self, buffer: Buffer) -> ResourceId {
        ResourceId(self.buffers.insert(buffer))
    }

    pub fn insert_texture(&mut self, texture: GpuTexture) -> ResourceId {
        ResourceId(self.textures.insert(texture))
    }

    pub fn insert_mesh(&mut self, mesh: GpuMesh) -> ResourceId {
        ResourceId(self.meshes.insert(mesh))
    }

    pub fn insert_bind_group(&mut self, bind_group: BindGroup) -> ResourceId {
        ResourceId(self.bind_groups.insert(bind_group))
    }

    pub fn replace_buffer(&mut self, buffer_id: ResourceId, buffer: Buffer) {
        if let Some(b) = self.buffers.get_mut(buffer_id.0) {
            *b = buffer;
        };
    }

    pub fn replace_texture(&mut self, texture_id: ResourceId, texture: GpuTexture) {
        if let Some(t) = self.textures.get_mut(texture_id.0) {
            *t = texture;
        };
    }

    pub fn replace_mesh(&mut self, mesh_id: ResourceId, mesh: GpuMesh) {
        if let Some(m) = self.meshes.get_mut(mesh_id.0) {
            *m = mesh;
        };
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
