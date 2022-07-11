use super::{renderer::Renderer, storage::RenderStorage, wgpu_imports::*};

/// Trait for meshes vertices
pub trait VertexLayout {
    fn layout<'a>() -> VertexBufferLayout<'a>;
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
    fn replace(
        &self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    );
}
