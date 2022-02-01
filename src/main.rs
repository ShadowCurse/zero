use cgmath::prelude::*;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod light;
mod model;
mod renderer;
mod skybox;
mod texture;

use model::Vertex;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(renderer::Renderer::new(&window));

    let mut camera = camera::Camera::new(
        (0.0, 0.0, 0.0),
        cgmath::Deg(0.0),
        cgmath::Deg(0.0),
        renderer.config.width,
        renderer.config.height,
        cgmath::Deg(90.0),
        0.1,
        100.0,
    );
    let mut camera_controller = camera::CameraController::new(5.0, 0.7);
    let mut render_camera = camera::RenderCamera::new(&renderer, &camera);

    let skybox = skybox::Skybox::load(
        &renderer,
        [
            "./res/skybox/right.jpg",
            "./res/skybox/left.jpg",
            "./res/skybox/top.jpg",
            "./res/skybox/bottom.jpg",
            "./res/skybox/front.jpg",
            "./res/skybox/back.jpg",
        ],
    )
    .unwrap();

    let skybox_pipeline = renderer.create_render_pipeline(
        &[&skybox.bind_group_layout, &render_camera.bind_group_layout],
        &[skybox::SkyboxVertex::desc()],
        "./shaders/skybox.wgsl",
        false,
    );

    let cube = model::Model::load(&renderer, "./res/cube.obj").unwrap();
    let cube_transform = model::Transform {
        translation: (0.0, 5.0, 5.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let cube_render_transform = model::RenderTransform::new(&renderer, &cube_transform);

    let rifle = model::Model::load(&renderer, "./res/sniper_rifle.obj").unwrap();
    let mut rifle_transform = model::Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let mut rifle_render_transform = model::RenderTransform::new(&renderer, &rifle_transform);

    let light = light::Light::new((5.0, 5.0, 5.0), (1.0, 1.0, 1.0));
    let render_light = light::RenderLight::new(&renderer, &light);
    let mut depth_texture = texture::Texture::create_depth_texture(&renderer, "depth_texture");
    let model_pipeline = renderer.create_render_pipeline(
        &[
            &cube.bind_group_layout,
            &cube_render_transform.bind_group_layout,
            &render_camera.bind_group_layout,
            &render_light.bind_group_layout,
        ],
        &[model::ModelVertex::desc()],
        "./shaders/shader.wgsl",
        true,
    );

    let color_material = model::ColorMaterial::new(&renderer, [0.5, 0.5, 0.5], [0.5, 0.0, 0.8], [0.0, 0.5, 0.0], 32.0);
    let color_pipeline = renderer.create_render_pipeline(
        &[
            &color_material.bind_group_layout,
            &cube_render_transform.bind_group_layout,
            &render_camera.bind_group_layout,
            &render_light.bind_group_layout,
        ],
        &[model::ModelVertex::desc()],
        "./shaders/color.wgsl",
        true,
    );

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
                    depth_texture =
                        texture::Texture::create_depth_texture(&renderer, "depth_texture");
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    depth_texture =
                        texture::Texture::create_depth_texture(&renderer, "depth_texture");
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                // println!("frame time: {}ms", dt.as_millis());

                camera_controller.update_camera(&mut camera, dt);
                render_camera.update(&renderer, &camera);

                rifle_transform.rotation = rifle_transform.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(dt.as_secs_f32() * 60.0),
                    );
                rifle_render_transform.update(&renderer, &rifle_transform);

                let skybox_command = skybox::SkyboxRenderCommand {
                    pipeline: &skybox_pipeline,
                    skybox: &skybox,
                    camera: &render_camera,
                };

                let model_command = model::ModelRenderCommand {
                    pipeline: &model_pipeline,
                    models: vec![&cube],
                    transforms: vec![&cube_render_transform],
                    camera: &render_camera,
                    light: &render_light,
                };

                let color_command = model::MeshRenderCommand {
                    pipeline: &color_pipeline,
                    mesh: &cube.meshes[0],
                    material: &color_material,
                    transform: &rifle_render_transform,
                    camera: &render_camera,
                    light: &render_light,
                };

                match renderer.render(&vec![&skybox_command, &model_command, &color_command], &depth_texture) {
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
