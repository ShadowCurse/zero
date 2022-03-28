use cgmath::prelude::*;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod deffered_rendering;
mod light;
mod material;
mod model;
mod renderer;
mod shadow_map;
mod shapes;
mod skybox;
mod texture;
mod transform;

use renderer::{GpuAsset, PipelineBuilder, RenderAsset, Vertex};

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(renderer::Renderer::new(&window));
    let mut depth_texture = texture::DepthTexture.build(&renderer);

    let camera_builder = renderer::RenderAssetBuilder::<camera::Camera>::new(&renderer);
    let lights_builder = renderer::RenderAssetBuilder::<light::PointLights>::new(&renderer);
    let shadow_map_d_light_builder =
        renderer::RenderAssetBuilder::<shadow_map::ShadowMapDLight>::new(&renderer);
    let shadow_map_builder = renderer::RenderAssetBuilder::<shadow_map::ShadowMap>::new(&renderer);
    let transform_builder = renderer::RenderAssetBuilder::<transform::Transform>::new(&renderer);
    let skybox_builder = renderer::RenderAssetBuilder::<skybox::Skybox>::new(&renderer);
    let material_builder = renderer::RenderAssetBuilder::<material::Material>::new(&renderer);
    let color_material_builder =
        renderer::RenderAssetBuilder::<material::ColorMaterial>::new(&renderer);
    let g_buffer_builder =
        renderer::RenderAssetBuilder::<deffered_rendering::GBuffer>::new(&renderer);

    let mut camera = camera::Camera::new(
        (-10.0, 2.0, 0.0),
        cgmath::Deg(0.0),
        cgmath::Deg(0.0),
        renderer.config.width,
        renderer.config.height,
        cgmath::Deg(90.0),
        0.1,
        100.0,
    );
    let render_camera = camera_builder.build(&renderer, &camera);
    let mut camera_controller = camera::CameraController::new(5.0, 0.7);

    let light = light::PointLight::new((2.0, 1.0, 0.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let light_2 = light::PointLight::new((-2.0, 0.8, 2.0), (0.7, 0.0, 0.8), 1.0, 0.109, 0.032);
    let light_3 = light::PointLight::new((-5.0, 1.5, 1.0), (0.7, 0.3, 0.3), 1.0, 0.209, 0.032);
    let mut lights = light::PointLights {
        lights: vec![light, light_2, light_3],
    };
    let render_lights = lights_builder.build(&renderer, &lights);

    let shadow_d_light = shadow_map::ShadowMapDLight::new(
        (0.0, 9.0, 0.0),
        (0.0, 0.0, 0.0),
        -50.0,
        50.0,
        -50.0,
        50.0,
        0.1,
        1000.0,
    );
    let render_shadow_d_light = shadow_map_d_light_builder.build(&renderer, &shadow_d_light);

    let mut shadow_map =
        shadow_map_builder.build(&renderer, &shadow_map::ShadowMap::default());

    let cube = model::Model::load("./res/cube/cube.obj").unwrap();
    let render_cube = cube.build(&renderer, &material_builder);

    let box_shape: model::Mesh = shapes::Box::new(9.0, 1.0, 5.0).into();
    let render_box = box_shape.build(&renderer);

    let shpere_shape: model::Mesh = shapes::Icoshphere::new(0.1, 5).into();
    let render_sphere = shpere_shape.build(&renderer);

    let mut cube_transform = transform::Transform {
        translation: (0.0, 5.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_cube_transform = transform_builder.build(&renderer, &cube_transform);

    let box_transform = transform::Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_box_transform = transform_builder.build(&renderer, &box_transform);

    let mut sphere_transform = transform::Transform {
        translation: (2.0, 1.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_sphere_transform = transform_builder.build(&renderer, &sphere_transform);

    let skybox = skybox::Skybox::load([
        "./res/skybox/right.jpg",
        "./res/skybox/left.jpg",
        "./res/skybox/top.jpg",
        "./res/skybox/bottom.jpg",
        "./res/skybox/front.jpg",
        "./res/skybox/back.jpg",
    ])
    .unwrap();
    let render_skybox = skybox_builder.build(&renderer, &skybox);

    let color_material = material::ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let color_render_material = color_material_builder.build(&renderer, &color_material);

    let g_buffer_format = wgpu::TextureFormat::Rgba32Float;
    let g_buffer = deffered_rendering::GBuffer::new(g_buffer_format);
    let mut render_g_buffer = g_buffer_builder.build(&renderer, &g_buffer);

    let g_pipeline = PipelineBuilder::new(
        vec![
            &material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/geometry_pass.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![g_buffer_format; 3])
    .build(&renderer);

    let g_color_pipeline = PipelineBuilder::new(
        vec![
            &color_material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/geometry_color_pass.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![g_buffer_format; 3])
    .build(&renderer);

    let color_pipeline = PipelineBuilder::new(
        vec![
            &color_material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/color.wgsl",
    )
    .write_depth(true)
    .build(&renderer);

    let shadow_map_pipeline = PipelineBuilder::new(
        vec![
            &transform_builder.bind_group_layout,
            &shadow_map_d_light_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/shadow_map.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![])
    .build(&renderer);

    let skybox_pipeline = PipelineBuilder::new(
        vec![
            &skybox_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        vec![skybox::SkyboxVertex::desc()],
        "./shaders/skybox.wgsl",
    )
    .write_depth(false)
    .build(&renderer);

    let lighting_pass_pipeline = PipelineBuilder::new(
        vec![
            &g_buffer_builder.bind_group_layout,
            &lights_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
            &shadow_map_builder.bind_group_layout,
        ],
        vec![texture::TextureVertex::desc()],
        "./shaders/lighting_pass.wgsl",
    )
    .depth_enabled(false)
    .build(&renderer);

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
                    depth_texture = texture::DepthTexture.build(&renderer);
                    render_g_buffer = g_buffer_builder.build(&renderer, &g_buffer);
                    shadow_map =
                        shadow_map_builder.build(&renderer, &shadow_map::ShadowMap::default());
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    depth_texture = texture::DepthTexture.build(&renderer);
                    render_g_buffer = g_buffer_builder.build(&renderer, &g_buffer);
                    shadow_map =
                        shadow_map_builder.build(&renderer, &shadow_map::ShadowMap::default());
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                camera_controller.update_camera(&mut camera, dt);
                camera.update(&renderer, &render_camera);

                lights.lights[0].position =
                    cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                        * lights.lights[0].position;
                lights.update(&renderer, &render_lights);

                sphere_transform.translation =
                    cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                        * sphere_transform.translation;
                sphere_transform.update(&renderer, &render_sphere_transform);

                cube_transform.rotation = cube_transform.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-dt.as_secs_f32() * 30.0),
                    );
                cube_transform.update(&renderer, &render_cube_transform);

                let model_command = model::ModelRenderCommand {
                    pipeline: &g_pipeline,
                    models: vec![&render_cube],
                    transforms: vec![&render_cube_transform],
                    camera: &render_camera,
                };

                let box_command = model::MeshRenderCommand {
                    pipeline: &g_color_pipeline,
                    mesh: &render_box,
                    material: &color_render_material,
                    transform: &render_box_transform,
                    camera: &render_camera,
                };

                let sphere_command = model::MeshRenderCommand {
                    pipeline: &color_pipeline,
                    mesh: &render_sphere,
                    material: &color_render_material,
                    transform: &render_sphere_transform,
                    camera: &render_camera,
                };

                let skybox_command = skybox::SkyboxRenderCommand {
                    pipeline: &skybox_pipeline,
                    skybox: &render_skybox,
                    camera: &render_camera,
                };

                let shadow_map_sphere_pass_command = shadow_map::ShadowMapRenderCommand {
                    pipeline: &shadow_map_pipeline,
                    mesh: &render_sphere,
                    transform: &render_sphere_transform,
                    dlight: &render_shadow_d_light,
                };

                let shadow_map_box_pass_command = shadow_map::ShadowMapRenderCommand {
                    pipeline: &shadow_map_pipeline,
                    mesh: &render_box,
                    transform: &render_box_transform,
                    dlight: &render_shadow_d_light,
                };

                let deffered_pass_command = deffered_rendering::DefferedPassRenderCommand {
                    pipeline: &lighting_pass_pipeline,
                    g_buffer: &render_g_buffer,
                    shadow_map: &shadow_map,
                    lights: &render_lights,
                    camera: &render_camera,
                };

                match renderer.deferred_render(
                    &[&box_command, &model_command],
                    &[&shadow_map_sphere_pass_command, &shadow_map_box_pass_command],
                    &[&deffered_pass_command],
                    Some(&[&sphere_command, &skybox_command]),
                    &render_g_buffer,
                    &depth_texture,
                    &shadow_map.shadow_map,
                ) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(None),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
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
