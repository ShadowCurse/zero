use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use zero::prelude::*;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut render_system = RenderSystem::default();
    let mut storage = RenderStorage::default();

    let depth_texture_id = storage.build_texture(&renderer, &DepthTexture::default());
    let shadow_map_id = storage.build_asset(&renderer, &ShadowMap::default());

    let g_buffer = GBuffer::new(TextureFormat::Rgba32Float);
    let g_buffer_id = storage.build_asset(&renderer, &g_buffer);
    let geometry_phase = RenderPhase::new(
        vec![ColorAttachment {
            view_id: g_buffer_id,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: true,
            },
        }],
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
        vec![],
        Some(DepthStencil {
            view_id: shadow_map_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    );
    render_system.add_phase("shadow", shadow_phase);

    let lighting_phase = RenderPhase::new(
        vec![ColorAttachment {
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
        vec![ColorAttachment {
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
        renderer.config.width,
        renderer.config.height,
        Deg(90.0),
        0.1,
        100.0,
    );
    let camera_id = storage.build_asset(&renderer, &camera);

    let mut camera_controller = CameraController::new(5.0, 0.7);

    let light = PointLight::new((-1.0, 9.0, 5.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let light_2 = PointLight::new((-2.0, 0.8, 2.0), (0.7, 0.0, 0.8), 1.0, 0.109, 0.032);
    let light_3 = PointLight::new((-5.0, 1.5, 1.0), (0.7, 0.3, 0.3), 1.0, 0.209, 0.032);
    let lights = PointLights {
        lights: vec![light, light_2, light_3],
    };
    let lights_id = storage.build_asset(&renderer, &lights);

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
    let shadow_d_light_id = storage.build_asset(&renderer, &shadow_d_light);

    let box_mesh: Mesh = Cube::new(9.0, 1.0, 5.0).into();
    let box_id = storage.build_mesh(&renderer, &box_mesh);

    let box_transform = Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (5.0, 1.0, 5.0).into(),
    };
    let box_transform_id = storage.build_asset(&renderer, &box_transform);

    let box2_mesh: Mesh = Cube::new(1.0, 1.0, 1.0).into();
    let box2_id = storage.build_mesh(&renderer, &box2_mesh);

    let box2_transform = Transform {
        translation: (0.0, 1.0, 1.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let box2_transform_id = storage.build_asset(&renderer, &box2_transform);

    let grey_material = ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let grey_material_id = storage.build_asset(&renderer, &grey_material);

    let green_material = ColorMaterial {
        ambient: [0.4, 0.9, 0.4],
        diffuse: [0.4, 0.9, 0.4],
        specular: [0.1, 0.1, 0.1],
        shininess: 1.0,
    };
    let green_material_id = storage.build_asset(&renderer, &green_material);

    let g_color_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<ColorMaterial>(),
            storage.get_bind_group_layout::<Transform>(),
            storage.get_bind_group_layout::<Camera>(),
        ],
        vertex_layouts: vec![MeshVertex::desc()],
        shader_path: "./shaders/geometry_color_pass.wgsl",
        write_depth: true,
        color_targets: Some(vec![TextureFormat::Rgba32Float; 3]),
        ..Default::default()
    }
    .build(&renderer);
    let g_color_pipeline_id = storage.add_pipeline(g_color_pipeline);

    let shadow_map_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<Transform>(),
            storage.get_bind_group_layout::<ShadowMapDLight>(),
        ],
        vertex_layouts: vec![MeshVertex::desc()],
        shader_path: "./shaders/shadow_map.wgsl",
        write_depth: true,
        color_targets: Some(vec![]),
        cull_mode: Face::Front,
        ..Default::default()
    }
    .build(&renderer);
    let shadow_map_pipeline_id = storage.add_pipeline(shadow_map_pipeline);

    let lighting_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<GBuffer>(),
            storage.get_bind_group_layout::<PointLights>(),
            storage.get_bind_group_layout::<Camera>(),
            storage.get_bind_group_layout::<ShadowMap>(),
            storage.get_bind_group_layout::<ShadowMapDLight>(),
        ],
        vertex_layouts: vec![TextureVertex::desc()],
        shader_path: "./shaders/lighting_pass.wgsl",
        depth_enabled: false,
        ..Default::default()
    }
    .build(&renderer);
    let lighting_pipeline_id = storage.add_pipeline(lighting_pipeline);

    let skybox = Skybox::load([
        "./res/skybox/right.jpg",
        "./res/skybox/left.jpg",
        "./res/skybox/top.jpg",
        "./res/skybox/bottom.jpg",
        "./res/skybox/front.jpg",
        "./res/skybox/back.jpg",
    ])
    .unwrap();
    let skybox_id = storage.build_asset(&renderer, &skybox);

    let skybox_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<Skybox>(),
            storage.get_bind_group_layout::<Camera>(),
        ],
        vertex_layouts: vec![SkyboxVertex::desc()],
        shader_path: "./shaders/skybox.wgsl",
        write_depth: false,
        ..Default::default()
    }
    .build(&renderer);
    let skybox_pipeline_id = storage.add_pipeline(skybox_pipeline);

    let mut last_render_time = std::time::Instant::now();
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
                    storage.rebuild_texture(&renderer, &DepthTexture::default(), depth_texture_id);
                    storage.rebuild_asset(&renderer, &g_buffer, g_buffer_id);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    storage.rebuild_texture(&renderer, &DepthTexture::default(), depth_texture_id);
                    storage.rebuild_asset(&renderer, &g_buffer, g_buffer_id);
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                camera_controller.update_camera(&mut camera, dt);
                storage.rebuild_asset(&renderer, &camera, camera_id);

                let box1 = RenderCommand::new(
                    g_color_pipeline_id,
                    box_id,
                    vec![grey_material_id, box_transform_id, camera_id],
                );
                let box2 = RenderCommand::new(
                    g_color_pipeline_id,
                    box2_id,
                    vec![green_material_id, box2_transform_id, camera_id],
                );
                render_system.add_phase_commands("geometry", vec![box1, box2]);

                let box1 = RenderCommand::new(
                    shadow_map_pipeline_id,
                    box_id,
                    vec![box_transform_id, shadow_d_light_id],
                );
                let box2 = RenderCommand::new(
                    shadow_map_pipeline_id,
                    box2_id,
                    vec![box2_transform_id, shadow_d_light_id],
                );
                render_system.add_phase_commands("shadow", vec![box1, box2]);

                let command = RenderCommand::new(
                    lighting_pipeline_id,
                    g_buffer_id,
                    vec![
                        g_buffer_id,
                        lights_id,
                        camera_id,
                        shadow_map_id,
                        shadow_d_light_id,
                    ],
                );
                render_system.add_phase_commands("lighting", vec![command]);

                let command =
                    RenderCommand::new(skybox_pipeline_id, skybox_id, vec![skybox_id, camera_id]);
                render_system.add_phase_commands("skybox", vec![command]);

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
