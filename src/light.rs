use crate::render::prelude::*;
use cgmath::Vector3;

#[macro_export]
macro_rules! impl_gpu_resource {
    ($type:ty, $resource_type:ty) => {
        impl GpuResource for $type {
            type ResourceType = $resource_type;

            fn build(&self, renderer: &Renderer) -> Self::ResourceType {
                let uniform = self.to_uniform();

                let buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
                    label: Some("{$type}_uniform"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
                Self::ResourceType { buffer }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_resource_handle {
    ($handle_type:ty, $original_type:ty, $resource_type:ty) => {
        impl ResourceHandle for $handle_type {
            type OriginalResource = $original_type;
            type ResourceType = $resource_type;

            fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
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
                renderer.queue().write_buffer(
                    storage.get_buffer(self.buffer_id),
                    0,
                    bytemuck::cast_slice(&[original.to_uniform()]),
                );
            }
        }
    };
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectionalLightUniform {
    direction: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
    _pad2: u32,
}

#[derive(Debug)]
pub struct DirectionalLight {
    pub direction: Vector3<f32>,
    pub color: Vector3<f32>,
}

impl DirectionalLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(direction: P, color: C) -> Self {
        Self {
            direction: direction.into(),
            color: color.into(),
        }
    }

    fn to_uniform(&self) -> DirectionalLightUniform {
        DirectionalLightUniform {
            direction: self.direction.into(),
            color: self.color.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct DirectionalLightResource {
    pub buffer: Buffer,
}
impl_gpu_resource!(DirectionalLight, DirectionalLightResource);

#[derive(Debug)]
pub struct DirectionalLightHandle {
    pub buffer_id: ResourceId,
}
impl_resource_handle!(
    DirectionalLightHandle,
    DirectionalLight,
    DirectionalLightResource
);

#[derive(Debug)]
struct DirectionalLightBindGroup(pub ResourceId);

impl AssetBindGroup for DirectionalLightBindGroup {
    type ResourceHandle = DirectionalLightHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
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
                label: Some("binding_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self {
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
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
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        storage.replace_bind_group(self.0, bind_group)
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightUniform {
    position: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
    _pad2: u32,
    constant: f32,
    linear: f32,
    quadratic: f32,
    _pad3: u32,
}

#[derive(Debug, Clone)]
pub struct PointLight {
    pub position: Vector3<f32>,
    pub color: Vector3<f32>,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(
        position: P,
        color: C,
        constant: f32,
        linear: f32,
        quadratic: f32,
    ) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
            constant,
            linear,
            quadratic,
        }
    }

    fn to_uniform(&self) -> PointLightUniform {
        PointLightUniform {
            position: self.position.into(),
            color: self.color.into(),
            constant: self.constant,
            linear: self.linear,
            quadratic: self.quadratic,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct PointLightResource {
    pub buffer: Buffer,
}
impl_gpu_resource!(PointLight, PointLightResource);

#[derive(Debug)]
pub struct PointLightHandle {
    pub buffer_id: ResourceId,
}
impl_resource_handle!(PointLightHandle, PointLight, PointLightResource);

#[derive(Debug)]
pub struct PointLightBindGroup(pub ResourceId);

impl AssetBindGroup for PointLightBindGroup {
    type ResourceHandle = PointLightHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
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
                label: Some("binding_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self {
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
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
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        storage.replace_bind_group(self.0, bind_group)
    }
}

const MAX_LIGHTS: usize = 10;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightsUniform {
    // using i32 because of the wgsl
    lights_num: i32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    lights: [PointLightUniform; MAX_LIGHTS],
}

#[derive(Debug, Clone)]
pub struct PointLights {
    pub lights: Vec<PointLight>,
}

impl PointLights {
    fn to_uniform(&self) -> PointLightsUniform {
        // TODO refactor this
        let mut lights = [PointLightUniform::default(); MAX_LIGHTS];
        for (i, u) in self
            .lights
            .iter()
            .map(|light| light.to_uniform())
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            lights[i] = u;
        }
        PointLightsUniform {
            lights_num: self.lights.len() as i32,
            lights,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct PointLightsResource {
    pub buffer: Buffer,
}

impl GpuResource for PointLights {
    type ResourceType = PointLightsResource;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let uniform = self.to_uniform();

        let buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("light_uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        Self::ResourceType { buffer }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PointLightsHandle {
    pub buffer_id: ResourceId,
}
impl_resource_handle!(PointLightsHandle, PointLights, PointLightsResource);

#[derive(Debug, Clone, Copy)]
pub struct PointLightsBindGroup(pub ResourceId);

impl AssetBindGroup for PointLightsBindGroup {
    type ResourceHandle = PointLightsHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("binding_group_layout"),
            })
    }

    fn new(
        renderer: &Renderer,
        storage: &mut RenderStorage,
        resource: &Self::ResourceHandle,
    ) -> Self {
        let layout = storage.get_bind_group_layout::<Self>();
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
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
        let buffer = storage.get_buffer(resource.buffer_id);

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        storage.replace_bind_group(self.0, bind_group);
    }
}
