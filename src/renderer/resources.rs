use super::{context::Renderer, wgpu_imports::*};
use crate::{
    mesh::GpuMesh,
    texture::{DepthTexture, GpuTexture},
};
use log::trace;
use std::{collections::HashMap, fs::File, io::Read};

/// Trait for render vertices
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

    fn from_resource(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(usize);

impl ResourceId {
    pub const WINDOW_VIEW_ID: ResourceId = ResourceId(usize::MAX);
}

#[derive(Debug, Default)]
pub struct RenderResources {
    pub buffers: Vec<Buffer>,
    pub textures: Vec<GpuTexture>,
    pub meshes: Vec<GpuMesh>,
    pub bind_group: Option<BindGroup>,
}

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

#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    pub bind_group_layouts: Vec<&'a BindGroupLayout>,
    pub vertex_layouts: Vec<VertexBufferLayout<'a>>,
    pub shader_path: &'a str,
    pub primitive_topology: PrimitiveTopology,
    pub depth_enabled: bool,
    pub stencil_enabled: bool,
    pub stencil_compare: CompareFunction,
    pub stencil_read_mask: u32,
    pub stencil_write_mask: u32,
    pub write_depth: bool,
    pub color_targets: Option<Vec<TextureFormat>>,
    pub cull_mode: Face,
}

impl<'a> std::default::Default for PipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            bind_group_layouts: vec![],
            vertex_layouts: vec![],
            shader_path: "",
            primitive_topology: PrimitiveTopology::TriangleList,
            depth_enabled: true,
            stencil_enabled: false,
            stencil_compare: CompareFunction::Always,
            stencil_read_mask: 0x00,
            stencil_write_mask: 0x00,
            write_depth: true,
            color_targets: None,
            cull_mode: Face::Back,
        }
    }
}

impl<'a> PipelineBuilder<'a> {
    pub fn build(self, renderer: &Renderer) -> RenderPipeline {
        trace!("Building pipilene: {}", self.shader_path);
        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("render_pipeline_descriptor"),
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let mut contents = String::new();
        {
            let mut file = File::open(self.shader_path).unwrap();
            file.read_to_string(&mut contents).unwrap();
        }

        let shader = ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(contents.into()),
        };
        let shader = renderer.device.create_shader_module(&shader);

        let targets = match self.color_targets {
            Some(ct) => ct
                .into_iter()
                .map(|ct| ColorTargetState {
                    format: ct,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })
                .collect(),
            None => vec![ColorTargetState {
                format: renderer.config.format,
                blend: Some(BlendState {
                    alpha: BlendComponent::REPLACE,
                    color: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            }],
        };

        let strip_index_format = match self.primitive_topology {
            PrimitiveTopology::TriangleList => None,
            PrimitiveTopology::TriangleStrip => Some(IndexFormat::Uint32),
            _ => unimplemented!(),
        };

        let depth_stencil = if self.depth_enabled {
            let stencil = if self.stencil_enabled {
                let stencil_state = StencilFaceState {
                    compare: self.stencil_compare,
                    fail_op: StencilOperation::Keep,
                    depth_fail_op: StencilOperation::Keep,
                    pass_op: StencilOperation::IncrementClamp,
                };

                StencilState {
                    front: stencil_state,
                    back: stencil_state,
                    read_mask: self.stencil_read_mask,
                    write_mask: self.stencil_write_mask,
                }
            } else {
                StencilState::default()
            };

            Some(DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: self.write_depth,
                depth_compare: CompareFunction::LessEqual,
                stencil,
                bias: DepthBiasState::default(),
            })
        } else {
            None
        };

        renderer
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(self.shader_path),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &self.vertex_layouts,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &targets,
                }),
                primitive: PrimitiveState {
                    topology: self.primitive_topology,
                    strip_index_format,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(self.cull_mode),
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
