use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use cgmath::prelude::*;
use wgpu::util::DeviceExt;

mod camera;
mod model;
mod renderer;
mod texture;

use model::Vertex;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(renderer::Renderer::new(&window));

    let model = model::Model::load(&renderer, "./res/cube.obj").unwrap();

    let mut transform = model::Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };

    let light_transform = model::Transform {
        translation: (5.0, 5.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (0.5, 0.5, 0.5).into(),
    };
    let instance_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("instance_buffer"),
        contents: bytemuck::cast_slice(&vec![transform.to_raw(), light_transform.to_raw()]),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let mut camera = camera::Camera::new(
        (0.0, 5.0, 10.0),
        cgmath::Deg(-90.0),
        cgmath::Deg(-20.0),
        renderer.config.width,
        renderer.config.height,
        cgmath::Deg(45.0),
        0.1,
        100.0,
    );
    let mut camera_controller = camera::CameraController::new(5.0, 0.4);
    let mut render_camera = camera::RenderCamera::new(&renderer, &camera);

    let mut depth_texture = texture::Texture::create_depth_texture(&renderer, "depth_texture");

    renderer.create_render_pipeline(
        &[&model.bind_group_layout, &render_camera.bind_group_layout],
        &[model::ModelVertex::desc(), model::TransformRaw::desc()],
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

                camera_controller.update_camera(&mut camera, dt);
                render_camera.update(&renderer, &camera);

                transform.rotation = transform.rotation * cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(dt.as_secs_f32() * 5.0));
                renderer
                    .queue
                    .write_buffer(&instance_buffer, 0, bytemuck::cast_slice(&[transform.to_raw()]));

                match renderer.render(&model, &instance_buffer, &render_camera, &depth_texture) {
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
