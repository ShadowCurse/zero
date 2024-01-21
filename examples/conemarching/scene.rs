use wgpu::StoreOp;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};
use zero::{const_vec, impl_simple_buffer, impl_simple_texture_bind_group, prelude::*};

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

#[derive(Debug)]
pub struct DepthResource {
    texture: GpuTexture,
}

#[derive(Debug, Clone, Copy)]
pub struct DepthHandle {
    pub texture_id: ResourceId,
}

impl ResourceHandle for DepthHandle {
    type OriginalResource<'a> = EmptyTexture;
    type ResourceType = DepthResource;

    fn new(storage: &mut RenderStorage, resource: Self::ResourceType) -> Self {
        Self {
            texture_id: storage.insert_texture(resource.texture),
        }
    }

    fn replace(&self, storage: &mut RenderStorage, resource: Self::ResourceType) {
        storage.replace_texture(self.texture_id, resource.texture);
    }
}

impl_simple_texture_bind_group!(
    DepthHandle,
    DepthBindGroup,
    { TextureViewDimension::D2 },
    { TextureSampleType::Float { filterable: false } },
    { SamplerBindingType::NonFiltering }
);

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut render_system = RenderSystem::default();
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    storage.register_bind_group_layout::<TimeBindGroup>(&renderer);
    storage.register_bind_group_layout::<DepthBindGroup>(&renderer);

    let depth_prepass_pipeline = PipelineBuilder {
        shader_path: "./examples/conemarching/depth_prepass.wgsl",
        label: Some("depth_prepass_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<TimeBindGroup>(),
                storage.get_bind_group_layout::<DepthBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[MeshVertex::layout()],
        vertex_entry_point: "vs_main",
        color_targets: Some(&[Some(ColorTargetState {
            format: TextureFormat::R32Float,
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
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    }
    .build(&renderer);
    let depth_prepass_pipeline_id = storage.insert_pipeline(depth_prepass_pipeline);

    let final_pipeline = PipelineBuilder {
        shader_path: "./examples/conemarching/conemarching.wgsl",
        label: Some("final_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<TimeBindGroup>(),
                storage.get_bind_group_layout::<DepthBindGroup>(),
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
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    }
    .build(&renderer);
    let final_pipeline_id = storage.insert_pipeline(final_pipeline);

    let depth_0 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((16, 16)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_0_handle = DepthHandle::new(&mut storage, depth_0);
    let depth_0_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_0_handle);

    let depth_1 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((32, 32)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_1_handle = DepthHandle::new(&mut storage, depth_1);
    let depth_1_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_1_handle);

    let depth_2 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((64, 64)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_2_handle = DepthHandle::new(&mut storage, depth_2);
    let depth_2_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_2_handle);

    let depth_3 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((128, 128)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_3_handle = DepthHandle::new(&mut storage, depth_3);
    let depth_3_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_3_handle);

    let depth_4 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((256, 256)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_4_handle = DepthHandle::new(&mut storage, depth_4);
    let depth_4_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_4_handle);

    let depth_5 = DepthResource {
        texture: EmptyTexture {
            dimensions: Some((512, 512)),
            format: TextureFormat::R32Float,
            filtered: false,
        }
        .build(&renderer),
    };
    let depth_5_handle = DepthHandle::new(&mut storage, depth_5);
    let depth_5_bind_group = DepthBindGroup::new(&renderer, &mut storage, &depth_5_handle);

    let phase_1 = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: depth_1_handle.texture_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let phase_1_id = render_system.add_phase(phase_1);

    let phase_2 = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: depth_2_handle.texture_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let phase_2_id = render_system.add_phase(phase_2);

    let phase_3 = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: depth_3_handle.texture_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let phase_3_id = render_system.add_phase(phase_3);

    let phase_4 = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: depth_4_handle.texture_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let phase_4_id = render_system.add_phase(phase_4);

    let phase_5 = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: depth_5_handle.texture_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let phase_5_id = render_system.add_phase(phase_5);

    let final_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        },],
        None,
    );
    let final_phase_id = render_system.add_phase(final_phase);

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

    let mut time = Time { time: 0.0 };
    let time_handle = TimeHandle::new(&mut storage, time.build(&renderer));
    let time_bind_group = TimeBindGroup::new(&renderer, &mut storage, &time_handle);

    let mesh: Mesh = Quad::new((2.0, 2.0)).into();
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

                    let depth_1_command = RenderCommand {
                        pipeline_id: depth_prepass_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_0_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_1_id, depth_1_command);

                    let depth_2_command = RenderCommand {
                        pipeline_id: depth_prepass_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_1_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_2_id, depth_2_command);

                    let depth_3_command = RenderCommand {
                        pipeline_id: depth_prepass_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_2_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_3_id, depth_3_command);

                    let depth_4_command = RenderCommand {
                        pipeline_id: depth_prepass_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_3_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_4_id, depth_4_command);

                    let depth_5_command = RenderCommand {
                        pipeline_id: depth_prepass_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_4_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(phase_5_id, depth_5_command);

                    let command = RenderCommand {
                        pipeline_id: final_pipeline_id,
                        mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            camera_bind_group.0,
                            time_bind_group.0,
                            depth_5_bind_group.0
                        ],
                    };
                    render_system.add_phase_command(final_phase_id, command);

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
