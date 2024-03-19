use wgpu::StoreOp;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};
use zero::{const_vec, impl_simple_buffer, prelude::*};

struct FpsLogger {
    last_log: std::time::Instant,
}

impl FpsLogger {
    fn new() -> Self {
        Self {
            last_log: std::time::Instant::now(),
        }
    }

    fn log(&mut self, now: std::time::Instant, dt: std::time::Duration) {
        if 1.0 <= (now - self.last_log).as_secs_f32() {
            println!(
                "Frame time: {:.2}ms(FPS: {:.2})",
                dt.as_secs_f64() * 1000.0,
                1.0 / dt.as_secs_f64()
            );
            self.last_log = now;
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    time: f32,
}

impl From<&Time> for TimeUniform {
    fn from(value: &Time) -> Self {
        Self { time: value.time }
    }
}

#[derive(Debug)]
pub struct Time {
    time: f32,
}

impl_simple_buffer!(
    Time,
    TimeUniform,
    TimeResources,
    TimeHandle,
    TimeBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    storage.register_bind_group_layout::<TimeBindGroup>(&renderer);

    let pipeline = PipelineBuilder {
        shader_path: "./examples/raymarching/raymarching.wgsl",
        label: None,
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<TimeBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[MeshVertex::layout()],
        vertex_entry_point: "vs_main",
        color_targets: Some(&[Some(ColorTargetState {
            format: renderer.surface_format(),
            blend: None,
            write_mask: ColorWrites::ALL,
        })]),
        fragment_entry_point: "fs_main",
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            polygon_mode: PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        multiview: None,
    }
    .build(&renderer);
    let pipeline_id = storage.insert_pipeline(pipeline);

    let depth_texture_id = storage.insert_texture(EmptyTexture::new_depth().build(&renderer));

    let phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        Some(DepthStencil {
            view_id: depth_texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
    );

    let mut camera = Camera::Perspective(PerspectiveCamera {
        position: (-10.0, 0.0, 0.0).into(),
        yaw: Deg(0.0).into(),
        pitch: Deg(0.0).into(),
        aspect: renderer.size().width as f32 / renderer.size().height as f32,
        fovy: Deg(90.0).into(),
        znear: 0.1,
        zfar: 100.0,
    });
    let camera_handle = CameraHandle::new(&mut storage, camera.build(&renderer));
    let camera_bind_group = CameraBindGroup::new(&renderer, &mut storage, &camera_handle);

    let mut camera_controller = CameraController::new(5.0, 0.7);

    let mut time = Time { time: 0.0 };
    let time_handle = TimeHandle::new(&mut storage, time.build(&renderer));
    let time_bind_group = TimeBindGroup::new(&renderer, &mut storage, &time_handle);

    let mesh: Mesh = Quad::new(2.0, 2.0).into();
    let mesh_id = storage.insert_mesh(mesh.build(&renderer));

    let mut last_render_time = std::time::Instant::now();
    let mut fps_logger = FpsLogger::new();
    _ = event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::DeviceEvent { ref event, .. } => {
                if let DeviceEvent::MouseMotion { delta } = event {
                    camera_controller.process_mouse(delta.0, delta.1);
                }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => camera_controller.set_mouse_active(*state == ElementState::Pressed),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: key,
                            state,
                            ..
                        },
                    ..
                } => match key {
                    Key::Named(NamedKey::Escape) => target.exit(),
                    k => _ = camera_controller.process_key(k.clone(), *state),
                },
                WindowEvent::Resized(physical_size) => {
                    camera.resize(physical_size.width, physical_size.height);
                    renderer.resize(Some(*physical_size));
                    storage.replace_texture(
                        depth_texture_id,
                        EmptyTexture::new_depth().build(&renderer),
                    );
                }
                WindowEvent::RedrawRequested => {
                    let now = std::time::Instant::now();
                    let dt = now - last_render_time;
                    last_render_time = now;

                    fps_logger.log(now, dt);

                    time.time += dt.as_secs_f32();
                    time_handle.update(&renderer, &storage, &time);

                    camera_controller.update_camera(&mut camera, dt);
                    camera_handle.update(&renderer, &storage, &camera);

                    let current_frame_context = match renderer.current_frame() {
                        Ok(cfc) => cfc,
                        Err(SurfaceError::Lost) => {
                            renderer.resize(None);
                            return;
                        }
                        Err(SurfaceError::OutOfMemory) => {
                            target.exit();
                            return;
                        }
                        Err(e) => {
                            eprintln!("{:?}", e);
                            return;
                        }
                    };

                    let current_frame_storage = CurrentFrameStorage {
                        storage: &storage,
                        current_frame_view: current_frame_context.view(),
                    };

                    let mut encoder = renderer.create_encoder();

                    let command = MeshRenderCommand {
                        pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![camera_bind_group.0, time_bind_group.0],
                    };

                    {
                        let mut render_pass =
                            phase.render_pass(&mut encoder, &current_frame_storage);
                        command.execute(&mut render_pass, &current_frame_storage);
                    }

                    let commands = encoder.finish();
                    renderer.submit(std::iter::once(commands));
                    current_frame_context.present();
                }
                _ => {}
            },
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        }
    });
}
