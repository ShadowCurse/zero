use super::wgpu_imports::*;
use winit::{dpi::PhysicalSize, window::Window};

trait IntoTextureDescriptor {
    fn texture_descriptor(&self) -> TextureDescriptor;
}

impl IntoTextureDescriptor for SurfaceConfiguration {
    fn texture_descriptor(&self) -> TextureDescriptor {
        TextureDescriptor {
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        }
    }
}

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
    Surface(Surface),
    Texture(Texture),
}

/// Main renderer struct
#[derive(Debug)]
pub struct Renderer {
    device: Device,
    queue: Queue,
    render_surface: RenderSurface,
    config: SurfaceConfiguration,
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

        let render_surface = RenderSurface::Surface(surface);

        Self {
            device,
            queue,
            render_surface,
            config,
            size,
        }
    }

    /// Creates new headless [`Renderer`] instance with internal texture with provided size 
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

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Rgba8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };
        let texture_desc = config.texture_descriptor();
        let texture = device.create_texture(&texture_desc);

        let render_surface = RenderSurface::Texture(texture);

        Self {
            device,
            queue,
            render_surface,
            config,
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
        self.config.format
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
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        match &mut self.render_surface {
            RenderSurface::Surface(surface) => {
                surface.configure(&self.device, &self.config);
            }
            RenderSurface::Texture(texture) => {
                *texture = self.device.create_texture(&self.config.texture_descriptor());
            }
        }
    }

    /// Returns context for the current frame
    pub fn current_frame(&self) -> Result<CurrentFrameContext, SurfaceError> {
        let context = match &self.render_surface {
            RenderSurface::Surface(surface) => {
                let output = surface.get_current_texture()?;
                let view = output
                    .texture
                    .create_view(&TextureViewDescriptor::default());
                CurrentFrameContext {
                    view,
                    output: Some(output),
                }
            }
            RenderSurface::Texture(texture) => CurrentFrameContext {
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
