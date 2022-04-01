use camera::{Camera, CameraController};
use cgmath::Rotation3;
use deffered_rendering::GBuffer;
use light::{PointLight, PointLights};
use material::ColorMaterial;
use model::{Mesh, ModelVertex};
use renderer::{
    BindGroupMeta, ColorAttachment, DepthStencil, RenderCommand, RenderPhase, RenderStorage,
    RenderSystem, ResourceId,
};
use renderer::{PipelineBuilder, Renderer, Vertex};
use skybox::Skybox;
use texture::DepthTexture;
use transform::Transform;
use wgpu::{Color, LoadOp, Operations, TextureFormat};
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::skybox::SkyboxVertex;

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

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut render_system = RenderSystem::default();
    let mut storage = RenderStorage::default();

    let depth_texture_id = storage.build_texture(&renderer, &DepthTexture);

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
        cgmath::Deg(0.0),
        cgmath::Deg(0.0),
        renderer.config.width,
        renderer.config.height,
        cgmath::Deg(90.0),
        0.1,
        100.0,
    );
    let camera_id = storage.build_asset(&renderer, &camera);

    let mut camera_controller = CameraController::new(5.0, 0.7);

    let light = PointLight::new((2.0, 1.0, 0.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let light_2 = PointLight::new((-2.0, 0.8, 2.0), (0.7, 0.0, 0.8), 1.0, 0.109, 0.032);
    let light_3 = PointLight::new((-5.0, 1.5, 1.0), (0.7, 0.3, 0.3), 1.0, 0.209, 0.032);
    let lights = PointLights {
        lights: vec![light, light_2, light_3],
    };
    let lights_id = storage.build_asset(&renderer, &lights);

    let box_mesh: Mesh = shapes::Box::new(9.0, 1.0, 5.0).into();
    let box_id = storage.build_mesh(&renderer, &box_mesh);

    let box_transform = Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let box_transform_id = storage.build_asset(&renderer, &box_transform);

    let color_material = ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let color_material_id = storage.build_asset(&renderer, &color_material);

    let g_color_pipeline = PipelineBuilder::new(
        vec![
            storage.get_bind_group_layout::<ColorMaterial>(),
            storage.get_bind_group_layout::<Transform>(),
            storage.get_bind_group_layout::<Camera>(),
        ],
        vec![ModelVertex::desc()],
        "./shaders/geometry_color_pass.wgsl",
    )
    .write_depth(true)
    .color_targets(vec![TextureFormat::Rgba32Float; 3])
    .build(&renderer);
    let g_color_pipeline_id = storage.add_pipeline(g_color_pipeline);

    let lighting_pipeline = PipelineBuilder::new(
        vec![
            storage.get_bind_group_layout::<GBuffer>(),
            storage.get_bind_group_layout::<PointLights>(),
            storage.get_bind_group_layout::<Camera>(),
        ],
        vec![texture::TextureVertex::desc()],
        "./shaders/lighting_pass.wgsl",
    )
    .depth_enabled(false)
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

    let skybox_pipeline = PipelineBuilder::new(
        vec![
            storage.get_bind_group_layout::<Skybox>(),
            storage.get_bind_group_layout::<Camera>(),
        ],
        vec![SkyboxVertex::desc()],
        "./shaders/skybox.wgsl",
    )
    .write_depth(false)
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
                    storage.rebuild_texture(&renderer, &DepthTexture, depth_texture_id);
                    storage.rebuild_asset(&renderer, &g_buffer, g_buffer_id);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    storage.rebuild_texture(&renderer, &DepthTexture, depth_texture_id);
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

                let command = RenderCommand {
                    pipeline_id: g_color_pipeline_id,
                    mesh_id: box_id,
                    bind_groups: vec![
                        BindGroupMeta {
                            index: 0,
                            bind_group_id: color_material_id,
                        },
                        BindGroupMeta {
                            index: 1,
                            bind_group_id: box_transform_id,
                        },
                        BindGroupMeta {
                            index: 2,
                            bind_group_id: camera_id,
                        },
                    ],
                };
                render_system.add_phase_commands("geometry", vec![command]);

                let command = RenderCommand {
                    pipeline_id: lighting_pipeline_id,
                    mesh_id: g_buffer_id,
                    bind_groups: vec![
                        BindGroupMeta {
                            index: 0,
                            bind_group_id: g_buffer_id,
                        },
                        BindGroupMeta {
                            index: 1,
                            bind_group_id: lights_id,
                        },
                        BindGroupMeta {
                            index: 2,
                            bind_group_id: camera_id,
                        },
                    ],
                };
                render_system.add_phase_commands("lighting", vec![command]);

                let command = RenderCommand {
                    pipeline_id: skybox_pipeline_id,
                    mesh_id: skybox_id,
                    bind_groups: vec![
                        BindGroupMeta {
                            index: 0,
                            bind_group_id: skybox_id,
                        },
                        BindGroupMeta {
                            index: 1,
                            bind_group_id: camera_id,
                        },
                    ],
                };
                render_system.add_phase_commands("skybox", vec![command]);

                match render_system.run(&renderer, &storage) {
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
