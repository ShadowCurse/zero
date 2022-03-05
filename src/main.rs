use cgmath::prelude::*;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod light;
mod material;
mod model;
mod present_texture;
mod renderer;
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
    let light_builder = renderer::RenderAssetBuilder::<light::PointLight>::new(&renderer);
    let transform_builder = renderer::RenderAssetBuilder::<transform::Transform>::new(&renderer);
    let material_builder = renderer::RenderAssetBuilder::<material::Material>::new(&renderer);
    let color_material_builder =
        renderer::RenderAssetBuilder::<material::ColorMaterial>::new(&renderer);

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

    let mut light = light::PointLight::new((2.0, 1.0, 0.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let render_light = light_builder.build(&renderer, &light);

    let light_2 = light::PointLight::new((-2.0, 0.2, 2.0), (0.7, 0.0, 0.8), 1.0, 0.109, 0.032);
    // let render_light_2 = light_builder.build(&renderer, &light_2);

    let cube = model::Model::load("./res/cube/cube.obj").unwrap();
    let render_cube = cube.build(&renderer, &material_builder);

    // let plane: model::Mesh = shapes::Plane::new(10.0).into();
    let plane: model::Mesh = shapes::Box::new(9.0, 1.0, 5.0).into();
    let gpu_plane = plane.build(&renderer);

    let mut transform_1 = transform::Transform {
        translation: (0.0, 5.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_transform_1 = transform_builder.build(&renderer, &transform_1);

    let transform_2 = transform::Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_transform_2 = transform_builder.build(&renderer, &transform_2);

    let color_material = material::ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let color_render_material = color_material_builder.build(&renderer, &color_material);

    let g_buffer_format = wgpu::TextureFormat::Rgba32Float;

    let lights_builder = renderer::RenderAssetBuilder::<light::PointLights>::new(&renderer);
    let lights = light::PointLights {
        lights: vec![light.clone(), light_2],
    };
    let render_lights = lights_builder.build(&renderer, &lights);

    let deffered_pass_builder = renderer::RenderAssetBuilder::<
        present_texture::DefferedPassTextures<texture::GBuffer>,
    >::new(&renderer);
    let deffered_pass_textures = present_texture::DefferedPassTextures {
        position_texture: texture::GBuffer {
            format: g_buffer_format,
        },
        normal_texture: texture::GBuffer {
            format: g_buffer_format,
        },
        albedo_texture: texture::GBuffer {
            format: g_buffer_format,
        },
    };
    let mut render_deffered_pass_textures =
        deffered_pass_builder.build(&renderer, &deffered_pass_textures);

    let g_pipeline = PipelineBuilder::new(
        vec![
            &material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
            &light_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/geometry_pass.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![g_buffer_format, g_buffer_format, g_buffer_format])
    .build(&renderer);

    let g_color_pipeline = PipelineBuilder::new(
        vec![
            &color_material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
            &light_builder.bind_group_layout,
        ],
        vec![model::ModelVertex::desc()],
        "./shaders/geometry_color_pass.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![g_buffer_format, g_buffer_format, g_buffer_format])
    .build(&renderer);

    let lighting_pass_pipeline = PipelineBuilder::new(
        vec![
            &deffered_pass_builder.bind_group_layout,
            &lights_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        vec![present_texture::Vertex::desc()],
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
                    render_deffered_pass_textures =
                        deffered_pass_builder.build(&renderer, &deffered_pass_textures);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    depth_texture = texture::DepthTexture.build(&renderer);
                    render_deffered_pass_textures =
                        deffered_pass_builder.build(&renderer, &deffered_pass_textures);
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                // println!("frame time: {}ms", dt.as_millis());

                light.position =
                    cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                        * light.position;
                // light.update(&renderer, &render_light);

                camera_controller.update_camera(&mut camera, dt);
                camera.update(&renderer, &render_camera);

                transform_1.rotation = transform_1.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-dt.as_secs_f32() * 120.0),
                    );
                transform_1.update(&renderer, &render_transform_1);

                let model_command = model::ModelRenderCommand {
                    pipeline: &g_pipeline,
                    models: vec![&render_cube],
                    transforms: vec![&render_transform_1],
                    camera: &render_camera,
                    light: &render_light,
                };

                let color_command = model::MeshRenderCommand {
                    pipeline: &g_color_pipeline,
                    mesh: &gpu_plane,
                    material: &color_render_material,
                    transform: &render_transform_2,
                    camera: &render_camera,
                    light: &render_light,
                };

                let deffered_pass_command = present_texture::DefferedPassRenderCommand {
                    pipeline: &lighting_pass_pipeline,
                    deffered_pass: &render_deffered_pass_textures,
                    lights: &render_lights,
                    camera: &render_camera,
                };

                match renderer.render_deferred(
                    &[&model_command, &color_command],
                    &[&deffered_pass_command],
                    &[
                        &render_deffered_pass_textures.position_texture,
                        &render_deffered_pass_textures.normal_texture,
                        &render_deffered_pass_textures.albedo_texture,
                    ],
                    &depth_texture,
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
