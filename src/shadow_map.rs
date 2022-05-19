use crate::camera::OPENGL_TO_WGPU_MATRIX;
use crate::cgmath_imports::*;
use crate::prelude::GpuTexture;
use crate::renderer::prelude::*;
use crate::texture::DepthTexture;

#[derive(Debug, Default)]
pub struct ShadowMap {
    pub shadow_map: DepthTexture,
}

pub struct ShadowMapResource {
    texture: GpuTexture,
}

impl GpuResource for ShadowMap {
    type ResourceType = ShadowMapResource;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture = self.shadow_map.build(renderer);
        Self::ResourceType { texture }
    }
}

pub struct ShadowMapHandle {
    pub texture_id: ResourceId,
}

impl ResourceHandle for ShadowMapHandle {
    type OriginalResource = ShadowMap;
    type ResourceType = ShadowMapResource;

    fn from_resource(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            texture_id: storage.insert_texture(resource.texture),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.texture_id, resource.texture);
    }
}

pub struct ShadowMapBindGroup(pub ResourceId);

impl AssetBindGroup for ShadowMapBindGroup {
    type ResourceHandle = ShadowMapHandle;

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
                            sample_type: TextureSampleType::Depth,
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

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resources: &Self::ResourceHandle,
    ) -> Self {
        storage.register_bind_group_layout::<Self>(renderer);
        let layout = storage.get_bind_group_layout::<Self>();
        let shadow_map = storage.get_texture(resources.texture_id);

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
        Self(storage.insert_bind_group(bind_group))
    }
}

// #[derive(Debug)]
// pub struct ShadowCubeMap {
//     pub cube_map: CubeMap,
// }
//
// impl Default for ShadowCubeMap {
//     fn default() -> Self {
//         Self {
//             cube_map: CubeMap {
//                 format: TextureFormat::Depth32Float,
//                 texture: None,
//                 dimensions: None,
//             },
//         }
//     }
// }
//
// impl AssetBindGroup for ShadowCubeMap {
//     fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
//         renderer
//             .device
//             .create_bind_group_layout(&BindGroupLayoutDescriptor {
//                 entries: &[
//                     BindGroupLayoutEntry {
//                         binding: 0,
//                         visibility: ShaderStages::FRAGMENT,
//                         ty: BindingType::Texture {
//                             multisampled: false,
//                             view_dimension: TextureViewDimension::Cube,
//                             sample_type: TextureSampleType::Float { filterable: true },
//                         },
//                         count: None,
//                     },
//                     BindGroupLayoutEntry {
//                         binding: 1,
//                         visibility: ShaderStages::FRAGMENT,
//                         ty: BindingType::Sampler(SamplerBindingType::Filtering),
//                         count: None,
//                     },
//                 ],
//                 label: Some("shadow_cube_bind_group_layout"),
//             })
//     }
//
//     fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
//         let shadow_cube = self.cube_map.build(renderer);
//
//         let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
//             layout,
//             entries: &[
//                 BindGroupEntry {
//                     binding: 0,
//                     resource: BindingResource::TextureView(&shadow_cube.view),
//                 },
//                 BindGroupEntry {
//                     binding: 1,
//                     resource: BindingResource::Sampler(&shadow_cube.sampler),
//                 },
//             ],
//             label: None,
//         });
//         RenderResources {
//             textures: vec![shadow_cube],
//             bind_group: Some(bind_group),
//             ..Default::default()
//         }
//     }
// }

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
        OPENGL_TO_WGPU_MATRIX
            * Matrix4::look_to_rh(self.position, self.direction, Vector3::unit_y())
    }

    fn projection(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX
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

pub struct ShadowMapDLightResource {
    buffer: Buffer,
}

impl GpuResource for ShadowMapDLight {
    type ResourceType = ShadowMapDLightResource;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let uniform = self.to_uniform();

        let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("shadow_map_dlight_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self::ResourceType { buffer }
    }
}

pub struct ShadowMapDLightHandle {
    pub buffer_id: ResourceId,
}

impl ResourceHandle for ShadowMapDLightHandle {
    type OriginalResource = ShadowMapDLight;
    type ResourceType = ShadowMapDLightResource;

    fn from_resource(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            buffer_id: storage.insert_buffer(resource.buffer),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_buffer(self.buffer_id, resource.buffer);
    }

    fn update(
        &self,
        renderer: &Renderer,
        storage: &RenderStorage,
        original: &Self::OriginalResource,
    ) {
        renderer.queue.write_buffer(
            storage.get_buffer(self.buffer_id),
            0,
            bytemuck::cast_slice(&[original.to_uniform()]),
        );
    }
}

pub struct ShadowMapDLightBindGroup(pub ResourceId);

impl AssetBindGroup for ShadowMapDLightBindGroup {
    type ResourceHandle = ShadowMapDLightHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
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

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resources: &Self::ResourceHandle,
    ) -> Self {
        storage.register_bind_group_layout::<Self>(renderer);
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resources.buffer_id);

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("shdow_map_bind_group"),
        });

        Self(storage.insert_bind_group(bind_group))
    }
}

//
// #[repr(C)]
// #[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct ShadowMapPLightUniform {
//     view_projections: [[[f32; 4]; 4]; 6],
// }
//
// #[derive(Debug)]
// pub struct ShadowMapPLight {
//     pub position: Point3<f32>,
//     pub aspect: f32,
//     pub fovy: Rad<f32>,
//     pub znear: f32,
//     pub zfar: f32,
// }
//
// impl ShadowMapPLight {
//     pub fn new<P: Into<Point3<f32>>>(
//         position: P,
//         width: u32,
//         height: u32,
//         znear: f32,
//         zfar: f32,
//     ) -> Self {
//         Self {
//             position: position.into(),
//             aspect: width as f32 / height as f32,
//             fovy: Deg(90.0).into(),
//             znear,
//             zfar,
//         }
//     }
//
//     fn to_uniform(&self) -> ShadowMapPLightUniform {
//         let proj =
//             OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar);
//
//         let dirs = [
//             ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
//             ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
//             ([0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
//             ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
//             ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
//             ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
//         ];
//         let mut view_projections = [[[0.0; 4]; 4]; 6];
//         view_projections
//             .iter_mut()
//             .zip(dirs.into_iter())
//             .for_each(|(vp, (dir, up))| {
//                 *vp = (proj * Matrix4::look_to_rh(self.position, dir.into(), up.into())).into();
//             });
//
//         ShadowMapPLightUniform { view_projections }
//     }
// }
//
// impl AssetBindGroup for ShadowMapPLight {
//     fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
//         renderer
//             .device
//             .create_bind_group_layout(&BindGroupLayoutDescriptor {
//                 entries: &[BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
//                     ty: BindingType::Buffer {
//                         ty: BufferBindingType::Uniform,
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 }],
//                 label: Some("shadow_map_binding_group_layout"),
//             })
//     }
//
//     fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
//         let uniform = self.to_uniform();
//
//         let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
//             label: Some("shadow_map_plight_buffer"),
//             contents: bytemuck::cast_slice(&[uniform]),
//             usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
//         });
//
//         let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
//             layout,
//             entries: &[BindGroupEntry {
//                 binding: 0,
//                 resource: buffer.as_entire_binding(),
//             }],
//             label: Some("shdow_map_bind_group"),
//         });
//
//         RenderResources {
//             buffers: vec![buffer],
//             bind_group: Some(bind_group),
//             ..Default::default()
//         }
//     }
//
//     fn update(&self, renderer: &Renderer, id: ResourceId, storage: &RenderStorage) {
//         renderer.queue.write_buffer(
//             &storage.get_buffers(id)[0],
//             0,
//             bytemuck::cast_slice(&[self.to_uniform()]),
//         );
//     }
// }
