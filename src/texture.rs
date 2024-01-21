use crate::render::prelude::*;
use image::{GenericImageView, ImageError};
use log::info;
use std::path::Path;
use wgpu::StoreOp;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl From<([f32; 3], [f32; 2])> for TextureVertex {
    fn from(data: ([f32; 3], [f32; 2])) -> Self {
        Self {
            position: data.0,
            tex_coords: data.1,
        }
    }
}

impl VertexLayout for TextureVertex {
    fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct GpuTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl GpuTexture {
    pub fn color_attachment(&self) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextureType {
    Diffuse,
    Normal,
}

#[derive(Debug)]
pub struct ImageTexture {
    texture_type: TextureType,
    texture: Option<image::RgbaImage>,
    dimensions: Option<(u32, u32)>,
}

impl ImageTexture {
    pub fn load<P: AsRef<Path>>(path: P, texture_type: TextureType) -> Result<Self, ImageError> {
        info!("loading texture from {:#?}", path.as_ref().to_path_buf());
        let img = image::open(path)?;

        Ok(Self {
            texture_type,
            texture: Some(img.to_rgba8()),
            dimensions: Some(img.dimensions()),
        })
    }
}

impl GpuResource for ImageTexture {
    type ResourceType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture_size = if let Some(dimensions) = self.dimensions {
            Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            }
        } else {
            Extent3d {
                width: renderer.size().width,
                height: renderer.size().height,
                depth_or_array_layers: 1,
            }
        };

        let texture = renderer.device().create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: match self.texture_type {
                TextureType::Diffuse => TextureFormat::Rgba8UnormSrgb,
                TextureType::Normal => TextureFormat::Rgba8Unorm,
            },
            view_formats: match self.texture_type {
                TextureType::Diffuse => &[TextureFormat::Rgba8UnormSrgb],
                TextureType::Normal => &[TextureFormat::Rgba8Unorm],
            },
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("texture"),
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

        if let Some(data) = &self.texture {
            renderer.queue().write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_size.width),
                    rows_per_image: Some(texture_size.height),
                },
                texture_size,
            );
        }

        Self::ResourceType {
            texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug)]
pub struct EmptyTexture {
    pub dimensions: Option<(u32, u32)>,
    pub format: TextureFormat,
    pub filtered: bool,
}

impl EmptyTexture {
    pub fn new_depth() -> Self {
        Self {
            dimensions: None,
            format: TextureFormat::Depth32Float,
            filtered: true,
        }
    }
}

impl GpuResource for EmptyTexture {
    type ResourceType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture_size = if let Some(dimensions) = self.dimensions {
            Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            }
        } else {
            Extent3d {
                width: renderer.size().width,
                height: renderer.size().height,
                depth_or_array_layers: 1,
            }
        };
        let desc = TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            view_formats: &[self.format],
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            label: None,
        };
        let texture = renderer.device().create_texture(&desc);

        let view = texture.create_view(&TextureViewDescriptor::default());

        let filter_mode = if self.filtered {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        };
        let sampler = renderer.device().create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: FilterMode::Nearest,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self::ResourceType {
            texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug)]
pub struct CubeMap {
    pub format: TextureFormat,
    pub texture: Option<Vec<u8>>,
    pub dimensions: Option<(u32, u32)>,
}

impl CubeMap {
    pub fn load<P: AsRef<Path>>(paths: [P; 6]) -> Result<Self, ImageError> {
        let mut texture_data = Vec::new();
        let mut dimensions = (0, 0);
        for path in paths {
            let path_copy = path.as_ref().to_path_buf();
            info!("Loading texture from {:#?}", path_copy);
            let img = image::open(path)?;
            dimensions = img.dimensions();
            texture_data.extend(img.to_rgba8().into_raw());
        }

        Ok(Self {
            format: TextureFormat::Rgba8UnormSrgb,
            texture: Some(texture_data),
            dimensions: Some(dimensions),
        })
    }
}

impl GpuResource for CubeMap {
    type ResourceType = GpuTexture;

    fn build(&self, renderer: &Renderer) -> Self::ResourceType {
        let texture_size = if let Some(dimensions) = self.dimensions {
            Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 6,
            }
        } else {
            Extent3d {
                width: renderer.size().width,
                height: renderer.size().height,
                depth_or_array_layers: 6,
            }
        };

        let texture = renderer.device().create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            view_formats: &[self.format],
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some("cube_texture"),
        });

        let view = texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });
        let sampler = renderer.device().create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        if let Some(data) = &self.texture {
            renderer.queue().write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_size.width),
                    rows_per_image: Some(texture_size.height),
                },
                texture_size,
            );
        }

        Self::ResourceType {
            texture,
            view,
            sampler,
        }
    }
}
