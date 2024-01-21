use crate::camera::OPENGL_TO_WGPU_MATRIX;
use crate::prelude::GpuTexture;
use crate::render::prelude::*;
use crate::texture::EmptyTexture;
use crate::{cgmath_imports::*, impl_simple_buffer, impl_simple_texture_bind_group};

#[derive(Debug)]
pub struct ShadowMap {
    pub shadow_map: EmptyTexture,
}

impl Default for ShadowMap {
    fn default() -> Self {
        Self {
            shadow_map: EmptyTexture {
                dimensions: None,
                format: TextureFormat::Depth32Float,
                filtered: true,
            },
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug, Clone, Copy)]
pub struct ShadowMapHandle {
    pub texture_id: ResourceId,
}

impl ResourceHandle for ShadowMapHandle {
    type OriginalResource<'a> = ShadowMap;
    type ResourceType = ShadowMapResource;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            texture_id: storage.insert_texture(resource.texture),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.texture_id, resource.texture);
    }
}

impl_simple_texture_bind_group!(
    ShadowMapHandle,
    ShadowMapBindGroup,
    { TextureViewDimension::D2 },
    { TextureSampleType::Depth },
    { SamplerBindingType::Filtering }
);

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowMapDLightUniform {
    view_projection: [[f32; 4]; 4],
}

impl From<&ShadowMapDLight> for ShadowMapDLightUniform {
    fn from(value: &ShadowMapDLight) -> Self {
        Self {
            view_projection: (value.projection() * value.view()).into(),
        }
    }
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
}

impl_simple_buffer!(
    ShadowMapDLight,
    ShadowMapDLightUniform,
    ShadowMapDLightResources,
    ShadowMapDLightHandle,
    ShadowMapDLightBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX },
    { BufferBindingType::Uniform }
);

#[derive(Debug, Clone, Copy)]
pub struct ShadowBindGroup(pub ResourceId);

impl AssetBindGroup for ShadowBindGroup {
    type ResourceHandle = (ShadowMapHandle, ShadowMapDLightHandle);

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
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
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("shadow_binding_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self {
        let layout = storage.get_bind_group_layout::<Self>();

        let (shadow_map, shadow_d_light) = resource;
        let texture = storage.get_texture(shadow_map.texture_id);
        let buffer = storage.get_buffer(shadow_d_light.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: Some("shadow_bind_group"),
        });

        Self(storage.insert_bind_group(bind_group))
    }

    fn replace(
        &self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) {
        let layout = storage.get_bind_group_layout::<Self>();

        let (shadow_map, shadow_d_light) = resource;
        let texture = storage.get_texture(shadow_map.texture_id);
        let buffer = storage.get_buffer(shadow_d_light.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: Some("shadow_bind_group"),
        });

        storage.replace_bind_group(self.0, bind_group);
    }
}
