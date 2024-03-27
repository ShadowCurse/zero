use crate::render::prelude::*;
use image::{ImageBuffer, Rgba};

#[cfg(feature = "headless")]
use std::num::NonZeroU32;

pub struct TextureBuffer {
    buffer: Buffer,
    width: u32,
    height: u32,
}

impl TextureBuffer {
    pub fn new(renderer: &Renderer, width: u32, height: u32) -> Self {
        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * width * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let buffer = renderer.device().create_buffer(&output_buffer_desc);
        Self {
            buffer,
            width,
            height,
        }
    }

    #[cfg(feature = "headless")]
    pub fn copy_render_surface_to_texture(&self, renderer: &Renderer) {
        let mut encoder = renderer.create_encoder();

        let u32_size = std::mem::size_of::<u32>() as u32;
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: renderer.surface_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(u32_size * self.width),
                    rows_per_image: NonZeroU32::new(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        renderer.submit(std::iter::once(encoder.finish()));
    }

    pub async fn get_image_buffer(
        &self,
        renderer: &Renderer<'_>,
    ) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let buffer_slice = self.buffer.slice(..);

        buffer_slice.map_async(MapMode::Read, |_| {});
        renderer.device().poll(Maintain::Wait);

        let data = buffer_slice.get_mapped_range().to_owned();
        self.buffer.unmap();
        ImageBuffer::<Rgba<u8>, _>::from_raw(self.width, self.height, data)
    }
}
