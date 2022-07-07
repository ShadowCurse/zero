use super::wgpu_imports::*;
use log::info;
use winit::dpi::PhysicalSize;

#[cfg(not(feature = "headless"))]
use winit::window::Window;

/// Contains the context of the current frame surface
#[derive(Debug)]
pub struct CurrentFrameContext {
    view: TextureView,
    #[cfg(not(feature = "headless"))]
    output: SurfaceTexture,
}

impl CurrentFrameContext {
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    #[cfg(not(feature = "headless"))]
    pub fn present(self) {
        self.output.present();
    }
}

/// Main renderer struct
#[derive(Debug)]
pub struct Renderer {
    device: Device,
    queue: Queue,

    #[cfg(not(feature = "headless"))]
    surface: Surface,
    #[cfg(not(feature = "headless"))]
    config: SurfaceConfiguration,

    #[cfg(feature = "headless")]
    texture: Texture,

    size: PhysicalSize<u32>,
}

impl Renderer {
    /// Creates new [`Renderer`] instance attached to the provided window
    #[cfg(not(feature = "headless"))]
    pub async fn new(window: &Window) -> Self {
        let instance = Instance::new(Backends::VULKAN);

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
                        max_bind_groups: 4,
                        ..Default::default()
                    },
                    label: Some("device_descriptor"),
                },
                None,
            )
            .await
            .unwrap();

        info!("Renderer device: {:#?}, queue: {:#?}", device, queue);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoNoVsync,
        };
        surface.configure(&device, &config);

        Self {
            device,
            queue,
            surface,
            config,
            size,
        }
    }

    /// Creates new headless [`Renderer`] instance with internal texture with provided size
    #[cfg(feature = "headless")]
    pub async fn new(width: u32, height: u32) -> Self {
        let instance = Instance::new(Backends::VULKAN);

        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits {
                        max_bind_groups: 4,
                        ..Default::default()
                    },
                    label: Some("device_descriptor"),
                },
                None,
            )
            .await
            .unwrap();

        info!("Renderer device: {:#?}, queue: {:#?}", device, queue);

        let size = PhysicalSize { width, height };

        let desc = TextureDescriptor {
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
            label: Some("surface_texture"),
        };
        let texture = device.create_texture(&desc);

        Self {
            device,
            queue,
            texture,
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

    #[cfg(not(feature = "headless"))]
    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    #[cfg(feature = "headless")]
    pub fn surface_format(&self) -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }

    #[cfg(feature = "headless")]
    pub fn surface_texture(&self) -> &Texture {
        &self.texture
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

    #[cfg(not(feature = "headless"))]
    fn resize_surface(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;

        self.surface.configure(&self.device, &self.config);
    }

    #[cfg(feature = "headless")]
    fn resize_surface(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;

        let desc = TextureDescriptor {
            size: Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
            label: Some("surface_texture"),
        };
        self.texture = self.device.create_texture(&desc);
    }

    /// Returns context for the current frame
    #[cfg(not(feature = "headless"))]
    pub fn current_frame(&self) -> Result<CurrentFrameContext, SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let context = CurrentFrameContext { view, output };
        Ok(context)
    }

    /// Returns context for the current frame
    #[cfg(feature = "headless")]
    pub fn current_frame(&self) -> CurrentFrameContext {
        CurrentFrameContext {
            view: self.texture.create_view(&TextureViewDescriptor::default()),
        }
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
