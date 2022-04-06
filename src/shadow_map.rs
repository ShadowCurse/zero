use crate::camera;
use crate::renderer::*;
use crate::texture::CubeMap;
use crate::texture::DepthTexture;
use cgmath::{ortho, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};

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

#[derive(Debug)]
pub struct ShadowCubeMap {
    pub cube_map: CubeMap,
}

impl RenderAsset for ShadowCubeMap {
    const ASSET_NAME: &'static str = "ShadowCubeMap";

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
                            view_dimension: TextureViewDimension::Cube,
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
                label: Some("shadow_cube_bind_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &BindGroupLayout) -> RenderResources {
        let shadow_cube = self.cube_map.build(renderer);

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&shadow_cube.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&shadow_cube.sampler),
                },
            ],
            label: None,
        });
        RenderResources {
            textures: vec![shadow_cube],
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
            * Matrix4::look_to_rh(self.position, self.direction, Vector3::unit_y())
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
            label: Some("shdow_map_bind_group"),
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
