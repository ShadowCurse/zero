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
    type OriginalResource<'a>;
    type ResourceType;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self;
    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType);
    fn update(
        &self,
        _renderer: &Renderer,
        _storage: &RenderStorage,
        _original: &Self::OriginalResource<'_>,
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

#[macro_export]
macro_rules! impl_simple_sized_gpu_buffer {
    ($buffer:ident, $resource:ident, $usage:block) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $buffer {
            pub size: u64,
        }

        #[derive(Debug)]
        pub struct $resource {
            buffer: Buffer,
        }

        impl GpuResource for $buffer {
            type ResourceType = $resource;

            fn build(&self, renderer: &Renderer) -> Self::ResourceType {
                let buffer = renderer.device().create_buffer(&BufferDescriptor {
                    label: Some(std::any::type_name::<Self>()),
                    usage: $usage,
                    size: self.size,
                    mapped_at_creation: false,
                });
                Self::ResourceType { buffer }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_simple_texture_bind_group {
    ($handle:ty, $bind_group:ident, $view_dimension:block, $sample_type:block, $sampler_binding_type:block) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $bind_group(pub ResourceId);

        impl AssetBindGroup for $bind_group {
            type ResourceHandle = $handle;

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
                                    view_dimension: $view_dimension,
                                    sample_type: $sample_type,
                                },
                                count: None,
                            },
                            BindGroupLayoutEntry {
                                binding: 1,
                                visibility: ShaderStages::FRAGMENT,
                                ty: BindingType::Sampler($sampler_binding_type),
                                count: None,
                            },
                        ],
                        label: Some(std::any::type_name::<Self>()),
                    })
            }

            fn new(
                renderer: &Renderer,
                storage: &mut RenderStorage,
                resource: &Self::ResourceHandle,
            ) -> Self {
                let layout = storage.get_bind_group_layout::<Self>();
                let texture = storage.get_texture(resource.texture_id);

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
                    ],
                    label: Some(std::any::type_name::<Self>()),
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
                let texture = storage.get_texture(resource.texture_id);

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
                    ],
                    label: Some(std::any::type_name::<Self>()),
                });

                storage.replace_bind_group(self.0, bind_group);
            }
        }
    };
}

/// Shorthadn for creating simple `GpuResource` buffer with `ResourceHandle` and `AssetBindGroup`
/// types and traits
/// uniform type need to implement `From<&BufferType>` and `bytemuck` traits
#[macro_export]
macro_rules! impl_simple_buffer {
    ($buffer:ty, $uniform:ty, $resource:ident, $handle:ident, $bind_group:ident, $usage:block, $visibility:block, $buffer_binding_type:block) => {
        #[derive(Debug)]
        pub struct $resource {
            buffer: Buffer,
        }

        impl GpuResource for $buffer {
            type ResourceType = $resource;

            fn build(&self, renderer: &Renderer) -> Self::ResourceType {
                let uniform: $uniform = self.into();
                let buffer = renderer.device().create_buffer_init(&BufferInitDescriptor {
                    label: Some(std::any::type_name::<Self>()),
                    contents: bytemuck::cast_slice(&[uniform]),
                    usage: $usage, // BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
                Self::ResourceType { buffer }
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $handle {
            buffer_id: ResourceId,
        }

        impl ResourceHandle for $handle {
            type OriginalResource<'a> = $buffer;
            type ResourceType = $resource;

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
                original: &Self::OriginalResource<'_>,
            ) {
                let uniform: $uniform = original.into();
                renderer.queue().write_buffer(
                    storage.get_buffer(self.buffer_id),
                    0,
                    bytemuck::cast_slice(&[uniform]),
                );
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $bind_group(pub ResourceId);

        impl AssetBindGroup for $bind_group {
            type ResourceHandle = $handle;

            fn bind_group_layout(renderer: &Renderer) -> BindGroupLayout {
                renderer
                    .device()
                    .create_bind_group_layout(&BindGroupLayoutDescriptor {
                        entries: &[BindGroupLayoutEntry {
                            binding: 0,
                            visibility: $visibility,
                            ty: BindingType::Buffer {
                                ty: $buffer_binding_type,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                        label: Some(std::any::type_name::<Self>()),
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
                    label: Some(std::any::type_name::<Self>()),
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
                    label: Some(std::any::type_name::<Self>()),
                });

                storage.replace_bind_group(self.0, bind_group);
            }
        }
    };
}
