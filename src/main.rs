use camera::{Camera, CameraController};
use render_phase::{
    ColorAttachment, RenderPhase, RenderStorage, RenderSystem, ResourceId, RenderCommand, BindGroupMeta,
};
use renderer::{Renderer, PipelineBuilder, Vertex};
use skybox::Skybox;
use texture::DepthTexture;
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
mod render_phase;
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

    let phase = RenderPhase::new(
        vec![ColorAttachment {
            view_id: ResourceId::WINDOW_VIEW_ID,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        }],
        // Some(DepthStencil {
        //     view_id: depth_texture_id,
        //     depth_ops: Some(wgpu::Operations {
        //         load: wgpu::LoadOp::Load,
        //         store: true,
        //     }),
        //     stencil_ops: None,
        // }),
        None,
    );
    render_system.add_phase("skybox", phase);

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
    .depth_enabled(false)
    .build(&renderer);
    let skybox_pipeline_id = storage.add_pipeline(skybox_pipeline);
    
    let command = RenderCommand {
        pipeline_id: skybox_pipeline_id,
        mesh_id: skybox_id,
        bind_groups: vec![BindGroupMeta {
            index: 0,
            bind_group_id: skybox_id,
        },
        BindGroupMeta {
            index: 1,
            bind_group_id: camera_id,
        }]
    };

    render_system.add_phase_commands("skybox", vec![command]);

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
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    camera.resize(new_inner_size.width, new_inner_size.height);
                    renderer.resize(Some(**new_inner_size));
                    storage.rebuild_texture(&renderer, &DepthTexture, depth_texture_id);
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                camera_controller.update_camera(&mut camera, dt);
                storage.rebuild_asset(&renderer, &camera, camera_id);

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
