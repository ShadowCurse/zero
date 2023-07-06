use wgpu::{BlendFactor, BlendOperation};
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use zero::{
    const_vec,
    egui::{EguiBufferBindGroup, EguiTextureBindGroup, EguiVertex, ZeroEguiContext},
    prelude::*,
};

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

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut render_system = RenderSystem::default();
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
        shader_path: "./shaders/geometry_pass.wgsl",
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
    let g_pipeline_id = storage.insert_pipeline(g_pipeline);

    let g_color_pipeline = PipelineBuilder {
        shader_path: "./shaders/geometry_color_pass.wgsl",
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
    let g_color_pipeline_id = storage.insert_pipeline(g_color_pipeline);

    let shadow_map_pipeline = PipelineBuilder {
        shader_path: "./shaders/shadow_map.wgsl",
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
    let shadow_map_pipeline_id = storage.insert_pipeline(shadow_map_pipeline);

    let lighting_pipeline = PipelineBuilder {
        shader_path: "./shaders/lighting_pass.wgsl",
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
        shader_path: "./shaders/skybox.wgsl",
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
            format: DepthTexture::DEPTH_FORMAT,
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

    let depth_texture_id = storage.insert_texture(DepthTexture::default().build(&renderer));
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
                    store: true,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.normal_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: true,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.albedo_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: true,
                },
            },
        ],
        Some(DepthStencil {
            view_id: depth_texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    );
    render_system.add_phase("geometry", geometry_phase);

    let shadow_phase = RenderPhase::new(
        const_vec![],
        Some(DepthStencil {
            view_id: shadow_map_handle.texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    );
    render_system.add_phase("shadow", shadow_phase);

    let lighting_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Color::BLACK),
                store: true,
            },
        }],
        None,
    );
    render_system.add_phase("lighting", lighting_phase);

    let skybox_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        }],
        Some(DepthStencil {
            view_id: depth_texture_id,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            }),
            stencil_ops: None,
        }),
    );

    render_system.add_phase("skybox", skybox_phase);

    let mut camera = Camera::new(
        (-10.0, 2.0, 0.0),
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

    // EGUI
    storage.register_bind_group_layout::<EguiBufferBindGroup>(&renderer);
    storage.register_bind_group_layout::<EguiTextureBindGroup>(&renderer);

    let egui_pipeline = PipelineBuilder {
        shader_path: "./shaders/egui.wgsl",
        label: Some("egui_pipeline"),
        layout_descriptor: Some(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                storage.get_bind_group_layout::<EguiBufferBindGroup>(),
                storage.get_bind_group_layout::<EguiTextureBindGroup>(),
            ],
            push_constant_ranges: &[],
        }),
        vertex_layouts: &[EguiVertex::layout()],
        vertex_entry_point: "vs_main",
        color_targets: Some(&[Some(ColorTargetState {
            format: renderer.surface_format(),
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrites::ALL,
        })]),
        fragment_entry_point: if renderer.surface_format().is_srgb() {
            "fs_main_linear_framebuffer"
        } else {
            "fs_main_gamma_framebuffer"
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Cw,
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
    let egui_pipeline_id = storage.insert_pipeline(egui_pipeline);

    let egui_phase = RenderPhase::new(
        const_vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        }],
        None,
    );

    render_system.add_phase("egui", egui_phase);

    let mut zero_egui_context = ZeroEguiContext::new(&renderer, &mut storage);
    let egui_ctx = egui::Context::default();
    let mut name = String::new();
    let mut age = 0;

    let mut last_render_time = std::time::Instant::now();
    let mut fps_logger = FpsLogger::new();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(key_code),
                    state,
                    ..
                }) => {
                    camera_controller.process_key(*key_code, *state);
                }
                DeviceEvent::Button { button: 1, state } => {
                    camera_controller.set_mouse_active(*state == ElementState::Pressed);
                }
                DeviceEvent::MouseMotion { delta } => {
                    camera_controller.process_mouse(delta.0, delta.1);
                }
                _ => {}
            },
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    camera.resize(physical_size.width, physical_size.height);
                    renderer.resize(Some(*physical_size));
                    storage.replace_texture(
                        depth_texture_id,
                        DepthTexture::default().build(&renderer),
                    );
                    g_buffer_handle.replace(&mut storage, g_buffer.build(&renderer));
                    g_buffer_bind_group.replace(&renderer, &mut storage, &g_buffer_handle);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    storage.replace_texture(
                        depth_texture_id,
                        DepthTexture::default().build(&renderer),
                    );
                    g_buffer_handle.replace(&mut storage, g_buffer.build(&renderer));
                    g_buffer_bind_group.replace(&renderer, &mut storage, &g_buffer_handle);
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
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

                let box1 = RenderCommand {
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
                let box2 = RenderCommand {
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
                let cube = RenderCommand {
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
                render_system.add_phase_commands("geometry", vec![box1, box2, cube]);

                let box1 = RenderCommand {
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
                let box2 = RenderCommand {
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
                let cube = RenderCommand {
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
                render_system.add_phase_commands("shadow", vec![box1, box2, cube]);

                let command = RenderCommand {
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
                render_system.add_phase_commands("lighting", vec![command]);

                let command = RenderCommand {
                    pipeline_id: skybox_pipeline_id,
                    mesh_id: skybox_handle.mesh_id,
                    index_slice: None,
                    vertex_slice: None,
                    scissor_rect: None,
                    bind_groups: const_vec![skybox_bind_group.0, camera_bind_group.0],
                };
                render_system.add_phase_commands("skybox", vec![command]);

                // EGUI
                let egui_input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(renderer.size().width as f32, renderer.size().height as f32),
                    )),
                    ..Default::default()
                };
                let egui_out = egui_ctx.run(egui_input, |ctx| {
                    egui::Window::new("Window").show(ctx, |ui| {
                        ui.heading("My egui Application");
                        ui.horizontal(|ui| {
                            ui.label("Your name: ");
                            ui.text_edit_singleline(&mut name);
                        });
                        ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
                        if ui.button("Click each year").clicked() {
                            age += 1;
                        }
                        ui.label(format!("Hello '{name}', age {age}"));
                    });
                });
                // handle_non_render_out(egui_out.platform_output)
                zero_egui_context.update_textures(&renderer, &mut storage, egui_out.textures_delta);

                let clipped = egui_ctx.tessellate(egui_out.shapes);
                zero_egui_context.update_meshes(&renderer, &mut storage, &clipped);
                let commands = zero_egui_context.create_commands(egui_pipeline_id, &clipped);

                render_system.add_phase_commands("egui", commands);

                match render_system.run(&renderer, &storage) {
                    Ok(_) => {}
                    Err(SurfaceError::Lost) => renderer.resize(None),
                    Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
