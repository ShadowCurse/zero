use wgpu::StoreOp;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};
use zero::{const_vec, prelude::*};

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

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    storage.register_bind_group_layout::<MaterialBindGroup>(&renderer);
    storage.register_bind_group_layout::<ColorMaterialBindGroup>(&renderer);
    storage.register_bind_group_layout::<PointLightBindGroup>(&renderer);
    storage.register_bind_group_layout::<TransformBindGroup>(&renderer);

    let color_pipeline = PipelineBuilder {
        shader_path: "./examples/forward/color.wgsl",
        label: Some("color_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<ColorMaterialBindGroup>(),
                storage.get_bind_group_layout::<TransformBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
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
    let color_pipeline_id = storage.insert_pipeline(color_pipeline);

    let texture_pipeline = PipelineBuilder {
        shader_path: "./examples/forward/texture.wgsl",
        label: Some("texture_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<MaterialBindGroup>(),
                storage.get_bind_group_layout::<TransformBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<PointLightBindGroup>(),
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
    let texture_pipeline_id = storage.insert_pipeline(texture_pipeline);

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
        position: (-10.0, 2.0, 0.0).into(),
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

    let light = PointLight::new((-1.0, 9.0, 5.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let light_handle = PointLightHandle::new(&mut storage, light.build(&renderer));
    let light_bind_group = PointLightBindGroup::new(&renderer, &mut storage, &light_handle);

    let box_mesh: Mesh = Cube::new(9.0, 1.0, 5.0).into();
    let box_id = storage.insert_mesh(box_mesh.build(&renderer));

    let box_transform = Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (3.0, 1.0, 3.0).into(),
    };
    let box_transform_handle = TransformHandle::new(&mut storage, box_transform.build(&renderer));
    let box_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &box_transform_handle);

    let box2_mesh: Mesh = Cube::new(1.0, 1.0, 1.0).into();
    let box2_id = storage.insert_mesh(box2_mesh.build(&renderer));

    let box2_transform = Transform {
        translation: (0.0, 1.0, 1.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let box2_transform_handle = TransformHandle::new(&mut storage, box2_transform.build(&renderer));
    let box2_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &box2_transform_handle);

    let grey_material = ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let grey_material_handle =
        ColorMaterialHandle::new(&mut storage, grey_material.build(&renderer));
    let grey_material_bind_group =
        ColorMaterialBindGroup::new(&renderer, &mut storage, &grey_material_handle);

    let green_material = ColorMaterial {
        ambient: [0.4, 0.9, 0.4],
        diffuse: [0.4, 0.9, 0.4],
        specular: [0.1, 0.1, 0.1],
        shininess: 1.0,
    };
    let green_material_handle =
        ColorMaterialHandle::new(&mut storage, green_material.build(&renderer));
    let green_material_bind_group =
        ColorMaterialBindGroup::new(&renderer, &mut storage, &green_material_handle);

    let cube_model = Model::load("./res/cube/cube.obj").unwrap();
    let (cube_model_handler, _cube_model_materials) = cube_model.build(&renderer, &mut storage);

    let mut cube_transform = Transform {
        translation: (2.0, 2.0, 4.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(69.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let cube_transform_handle = TransformHandle::new(&mut storage, cube_transform.build(&renderer));
    let cube_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &cube_transform_handle);

    let mut last_render_time = std::time::Instant::now();
    let mut fps_logger = FpsLogger::new();
    _ = event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    camera_controller.process_mouse(delta.0, delta.1);
                }
                _ => {}
            },
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

                    camera_controller.update_camera(&mut camera, dt);
                    camera_handle.update(&renderer, &storage, &camera);

                    cube_transform.rotation = cube_transform.rotation
                        * cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_y(),
                            cgmath::Deg(-dt.as_secs_f32() * 30.0),
                        );
                    cube_transform_handle.update(&renderer, &storage, &cube_transform);

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

                    let box1 = MeshRenderCommand {
                        pipeline_id: color_pipeline_id,
                        mesh_id: box_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            grey_material_bind_group.0,
                            box_transform_bind_group.0,
                            camera_bind_group.0,
                        ],
                    };
                    let box2 = MeshRenderCommand {
                        pipeline_id: color_pipeline_id,
                        mesh_id: box2_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            green_material_bind_group.0,
                            box2_transform_bind_group.0,
                            camera_bind_group.0,
                        ],
                    };
                    let cube = MeshRenderCommand {
                        pipeline_id: texture_pipeline_id,
                        mesh_id: cube_model_handler[0].mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            cube_model_handler[0].material_bind_group.0,
                            cube_transform_bind_group.0,
                            camera_bind_group.0,
                            light_bind_group.0,
                        ],
                    };

                    {
                        let mut render_pass =
                            phase.render_pass(&mut encoder, &current_frame_storage);
                        for command in [box1, box2, cube] {
                            command.execute(&mut render_pass, &current_frame_storage);
                        }
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
