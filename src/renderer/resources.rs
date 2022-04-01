use super::Renderer;
use super::*;
use crate::texture::DepthTexture;
use crate::{model::GpuMesh, texture::GpuTexture};
use std::io::prelude::*;
use std::{collections::HashMap, fs::File};

/// Trait for render vertices
pub trait Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a>;
}

/// Trait for resources located on the GPU
pub trait GpuResource {}

/// Trait for types that can be loaded to the GPU
pub trait GpuAsset {
    type GpuType: GpuResource;

    fn build(&self, renderer: &Renderer) -> Self::GpuType;
}

/// Trait for the types that can be converted to the RenderResource
pub trait RenderAsset {
    const ASSET_NAME: &'static str;
    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout;
    fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources;
    fn update(&self, _renderer: &Renderer, _id: ResourceId, _storage: &RenderStorage) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub usize);

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
    pub layouts: HashMap<&'static str, BindGroupLayout>,
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

#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    bind_group_layouts: Vec<&'a BindGroupLayout>,
    vertex_layouts: Vec<VertexBufferLayout<'a>>,
    shader_path: String,
    primitive_topology: PrimitiveTopology,
    depth_enabled: bool,
    stencil_enabled: bool,
    stencil_compare: CompareFunction,
    stencil_read_mask: u32,
    stencil_write_mask: u32,
    write_depth: bool,
    color_targets: Option<Vec<TextureFormat>>,
}

impl<'a> std::default::Default for PipelineBuilder<'a> {
    fn default() -> Self {
        Self {
            bind_group_layouts: Vec::new(),
            vertex_layouts: Vec::new(),
            shader_path: "".to_string(),
            primitive_topology: PrimitiveTopology::TriangleList,
            depth_enabled: true,
            stencil_enabled: false,
            stencil_compare: CompareFunction::Always,
            stencil_read_mask: 0x00,
            stencil_write_mask: 0x00,
            write_depth: true,
            color_targets: None,
        }
    }
}

impl<'a> PipelineBuilder<'a> {
    pub fn new<P: Into<String>>(
        bind_group_layouts: Vec<&'a BindGroupLayout>,
        vertex_layouts: Vec<VertexBufferLayout<'a>>,
        shader_path: P,
    ) -> Self {
        Self {
            bind_group_layouts,
            vertex_layouts,
            shader_path: shader_path.into(),
            stencil_compare: CompareFunction::Always,
            ..Default::default()
        }
    }

    pub fn primitive_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.primitive_topology = topology;
        self
    }

    pub fn depth_enabled(mut self, enabled: bool) -> Self {
        self.depth_enabled = enabled;
        self
    }

    pub fn stencil_enabled(mut self, enabled: bool) -> Self {
        self.stencil_enabled = enabled;
        self
    }

    pub fn stencil_compare(mut self, stencil_compare: CompareFunction) -> Self {
        self.stencil_compare = stencil_compare;
        self
    }

    pub fn stencil_read_mask(mut self, stencil_read_mask: u32) -> Self {
        self.stencil_read_mask = stencil_read_mask;
        self
    }

    pub fn stencil_write_mask(mut self, stencil_write_mask: u32) -> Self {
        self.stencil_write_mask = stencil_write_mask;
        self
    }

    pub fn write_depth(mut self, write_depth: bool) -> Self {
        self.write_depth = write_depth;
        self
    }

    pub fn color_targets(mut self, color_targets: Vec<TextureFormat>) -> Self {
        self.color_targets = Some(color_targets);
        self
    }

    pub fn build(self, renderer: &Renderer) -> RenderPipeline {
        println!("building pipilene: {}", self.shader_path);
        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("render_pipeline_descriptor"),
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        let mut contents = String::new();
        let mut file = File::open(self.shader_path).unwrap();
        file.read_to_string(&mut contents).unwrap();

        let shader = ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(contents.into()),
        };

        let shader = renderer.device.create_shader_module(&shader);

        let targets = if let Some(color_targets) = self.color_targets {
            color_targets
                .into_iter()
                .map(|ct| ColorTargetState {
                    format: ct,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })
                .collect()
        } else {
            vec![ColorTargetState {
                format: renderer.config.format,
                blend: Some(BlendState {
                    alpha: BlendComponent::REPLACE,
                    color: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            }]
        };

        renderer
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("render_pipeline"),
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
                    strip_index_format: match self.primitive_topology {
                        PrimitiveTopology::TriangleList => None,
                        PrimitiveTopology::TriangleStrip => Some(IndexFormat::Uint32),
                        _ => unimplemented!(),
                    },
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: if self.depth_enabled {
                    Some(DepthStencilState {
                        format: DepthTexture::DEPTH_FORMAT,
                        depth_write_enabled: self.write_depth,
                        depth_compare: CompareFunction::LessEqual,
                        stencil: if self.stencil_enabled {
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
                        },
                        bias: DepthBiasState::default(),
                    })
                } else {
                    None
                },
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }
}
