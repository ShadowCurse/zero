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
mod renderer;
mod skybox;
mod texture;
mod transform;
mod shapes;

use renderer::{Vertex, GpuAsset};

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(renderer::Renderer::new(&window));
    let mut depth_texture = texture::DepthTexture::build(&renderer);

    let camera_builder = renderer::RenderAssetBuilder::<camera::Camera>::new(&renderer);
    let light_builder = renderer::RenderAssetBuilder::<light::PointLight>::new(&renderer);
    let transform_builder = renderer::RenderAssetBuilder::<transform::Transform>::new(&renderer);
    let material_builder = renderer::RenderAssetBuilder::<material::Material>::new(&renderer);
    let color_material_builder =
        renderer::RenderAssetBuilder::<material::ColorMaterial>::new(&renderer);
    let skybox_builder = renderer::RenderAssetBuilder::<skybox::Skybox>::new(&renderer);

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
    let mut render_camera = camera_builder.build(&renderer, &camera);
    let mut camera_controller = camera::CameraController::new(5.0, 0.7);

    let mut light = light::PointLight::new((5.0, 0.0, 5.0), (1.0, 1.0, 1.0), 1.0, 0.09, 0.032);
    let mut render_light = light_builder.build(&renderer, &light);

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

    let cube = model::Model::load("./res/cube/cube.obj").unwrap();
    let render_cube = cube.build(&renderer, &material_builder);

    let plane: model::Mesh = shapes::Box::new(2.0, 5.0, 1.0).into();
    let gpu_plane = plane.build(&renderer);

    let mut transform_1 = transform::Transform {
        translation: (5.0, -5.0, 5.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let mut render_transform_1 = transform_builder.build(&renderer, &transform_1);
    let mut transform_2 = transform::Transform {
        translation: (5.0, 5.0, 5.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let mut render_transform_2 = transform_builder.build(&renderer, &transform_2);

    let color_material = material::ColorMaterial {
        ambient: [0.5, 0.0, 0.8],
        diffuse: [0.5, 0.0, 0.8],
        specular: [0.5, 0.0, 0.8],
        shininess: 0.0,
    };
    let color_render_material = color_material_builder.build(&renderer, &color_material);

    let skybox_pipeline = renderer.create_render_pipeline(
        &[
            &skybox_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
        ],
        &[skybox::SkyboxVertex::desc()],
        "./shaders/skybox.wgsl",
        false,
    );

    let model_pipeline = renderer.create_render_pipeline(
        &[
            &material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
            &light_builder.bind_group_layout,
        ],
        &[model::ModelVertex::desc()],
        "./shaders/shader.wgsl",
        true,
    );

    let color_pipeline = renderer.create_render_pipeline(
        &[
            &color_material_builder.bind_group_layout,
            &transform_builder.bind_group_layout,
            &camera_builder.bind_group_layout,
            &light_builder.bind_group_layout,
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
                    depth_texture = texture::DepthTexture::build(&renderer);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    depth_texture = texture::DepthTexture::build(&renderer);
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
                render_light.update(&renderer, &light);

                camera_controller.update_camera(&mut camera, dt);
                render_camera.update(&renderer, &camera);

                transform_2.rotation = transform_2.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(dt.as_secs_f32() * 60.0),
                    );
                render_transform_2.update(&renderer, &transform_2);

                transform_1.rotation = transform_1.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-dt.as_secs_f32() * 120.0),
                    );
                render_transform_1.update(&renderer, &transform_1);

                let model_command = model::ModelRenderCommand {
                    pipeline: &model_pipeline,
                    models: vec![&render_cube],
                    transforms: vec![&render_transform_1],
                    camera: &render_camera,
                    light: &render_light,
                };

                let color_command = model::MeshRenderCommand {
                    pipeline: &color_pipeline,
                    mesh: &gpu_plane,
                    material: &color_render_material,
                    transform: &render_transform_2,
                    camera: &render_camera,
                    light: &render_light,
                };

                let skybox_command = skybox::SkyboxRenderCommand {
                    pipeline: &skybox_pipeline,
                    skybox: &render_skybox,
                    camera: &render_camera,
                };

                match renderer.render(
                    &vec![&model_command, &color_command, &skybox_command],
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
