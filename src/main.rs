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
    // let skybox_builder = renderer::RenderAssetBuilder::<skybox::Skybox>::new(&renderer);

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

    // let skybox = skybox::Skybox::load([
    //     "./res/skybox/right.jpg",
    //     "./res/skybox/left.jpg",
    //     "./res/skybox/top.jpg",
    //     "./res/skybox/bottom.jpg",
    //     "./res/skybox/front.jpg",
    //     "./res/skybox/back.jpg",
    // ])
    // .unwrap();
    // let render_skybox = skybox_builder.build(&renderer, &skybox);

    let cube = model::Model::load("./res/cube/cube.obj").unwrap();
    let render_cube = cube.build(&renderer, &material_builder);

    let plane: model::Mesh = shapes::Plane::new(10.0).into();
    let gpu_plane = plane.build(&renderer);

    let mut transform_1 = transform::Transform {
        translation: (0.0, 5.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let render_transform_1 = transform_builder.build(&renderer, &transform_1);

    // let mut transform_1_scaled = transform_1.clone();
    // transform_1_scaled.scale = (1.1, 1.1, 1.1).into();
    // let render_transform_1_scaled = transform_builder.build(&renderer, &transform_1_scaled);

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

    // let skybox_pipeline = PipelineBuilder::new(
    //     vec![
    //         &skybox_builder.bind_group_layout,
    //         &camera_builder.bind_group_layout,
    //     ],
    //     vec![skybox::SkyboxVertex::desc()],
    //     "./shaders/skybox.wgsl",
    // )
    // .write_depth(false)
    // .build(&renderer);

    // let model_pipeline = PipelineBuilder::new(
    //     vec![
    //         &material_builder.bind_group_layout,
    //         &transform_builder.bind_group_layout,
    //         &camera_builder.bind_group_layout,
    //         &light_builder.bind_group_layout,
    //     ],
    //     vec![model::ModelVertex::desc()],
    //     "./shaders/shader.wgsl",
    // )
    // .stencil_write_mask(0xff)
    // .write_depth(true)
    // .build(&renderer);

    // let color_pipeline = PipelineBuilder::new(
    //     vec![
    //         &color_material_builder.bind_group_layout,
    //         &transform_builder.bind_group_layout,
    //         &camera_builder.bind_group_layout,
    //         &light_builder.bind_group_layout,
    //     ],
    //     vec![model::ModelVertex::desc()],
    //     "./shaders/color.wgsl",
    // )
    // .write_depth(true)
    // .build(&renderer);

    // let present_texture_builder = renderer::RenderAssetBuilder::<
    //     present_texture::PresentTexture<texture::DepthTexture>,
    // >::new(&renderer);
    // let present_depth_texture = present_texture::PresentTexture {
    //     texture: texture::DepthTexture,
    // };
    // let mut render_pdt = present_texture_builder.build(&renderer, &present_depth_texture);
    // let present_texture_pipeline = PipelineBuilder::new(
    //     vec![&present_texture_builder.bind_group_layout],
    //     vec![present_texture::Vertex::desc()],
    //     "./shaders/present_texture.wgsl",
    // )
    // .depth_enabled(false)
    // .build(&renderer);

    let g_buffer_format = wgpu::TextureFormat::Rgba32Float;
    let g_buffer_builder = renderer::RenderAssetBuilder::<
        present_texture::PresentTexture<texture::GBuffer>,
    >::new(&renderer);
    let present_texture = present_texture::PresentTexture {
        texture: texture::GBuffer {
            format: g_buffer_format,
        },
    };
    let mut position_texture = g_buffer_builder.build(&renderer, &present_texture);
    let mut normal_texture = g_buffer_builder.build(&renderer, &present_texture);
    let mut albedo_texture = g_buffer_builder.build(&renderer, &present_texture);
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

    let present_texture_pipeline = PipelineBuilder::new(
        vec![&g_buffer_builder.bind_group_layout],
        vec![present_texture::Vertex::desc()],
        "./shaders/present_texture.wgsl",
    )
    .depth_enabled(false)
    .build(&renderer);

    let mut last_render_time = std::time::Instant::now();
    let mut present_depth = false;
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

                    let pressed = if *state == ElementState::Pressed {
                        true
                    } else {
                        false
                    };
                    match key_code {
                        VirtualKeyCode::T => {
                            present_depth = pressed;
                        }
                        _ => {}
                    };
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
                    position_texture = g_buffer_builder.build(&renderer, &present_texture);
                    normal_texture = g_buffer_builder.build(&renderer, &present_texture);
                    albedo_texture = g_buffer_builder.build(&renderer, &present_texture);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    depth_texture = texture::DepthTexture.build(&renderer);
                    position_texture = g_buffer_builder.build(&renderer, &present_texture);
                    normal_texture = g_buffer_builder.build(&renderer, &present_texture);
                    albedo_texture = g_buffer_builder.build(&renderer, &present_texture);
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
                light.update(&renderer, &render_light);

                camera_controller.update_camera(&mut camera, dt);
                camera.update(&renderer, &render_camera);

                transform_1.rotation = transform_1.rotation
                    * cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(-dt.as_secs_f32() * 120.0),
                    );
                transform_1.update(&renderer, &render_transform_1);

                // transform_1_scaled.rotation = transform_1_scaled.rotation
                //     * cgmath::Quaternion::from_axis_angle(
                //         cgmath::Vector3::unit_z(),
                //         cgmath::Deg(-dt.as_secs_f32() * 120.0),
                //     );
                // transform_1_scaled.update(&renderer, &render_transform_1_scaled);

                // let model_command = model::ModelRenderCommand {
                //     pipeline: &model_pipeline,
                //     models: vec![&render_cube],
                //     transforms: vec![&render_transform_1],
                //     camera: &render_camera,
                //     light: &render_light,
                // };

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

                // let skybox_command = skybox::SkyboxRenderCommand {
                //     pipeline: &skybox_pipeline,
                //     skybox: &render_skybox,
                //     camera: &render_camera,
                // };

                let present_texture_command = present_texture::PresentTextureRenderCommand {
                    pipeline: &present_texture_pipeline,
                    screen_quad: &position_texture,
                };
                match renderer.render_deferred(
                    &vec![&model_command, &color_command],
                    &vec![&present_texture_command],
                    &vec![
                        &position_texture.texture,
                        &normal_texture.texture,
                        &albedo_texture.texture,
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
