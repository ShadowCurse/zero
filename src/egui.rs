use std::{borrow::Cow, collections::HashMap, num::NonZeroU64};

use wgpu::BufferDescriptor;

use crate::{
    const_vec, impl_simple_buffer, impl_simple_sized_gpu_buffer, impl_simple_texture_bind_group,
    mesh::{Mesh, MeshRenderCommand},
    render::prelude::*,
    texture::GpuTexture,
    utils::ConstVec,
};

pub struct EguiRenderContext {
    mesh_id: ResourceId,
    index_buffer_slices: Vec<std::ops::Range<u64>>,
    vertex_buffer_slices: Vec<std::ops::Range<u64>>,

    textures: HashMap<egui::TextureId, (EguiTextureHandle, EguiTextureBindGroup)>,

    screen_size: [f32; 2],
    uniform_buffer_handle: EguiBufferHandle,
    uniform_buffer_bind_group: EguiBufferBindGroup,
}

impl EguiRenderContext {
    pub fn new(renderer: &Renderer, storage: &mut RenderStorage) -> Self {
        let egui_buffer = EguiBuffer {
            screen_size: [renderer.size().width as f32, renderer.size().height as f32],
        };
        let buffer_handle = EguiBufferHandle::new(storage, egui_buffer.build(renderer));
        let buffer_bind_group = EguiBufferBindGroup::new(renderer, storage, &buffer_handle);

        Self {
            mesh_id: storage.insert_mesh(
                Mesh {
                    name: "".to_owned(),
                    vertices: vec![],
                    indices: vec![],
                }
                .build(renderer),
            ),
            index_buffer_slices: Vec::new(),
            vertex_buffer_slices: Vec::new(),
            textures: HashMap::new(),
            screen_size: Default::default(),
            uniform_buffer_handle: buffer_handle,
            uniform_buffer_bind_group: buffer_bind_group,
        }
    }

    pub fn update_textures(
        &mut self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        textures_delta: egui::TexturesDelta,
    ) {
        for (texture_id, imgae_delta) in textures_delta.set {
            self.create_or_update_texture(renderer, storage, texture_id, imgae_delta);
        }
        for texture_id in textures_delta.free {
            println!("freeing texture: {:?}", texture_id);
            // TODO
            // self.free_texture(f);
        }
    }

    fn create_or_update_texture(
        &mut self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        texture_id: egui::TextureId,
        image_delta: egui::epaint::ImageDelta,
    ) {
        let width = image_delta.image.width() as u32;
        let height = image_delta.image.height() as u32;

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let data_color32 = match &image_delta.image {
            egui::epaint::ImageData::Color(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Borrowed(&image.pixels)
            }
            egui::epaint::ImageData::Font(image) => {
                assert_eq!(
                    width as usize * height as usize,
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                Cow::Owned(image.srgba_pixels(None).collect::<Vec<_>>())
            }
        };
        let data_bytes: &[u8] = bytemuck::cast_slice(data_color32.as_slice());

        if let Some(pos) = image_delta.pos {
            // update the existing texture
            let (texture_handle, _texture_bind_group) = self
                .textures
                .get(&texture_id)
                .expect("Tried to update a texture that has not been allocated yet.");
            let origin = wgpu::Origin3d {
                x: pos[0] as u32,
                y: pos[1] as u32,
                z: 0,
            };
            let texture = EguiTexture {
                texture: data_bytes,
                size: texture_size,
                origin,
            };
            texture_handle.update(renderer, storage, &texture);
        } else {
            // allocate a new texture
            let origin = wgpu::Origin3d::ZERO;
            let texture = EguiTexture {
                texture: data_bytes,
                size: texture_size,
                origin,
            };
            let texture_handle = EguiTextureHandle::new(storage, texture.build(renderer));
            let texture_bind_group = EguiTextureBindGroup::new(renderer, storage, &texture_handle);
            self.textures
                .insert(texture_id, (texture_handle, texture_bind_group));
        };
    }

    pub fn update_meshes(
        &mut self,
        renderer: &Renderer,
        storage: &mut RenderStorage,
        primitives: &[egui::epaint::ClippedPrimitive],
    ) {
        let current_screen_size = [renderer.size().width as f32, renderer.size().height as f32];
        if self.screen_size != current_screen_size {
            let egui_buffer = EguiBuffer {
                screen_size: current_screen_size,
            };
            self.uniform_buffer_handle
                .update(renderer, storage, &egui_buffer);
            self.screen_size = current_screen_size;
        }

        // Determine how many vertices & indices need to be rendered.
        let (vertex_count, index_count) = {
            primitives.iter().fold((0, 0), |acc, clipped_primitive| {
                match &clipped_primitive.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        (acc.0 + mesh.vertices.len(), acc.1 + mesh.indices.len())
                    }
                    egui::epaint::Primitive::Callback(_) => acc,
                }
            })
        };

        let mesh = storage.get_mesh_mut(self.mesh_id);

        if index_count > 0 {
            self.index_buffer_slices.clear();
            let required_index_buffer_size = (std::mem::size_of::<u32>() * index_count) as u64;
            if mesh.index_buffer.as_ref().unwrap().size() < required_index_buffer_size {
                // Resize index buffer if needed.
                let size = (mesh.index_buffer.as_ref().unwrap().size() * 2)
                    .max(required_index_buffer_size);
                mesh.index_buffer = Some(EguiIndexBuffer { size }.build(renderer).buffer);
                mesh.num_elements = index_count as u32;
            }

            let mut index_buffer_staging = renderer
                .queue()
                .write_buffer_with(
                    mesh.index_buffer.as_ref().unwrap(),
                    0,
                    NonZeroU64::new(required_index_buffer_size).unwrap(),
                )
                .expect("Failed to create staging buffer for index data");
            let mut index_offset = 0;
            for egui::epaint::ClippedPrimitive { primitive, .. } in primitives.iter() {
                match primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        let size = mesh.indices.len() * std::mem::size_of::<u32>();
                        index_buffer_staging[index_offset..(size + index_offset)]
                            .copy_from_slice(bytemuck::cast_slice(&mesh.indices));
                        self.index_buffer_slices
                            .push(index_offset as u64..(size + index_offset) as u64);
                        index_offset += size;
                    }
                    egui::epaint::Primitive::Callback(_) => {}
                }
            }
        }
        if vertex_count > 0 {
            self.vertex_buffer_slices.clear();
            let required_vertex_buffer_size =
                (std::mem::size_of::<EguiVertex>() * vertex_count) as u64;
            if mesh.vertex_buffer.size() < required_vertex_buffer_size {
                // Resize vertex buffer if needed.
                let size = (mesh.vertex_buffer.size() * 2).max(required_vertex_buffer_size);
                mesh.vertex_buffer = EguiVertexBuffer { size }.build(renderer).buffer;
            }

            let mut vertex_buffer_staging = renderer
                .queue()
                .write_buffer_with(
                    &mesh.vertex_buffer,
                    0,
                    NonZeroU64::new(required_vertex_buffer_size).unwrap(),
                )
                .expect("Failed to create staging buffer for vertex data");
            let mut vertex_offset = 0;
            for egui::epaint::ClippedPrimitive { primitive, .. } in primitives.iter() {
                match primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        let size = mesh.vertices.len() * std::mem::size_of::<EguiVertex>();
                        vertex_buffer_staging[vertex_offset..(size + vertex_offset)]
                            .copy_from_slice(bytemuck::cast_slice(&mesh.vertices));
                        self.vertex_buffer_slices
                            .push(vertex_offset as u64..(size + vertex_offset) as u64);
                        vertex_offset += size;
                    }
                    egui::epaint::Primitive::Callback(_) => {}
                }
            }
        }
    }

    pub fn create_commands(
        &self,
        pipeline_id: ResourceId,
        primitives: &[egui::epaint::ClippedPrimitive],
    ) -> Vec<MeshRenderCommand> {
        primitives
            .iter()
            .zip(self.index_buffer_slices.iter())
            .zip(self.vertex_buffer_slices.iter())
            .filter_map(
                |(
                    (
                        egui::epaint::ClippedPrimitive {
                            clip_rect,
                            primitive,
                        },
                        index_slice,
                    ),
                    vertex_slice,
                )| {
                    match primitive {
                        egui::epaint::Primitive::Mesh(mesh) => {
                            let (_, texture_bind_group) =
                                self.textures.get(&mesh.texture_id).unwrap();

                            let rect = ScissorRect::new(clip_rect, 1.0, self.screen_size);
                            if rect.width == 0 || rect.height == 0 {
                                // Skip rendering zero-sized clip areas.
                                return None;
                            }

                            Some(MeshRenderCommand {
                                pipeline_id,
                                mesh_id: self.mesh_id,
                                index_slice: Some(index_slice.clone()),
                                vertex_slice: Some(vertex_slice.clone()),
                                scissor_rect: Some([rect.x, rect.y, rect.width, rect.height]),
                                bind_groups: const_vec![
                                    self.uniform_buffer_bind_group.0,
                                    texture_bind_group.0
                                ],
                            })
                        }
                        egui::epaint::Primitive::Callback(_) => None,
                    }
                },
            )
            .collect()
    }
}

struct ScissorRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl ScissorRect {
    fn new(clip_rect: &egui::epaint::Rect, pixels_per_point: f32, target_size: [f32; 2]) -> Self {
        // Transform clip rect to physical pixels:
        let clip_min_x = pixels_per_point * clip_rect.min.x;
        let clip_min_y = pixels_per_point * clip_rect.min.y;
        let clip_max_x = pixels_per_point * clip_rect.max.x;
        let clip_max_y = pixels_per_point * clip_rect.max.y;

        // Round to integer:
        let clip_min_x = clip_min_x.round() as u32;
        let clip_min_y = clip_min_y.round() as u32;
        let clip_max_x = clip_max_x.round() as u32;
        let clip_max_y = clip_max_y.round() as u32;

        // Clamp:
        let clip_min_x = clip_min_x.clamp(0, target_size[0] as u32);
        let clip_min_y = clip_min_y.clamp(0, target_size[1] as u32);
        let clip_max_x = clip_max_x.clamp(clip_min_x, target_size[0] as u32);
        let clip_max_y = clip_max_y.clamp(clip_min_y, target_size[1] as u32);

        Self {
            x: clip_min_x,
            y: clip_min_y,
            width: clip_max_x - clip_min_x,
            height: clip_max_y - clip_min_y,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EguiVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: u32,
}

impl VertexLayout for EguiVertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct EguiTexture<'a> {
    texture: &'a [u8],
    size: Extent3d,
    origin: Origin3d,
}

impl<'a> GpuResource for EguiTexture<'a> {
    type ResourceType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture = renderer.device().create_texture(&TextureDescriptor {
            size: self.size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            view_formats: &[TextureFormat::Rgba8UnormSrgb],
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("egui_texture"),
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = renderer.device().create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        renderer.queue().write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: self.origin,
                aspect: TextureAspect::All,
            },
            self.texture,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.size.width),
                rows_per_image: Some(self.size.height),
            },
            self.size,
        );

        Self::ResourceType {
            texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EguiTextureHandle {
    pub texture_id: ResourceId,
}

impl ResourceHandle for EguiTextureHandle {
    type OriginalResource<'a> = EguiTexture<'a>;
    type ResourceType = GpuTexture;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            texture_id: storage.insert_texture(resource),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.texture_id, resource);
    }

    fn update(
        &self,
        renderer: &Renderer,
        storage: &RenderStorage,
        original: &Self::OriginalResource<'_>,
    ) {
        let texture = storage.get_texture(self.texture_id);
        renderer.queue().write_texture(
            ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: original.origin,
                aspect: TextureAspect::All,
            },
            original.texture,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * original.size.width),
                rows_per_image: Some(original.size.height),
            },
            original.size,
        );
    }
}

impl_simple_texture_bind_group!(
    EguiTextureHandle,
    EguiTextureBindGroup,
    { TextureViewDimension::D2 },
    { TextureSampleType::Float { filterable: true } },
    { SamplerBindingType::Filtering }
);

impl_simple_sized_gpu_buffer!(EguiIndexBuffer, EguiIndexBufferResources, {
    BufferUsages::COPY_DST | BufferUsages::INDEX
});
impl_simple_sized_gpu_buffer!(EguiVertexBuffer, EguiVertexBufferResources, {
    BufferUsages::COPY_DST | BufferUsages::VERTEX
});

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EguiBufferUniform {
    pub screen_size: [f32; 2],
    pub _padding: [u32; 2],
}

impl From<&EguiBuffer> for EguiBufferUniform {
    fn from(value: &EguiBuffer) -> Self {
        EguiBufferUniform {
            screen_size: value.screen_size,
            _padding: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EguiBuffer {
    pub screen_size: [f32; 2],
}

impl_simple_buffer!(
    EguiBuffer,
    EguiBufferUniform,
    EguiBufferResources,
    EguiBufferHandle,
    EguiBufferBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX },
    { BufferBindingType::Uniform }
);
