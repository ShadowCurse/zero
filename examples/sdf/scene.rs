use wgpu::StoreOp;
use winit::{
    dpi::LogicalSize,
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
pub struct ScreenInfoUniform {
    size: [f32; 2],
}

impl From<&ScreenInfo> for ScreenInfoUniform {
    fn from(value: &ScreenInfo) -> Self {
        Self {
            size: [value.width, value.height],
        }
    }
}

#[derive(Debug)]
pub struct ScreenInfo {
    width: f32,
    height: f32,
}

impl_simple_buffer!(
    ScreenInfo,
    ScreenInfoUniform,
    ScreenInfoResources,
    ScreenInfoHandle,
    ScreenInfoBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut render_system = RenderSystem::default();
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    storage.register_bind_group_layout::<TransformBindGroup>(&renderer);
    storage.register_bind_group_layout::<ScreenInfoBindGroup>(&renderer);

    let pipeline = PipelineBuilder {
        shader_path: "./examples/sdf/sdf.wgsl",
        label: None,
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<TransformBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<ScreenInfoBindGroup>(),
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
            format: DepthTexture::DEPTH_FORMAT,
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

    let depth_texture_id = storage.insert_texture(DepthTexture::default().build(&renderer));

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
    let phase_id = render_system.add_phase(phase);

    let mut camera = Camera::new(
        (-10.0, 0.0, 0.0),
        Deg(0.0),
        Deg(0.0),
        renderer.size().width,
        renderer.size().height,
        Deg(90.0),
        0.1,
        100.0,
    );
    let camera_handle = CameraHandle::new(&mut storage, camera.build(&renderer));
    let camera_bind_group = CameraBindGroup::new(&renderer, &mut storage, &camera_handle);

    let mut camera_controller = CameraController::new(5.0, 0.7);

    let mut screen_info = ScreenInfo {
        width: 400.0,
        height: 400.0,
    };
    let screen_info_handle = ScreenInfoHandle::new(&mut storage, screen_info.build(&renderer));
    let screen_info_bind_group =
        ScreenInfoBindGroup::new(&renderer, &mut storage, &screen_info_handle);

    let quad_mesh: Mesh = Quad::flipped((10.0, 10.0)).into();
    let quad_id = storage.insert_mesh(quad_mesh.build(&renderer));

    let quad_transform = Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(90.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let quad_transform_handle = TransformHandle::new(&mut storage, quad_transform.build(&renderer));
    let quad_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &quad_transform_handle);

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
                        DepthTexture::default().build(&renderer),
                    );
                    screen_info.height = physical_size.height as f32;
                    screen_info.width = physical_size.width as f32;
                    screen_info_handle.update(&renderer, &storage, &screen_info);
                }
                WindowEvent::RedrawRequested => {
                    let now = std::time::Instant::now();
                    let dt = now - last_render_time;
                    last_render_time = now;

                    fps_logger.log(now, dt);

                    camera_controller.update_camera(&mut camera, dt);
                    camera_handle.update(&renderer, &storage, &camera);

                    let box1 = RenderCommand {
                        pipeline_id,
                        mesh_id: quad_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            quad_transform_bind_group.0,
                            camera_bind_group.0,
                            screen_info_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_id, box1);

                    match render_system.run(&renderer, &storage) {
                        Ok(_) => {}
                        Err(SurfaceError::Lost) => renderer.resize(None),
                        Err(SurfaceError::OutOfMemory) => target.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            },
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        }
    });
}
