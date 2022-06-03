use crate::cgmath_imports::*;
use crate::render::prelude::*;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformUniform {
    transform: [[f32; 4]; 4],
    rotate: [[f32; 4]; 4],
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    fn to_uniform(&self) -> TransformUniform {
        let rotate = Matrix4::from(self.rotation);
        TransformUniform {
            transform: (Matrix4::from_translation(self.translation)
                * rotate
                * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z))
            .into(),
            rotate: rotate.into(),
        }
    }
}

#[derive(Debug)]
pub struct TransformResources {
    buffer: Buffer,
}

impl GpuResource for Transform {
    type ResourceType = TransformResources;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let uniform = self.to_uniform();
        let buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
            label: Some("transform_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self::ResourceType { buffer }
    }
}

#[derive(Debug)]
pub struct TransformHandle {
    buffer_id: ResourceId,
}

impl ResourceHandle for TransformHandle {
    type OriginalResource = Transform;
    type ResourceType = TransformResources;

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

#[derive(Debug)]
pub struct TransformBindGroup(pub ResourceId);

impl AssetBindGroup for TransformBindGroup {
    type ResourceHandle = TransformHandle;

    fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
        renderer
            .device()
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
                label: Some("transform_bind_group_layout"),
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

        let bind_group = renderer.device().create_bind_group(&BindGroupDescriptor {
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("transform_bind_group"),
        });

        Self(storage.insert_bind_group(bind_group))
    }
}
