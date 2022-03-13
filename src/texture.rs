use anyhow::{Ok, Result};
use image::GenericImageView;
use std::path::Path;

use crate::renderer;

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

impl renderer::Vertex for TextureVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl renderer::GpuResource for GpuTexture {}

#[derive(Debug, Clone, Copy)]
pub enum TextureType {
    Diffuse,
    Normal,
}

#[derive(Debug)]
pub struct Texture {
    texture_type: TextureType,
    texture: image::RgbaImage,
    dimensions: (u32, u32),
}

#[derive(Debug)]
pub struct DepthTexture;

#[derive(Debug)]
pub struct CubeMap {
    texture: Vec<u8>,
    dimensions: (u32, u32),
}

impl Texture {
    pub fn load<P: AsRef<Path>>(path: P, texture_type: TextureType) -> Result<Self> {
        let path_copy = path.as_ref().to_path_buf();

        println!("loading texture from {:#?}", path_copy);
        let img = image::open(path)?;

        Ok(Self {
            texture_type,
            texture: img.to_rgba8(),
            dimensions: img.dimensions(),
        })
    }
}

impl renderer::GpuAsset for Texture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &renderer::Renderer) -> Self::GpuType {
        let texture_size = wgpu::Extent3d {
            width: self.dimensions.0,
            height: self.dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: match self.texture_type {
                TextureType::Diffuse => wgpu::TextureFormat::Rgba8UnormSrgb,
                TextureType::Normal => wgpu::TextureFormat::Rgba8Unorm,
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        renderer.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.texture,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * self.dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(self.dimensions.1),
            },
            texture_size,
        );

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl renderer::GpuAsset for DepthTexture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &renderer::Renderer) -> Self::GpuType {
        let size = wgpu::Extent3d {
            width: renderer.config.width,
            height: renderer.config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("depth_texture"),
        };
        let texture = renderer.device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None, //Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}

impl CubeMap {
    pub fn load<P: AsRef<Path>>(paths: [P; 6]) -> Result<Self> {
        let mut texture_data = Vec::new();
        let mut dimensions = (0, 0);
        for path in paths {
            let path_copy = path.as_ref().to_path_buf();
            println!("loading texture from {:#?}", path_copy);
            let img = image::open(path)?;
            dimensions = img.dimensions();
            texture_data.extend(img.to_rgba8().into_raw());
        }

        Ok(Self {
            texture: texture_data,
            dimensions,
        })
    }
}

impl renderer::GpuAsset for CubeMap {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &renderer::Renderer) -> Self::GpuType {
        let texture_size = wgpu::Extent3d {
            width: self.dimensions.0,
            height: self.dimensions.1,
            depth_or_array_layers: 6,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("cube_texture"),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        renderer.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.texture,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * self.dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(self.dimensions.1),
            },
            texture_size,
        );

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}
