use crate::camera;
use crate::renderer::*;
use crate::texture::DepthTexture;
use cgmath::{ortho, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};

//TODO this is a copy of depth texture 
//do i even need this?
// #[derive(Debug, Default)]
// pub struct ShadowMapTexture;
//
// impl ShadowMapTexture {
//     pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
// }
//
// impl GpuAsset for ShadowMapTexture {
//     type GpuType = GpuTexture;
//
//     fn build(&self, renderer: &Renderer) -> Self::GpuType {
//         let size = Extent3d {
//             width: renderer.config.width,
//             height: renderer.config.height,
//             depth_or_array_layers: 1,
//         };
//         let desc = TextureDescriptor {
//             size,
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: TextureDimension::D2,
//             format: Self::DEPTH_FORMAT,
//             usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
//             label: Some("depth_map_texture"),
//         };
//         let texture = renderer.device.create_texture(&desc);
//
//         let view = texture.create_view(&TextureViewDescriptor::default());
//         let sampler = renderer.device.create_sampler(&SamplerDescriptor {
//             address_mode_u: AddressMode::ClampToEdge,
//             address_mode_v: AddressMode::ClampToEdge,
//             address_mode_w: AddressMode::ClampToEdge,
//             mag_filter: FilterMode::Linear,
//             min_filter: FilterMode::Linear,
//             mipmap_filter: FilterMode::Nearest,
//             compare: None, //Some(CompareFunction::LessEqual),
//             lod_min_clamp: -100.0,
//             lod_max_clamp: 100.0,
//             ..Default::default()
//         });
//
//         Self::GpuType {
//             texture,
//             view,
//             sampler,
//         }
//     }
// }

#[derive(Debug, Default)]
pub struct ShadowMap {
    pub shadow_map: DepthTexture,
}

impl RenderAsset for ShadowMap {
    const ASSET_NAME: &'static str = "ShadowMap";

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("shadow_map_bind_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
        let shadow_map = self.shadow_map.build(renderer);

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&shadow_map.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&shadow_map.sampler),
                },
            ],
            label: None,
        });
        RenderResources {
            textures: vec![shadow_map],
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowMapDLightUniform {
    view_projection: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct ShadowMapDLight {
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl ShadowMapDLight {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P: Into<Point3<f32>>, D: Into<Vector3<f32>>>(
        position: P,
        direction: D,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position: position.into(),
            direction: direction.into(),
            left,
            right,
            bottom,
            top,
            near,
            far,
        }
    }

    fn view(&self) -> Matrix4<f32> {
        camera::OPENGL_TO_WGPU_MATRIX
            * Matrix4::look_to_rh(
                self.position,
                (self.position.to_vec() + self.direction).normalize(),
                Vector3::unit_y(),
            )
    }

    fn projection(&self) -> Matrix4<f32> {
        camera::OPENGL_TO_WGPU_MATRIX
            * ortho(
                self.left,
                self.right,
                self.bottom,
                self.top,
                self.near,
                self.far,
            )
    }

    fn to_uniform(&self) -> ShadowMapDLightUniform {
        ShadowMapDLightUniform {
            view_projection: (self.projection() * self.view()).into(),
        }
    }
}

impl RenderAsset for ShadowMapDLight {
    const ASSET_NAME: &'static str = "ShadowMapDLight";

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("shadow_map_binding_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
        let uniform = self.to_uniform();

        let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("shadow_map_dlight_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("comera_bind_group"),
        });

        RenderResources {
            buffers: vec![buffer],
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }

    fn update(&self, renderer: &Renderer, id: ResourceId, storage: &RenderStorage) {
        renderer.queue.write_buffer(
            &storage.get_buffers(id)[0],
            0,
            bytemuck::cast_slice(&[self.to_uniform()]),
        );
    }
}
