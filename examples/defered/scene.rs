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
    storage.register_bind_group_layout::<GBufferBindGroup>(&renderer);
    storage.register_bind_group_layout::<PointLightBindGroup>(&renderer);
    storage.register_bind_group_layout::<PointLightsBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowMapBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowMapDLightBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowBindGroup>(&renderer);
    storage.register_bind_group_layout::<SkyboxBindGroup>(&renderer);
    storage.register_bind_group_layout::<TransformBindGroup>(&renderer);

    let g_pipeline = PipelineBuilder {
        shader_path: "./examples/defered/geometry_pass.wgsl",
        label: Some("g_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<MaterialBindGroup>(),
                storage.get_bind_group_layout::<TransformBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[MeshVertex::layout()],
        vertex_entry_point: "vs_main",
        color_targets: Some(&[
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
        ]),
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
    let g_pipeline_id = storage.insert_pipeline(g_pipeline);

    let g_color_pipeline = PipelineBuilder {
        shader_path: "./examples/defered/geometry_color_pass.wgsl",
        label: Some("g_color_pipeline"),
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
        color_targets: Some(&[
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: TextureFormat::Rgba32Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
        ]),
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
    let g_color_pipeline_id = storage.insert_pipeline(g_color_pipeline);

    let shadow_map_pipeline = PipelineBuilder {
        shader_path: "./examples/defered/shadow_map.wgsl",
        label: Some("shadow_map_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<TransformBindGroup>(),
                storage.get_bind_group_layout::<ShadowMapDLightBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[MeshVertex::layout()],
        vertex_entry_point: "vs_main",
        color_targets: None,
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
    let shadow_map_pipeline_id = storage.insert_pipeline(shadow_map_pipeline);

    let lighting_pipeline = PipelineBuilder {
        shader_path: "./examples/defered/lighting_pass.wgsl",
        label: Some("lighting_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<GBufferBindGroup>(),
                storage.get_bind_group_layout::<PointLightsBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
                storage.get_bind_group_layout::<ShadowBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[TextureVertex::layout()],
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
    let lighting_pipeline_id = storage.insert_pipeline(lighting_pipeline);

    let skybox_pipeline = PipelineBuilder {
        shader_path: "./examples/defered/skybox.wgsl",
        label: Some("skybox_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<SkyboxBindGroup>(),
                storage.get_bind_group_layout::<CameraBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[SkyboxVertex::layout()],
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
            depth_write_enabled: false,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        multiview: None,
    }
    .build(&renderer);
    let skybox_pipeline_id = storage.insert_pipeline(skybox_pipeline);

    let depth_texture_id = storage.insert_texture(EmptyTexture::new_depth().build(&renderer));
    let shadow_map_handle =
        ShadowMapHandle::new(&mut storage, ShadowMap::default().build(&renderer));

    let g_buffer = GBuffer::new(TextureFormat::Rgba32Float);
    let g_buffer_handle = GBufferHandle::new(&mut storage, g_buffer.build(&renderer));
    let g_buffer_bind_group = GBufferBindGroup::new(&renderer, &mut storage, &g_buffer_handle);

    let geometry_phase = RenderPhase::new(
        const_vec![
            ColorAttachment {
                view_id: g_buffer_handle.position_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.normal_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.albedo_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            },
        ],
        Some(DepthStencil {
            view_id: depth_texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
    );

    let shadow_phase = RenderPhase::new(
        const_vec![],
        Some(DepthStencil {
            view_id: shadow_map_handle.texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
    );

    let lighting_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        }],
        None,
    );

    let skybox_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: StoreOp::Store,
            },
        }],
        Some(DepthStencil {
            view_id: depth_texture_id,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
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
    let light_2 = PointLight::new((-2.0, 1.0, -2.0), (0.8, 0.1, 0.1), 1.0, 0.109, 0.032);
    let light_3 = PointLight::new((-2.0, 1.0, 0.0), (0.1, 0.8, 0.1), 1.0, 0.209, 0.032);
    let light_4 = PointLight::new((-2.0, 1.0, 2.0), (0.1, 0.1, 0.8), 1.0, 0.209, 0.032);
    let lights = PointLights {
        lights: vec![light, light_2, light_3, light_4],
    };
    let lights_handle = PointLightsHandle::new(&mut storage, lights.build(&renderer));
    let lights_bind_group = PointLightsBindGroup::new(&renderer, &mut storage, &lights_handle);

    let shadow_d_light = ShadowMapDLight::new(
        (-2.0, 9.0, 8.0),
        (1.0, -3.0, -3.0),
        -10.0,
        10.0,
        -10.0,
        10.0,
        0.1,
        8.0,
    );
    let shadow_d_light_handle =
        ShadowMapDLightHandle::new(&mut storage, shadow_d_light.build(&renderer));
    let shadow_d_light_bind_group =
        ShadowMapDLightBindGroup::new(&renderer, &mut storage, &shadow_d_light_handle);

    let shadow_bind_group = ShadowBindGroup::new(
        &renderer,
        &mut storage,
        &(shadow_map_handle, shadow_d_light_handle),
    );

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

    // order is incorrect
    // should be
    // - right
    // - left
    // - botton
    // - back
    // - front
    let skybox = Skybox::load([
        "./res/skybox/right.jpg",
        "./res/skybox/left.jpg",
        "./res/skybox/top.jpg",
        "./res/skybox/bottom.jpg",
        "./res/skybox/front.jpg",
        "./res/skybox/back.jpg",
    ])
    .unwrap();
    let skybox_handle = SkyboxHandle::new(&mut storage, skybox.build(&renderer));
    let skybox_bind_group = SkyboxBindGroup::new(&renderer, &mut storage, &skybox_handle);

    let mut last_render_time = std::time::Instant::now();
    let mut fps_logger = FpsLogger::new();
    _ = event_loop.run(|event, target| {
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
                    g_buffer_handle.replace(&mut storage, g_buffer.build(&renderer));
                    g_buffer_bind_group.replace(&renderer, &mut storage, &g_buffer_handle);
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
                        pipeline_id: g_color_pipeline_id,
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
                        pipeline_id: g_color_pipeline_id,
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
                        pipeline_id: g_pipeline_id,
                        mesh_id: cube_model_handler[0].mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            cube_model_handler[0].material_bind_group.0,
                            cube_transform_bind_group.0,
                            camera_bind_group.0,
                        ],
                    };

                    {
                        let mut render_pass =
                            geometry_phase.render_pass(&mut encoder, &current_frame_storage);
                        for command in [box1, box2, cube] {
                            command.execute(&mut render_pass, &current_frame_storage);
                        }
                    }

                    let box1 = MeshRenderCommand {
                        pipeline_id: shadow_map_pipeline_id,
                        mesh_id: box_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            box_transform_bind_group.0,
                            shadow_d_light_bind_group.0
                        ],
                    };
                    let box2 = MeshRenderCommand {
                        pipeline_id: shadow_map_pipeline_id,
                        mesh_id: box2_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            box2_transform_bind_group.0,
                            shadow_d_light_bind_group.0
                        ],
                    };
                    let cube = MeshRenderCommand {
                        pipeline_id: shadow_map_pipeline_id,
                        mesh_id: cube_model_handler[0].mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            cube_transform_bind_group.0,
                            shadow_d_light_bind_group.0
                        ],
                    };
                    {
                        let mut render_pass =
                            shadow_phase.render_pass(&mut encoder, &current_frame_storage);
                        for command in [box1, box2, cube] {
                            command.execute(&mut render_pass, &current_frame_storage);
                        }
                    }

                    let command = MeshRenderCommand {
                        pipeline_id: lighting_pipeline_id,
                        mesh_id: g_buffer_handle.mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![
                            g_buffer_bind_group.0,
                            lights_bind_group.0,
                            camera_bind_group.0,
                            shadow_bind_group.0,
                        ],
                    };
                    {
                        let mut render_pass =
                            lighting_phase.render_pass(&mut encoder, &current_frame_storage);
                        command.execute(&mut render_pass, &current_frame_storage);
                    }

                    let command = MeshRenderCommand {
                        pipeline_id: skybox_pipeline_id,
                        mesh_id: skybox_handle.mesh_id,
                        index_slice: None,
                        vertex_slice: None,
                        scissor_rect: None,
                        bind_groups: const_vec![skybox_bind_group.0, camera_bind_group.0],
                    };
                    {
                        let mut render_pass =
                            skybox_phase.render_pass(&mut encoder, &current_frame_storage);
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
