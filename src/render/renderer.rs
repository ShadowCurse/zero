use super::wgpu_imports::*;
use winit::{dpi::PhysicalSize, window::Window};

/// Contains the context of the current frame surface
#[derive(Debug)]
pub struct CurrentFrameContext {
    view: TextureView,
    output: Option<SurfaceTexture>,
}

impl CurrentFrameContext {
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    pub fn present(self) {
        match self.output {
            Some(surf) => surf.present(),
            None => {}
        }
    }
}

/// Surface for headless and not headless renderer
#[derive(Debug)]
enum RenderSurface {
    WindowSurface {
        config: SurfaceConfiguration,
        surface: Surface,
    },
    TextureSurface {
        texture: Texture,
    },
}

/// Main renderer struct
#[derive(Debug)]
pub struct Renderer {
    device: Device,
    queue: Queue,
    render_surface: RenderSurface,
    size: PhysicalSize<u32>,
}

impl Renderer {
    /// Creates new [`Renderer`] instance attached to the provided window
    pub async fn new(window: &Window) -> Self {
        let instance = Instance::new(Backends::all());

        let size = window.inner_size();
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits {
                        max_bind_groups: 8,
                        ..Default::default()
                    },
                    label: Some("device_descriptor"),
                },
                None,
            )
            .await
            .unwrap();

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let render_surface = RenderSurface::WindowSurface { config, surface };

        Self {
            device,
            queue,
            render_surface,
            size,
        }
    }

    pub async fn new_headless(width: u32, height: u32) -> Self {
        let instance = Instance::new(Backends::all());

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits {
                        max_bind_groups: 8,
                        ..Default::default()
                    },
                    label: Some("device_descriptor"),
                },
                None,
            )
            .await
            .unwrap();

        let size = PhysicalSize { width, height };
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };
        let texture = device.create_texture(&texture_desc);

        let render_surface = RenderSurface::TextureSurface { texture };

        Self {
            device,
            queue,
            render_surface,
            size,
        }
    }

    /// Size of current surface
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Size of current surface
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Size of current surface
    pub fn size(&self) -> &PhysicalSize<u32> {
        &self.size
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        match &self.render_surface {
            RenderSurface::WindowSurface { config, .. } => config.format,
            RenderSurface::TextureSurface { .. } => wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }

    /// Reconfigures current surface with new size if provided.
    /// Otherwise reconfigures with old size (used when [`SurfaceError::Lost`] is recieved)
    pub fn resize(&mut self, new_size: Option<PhysicalSize<u32>>) {
        if let Some(new_size) = new_size {
            if 0 < new_size.width && 0 < new_size.height {
                self.resize_surface(new_size);
            }
        }
        self.resize_surface(self.size);
    }

    pub fn resize_surface(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        match &mut self.render_surface {
            RenderSurface::WindowSurface { config, surface } => {
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(&self.device, config);
            }
            RenderSurface::TextureSurface { texture } => {
                let texture_desc = wgpu::TextureDescriptor {
                    size: wgpu::Extent3d {
                        width: self.size.width,
                        height: self.size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    label: None,
                };
                *texture = self.device.create_texture(&texture_desc);
            }
        }
    }

    /// Returns context for the current frame
    pub fn current_frame(&self) -> Result<CurrentFrameContext, SurfaceError> {
        let context = match &self.render_surface {
            RenderSurface::WindowSurface { surface, .. } => {
                let output = surface.get_current_texture()?;
                let view = output
                    .texture
                    .create_view(&TextureViewDescriptor::default());
                CurrentFrameContext {
                    view,
                    output: Some(output),
                }
            }
            RenderSurface::TextureSurface { texture } => CurrentFrameContext {
                view: texture.create_view(&TextureViewDescriptor::default()),
                output: None,
            },
        };
        Ok(context)
    }

    /// Creates command encoder
    pub fn create_encoder(&self) -> CommandEncoder {
        self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render_encoder"),
            })
    }

    /// Submits commands to the queue
    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(&self, command_buffers: I) {
        self.queue.submit(command_buffers);
    }
}
