pub mod context;
pub mod resources;
pub mod system;

pub mod prelude {
    use super::*;

    pub use context::*;
    pub use resources::*;
    pub use system::*;

    pub use wgpu_imports::*;
}

pub mod wgpu_imports {
    pub use wgpu::util::{BufferInitDescriptor, DeviceExt};
    pub use wgpu::{
        AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
        BlendComponent, BlendState, Buffer, BufferAddress, BufferBindingType, BufferUsages, Color,
        ColorTargetState, ColorWrites, CommandBuffer, CommandEncoder, CommandEncoderDescriptor,
        CompareFunction, DepthBiasState, DepthStencilState, Device, DeviceDescriptor, Extent3d,
        Face, Features, FilterMode, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout,
        IndexFormat, Instance, Limits, LoadOp, MultisampleState, Operations, Origin3d,
        PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode, PrimitiveState,
        PrimitiveTopology, Queue, RenderPass, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
        RenderPipelineDescriptor, RequestAdapterOptions, Sampler, SamplerBindingType,
        SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilFaceState,
        StencilOperation, StencilState, Surface, SurfaceConfiguration, SurfaceError,
        SurfaceTexture, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
        VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    };
}
