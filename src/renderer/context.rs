use super::wgpu_imports::*;
use winit::{dpi::PhysicalSize, window::Window};

/// Contains texture and view of the current frame
#[derive(Debug)]
pub struct CurrentFrameContext {
    pub output: SurfaceTexture,
    pub view: TextureView,
}

/// Main renderer struct
#[derive(Debug)]
pub struct Renderer {
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
}

impl Renderer {
    /// Creates new [`Renderer`] instance attached to the provided window
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(Backends::all());
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

        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    /// Reconfigures current surface with new size if provided.
    /// Otherwise reconfigures with old size (used when [`SurfaceError::Lost`] is recieved)
    pub fn resize(&mut self, new_size: Option<PhysicalSize<u32>>) {
        if let Some(new_size) = new_size {
            if 0 < new_size.width && 0 < new_size.height {
                self.size = new_size;
                self.config.width = new_size.width;
                self.config.height = new_size.height;
            }
        }
        self.surface.configure(&self.device, &self.config);
    }

    /// Returns context for the current frame
    pub fn current_frame(&self) -> Result<CurrentFrameContext, SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        Ok(CurrentFrameContext { output, view })
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
