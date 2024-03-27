use wgpu::{BlendFactor, BlendOperation, StoreOp};
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};
use zero::{
    const_vec,
    egui::{EguiBufferBindGroup, EguiRenderContext, EguiTextureBindGroup, EguiVertex},
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

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = pollster::block_on(Renderer::new(&window));
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<EguiBufferBindGroup>(&renderer);
    storage.register_bind_group_layout::<EguiTextureBindGroup>(&renderer);

    let egui_pipeline = PipelineBuilder {
        shader_path: "./examples/egui/egui.wgsl",
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
            front_face: FrontFace::Cw,
            cull_mode: None,
            ..Default::default()
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
                store: StoreOp::Store,
            },
        }],
        None,
    );

    let mut egui_render_context = EguiRenderContext::new(&renderer, &mut storage);
    let egui_ctx = egui::Context::default();
    let mut winit_egui = egui_winit::State::new(
        egui_ctx.clone(),
        egui_winit::egui::ViewportId::ROOT,
        &window,
        None,
        None,
    );
    let mut name = String::new();
    let mut age = 0;

    let mut last_render_time = std::time::Instant::now();
    let mut fps_logger = FpsLogger::new();
    _ = event_loop.run(|event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let _response = winit_egui.on_window_event(&window, event);
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    } => target.exit(),
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(Some(*physical_size));
                    }
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now - last_render_time;
                        last_render_time = now;

                        fps_logger.log(now, dt);

                        let egui_input = winit_egui.take_egui_input(&window);
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
                        winit_egui.handle_platform_output(&window, egui_out.platform_output);
                        egui_render_context.update_textures(
                            &renderer,
                            &mut storage,
                            egui_out.textures_delta,
                        );
                        let clipped = egui_ctx.tessellate(egui_out.shapes, 1.0);
                        egui_render_context.update_meshes(&renderer, &mut storage, &clipped);

                        let current_frame_context = match renderer.current_frame() {
                            Ok(cfc) => cfc,
                            Err(SurfaceError::Lost) => {
                                renderer.resize(None);
                                return;
                            }
                            Err(SurfaceError::OutOfMemory) => {
                                target.exit();
                                return;
                            }
                            Err(e) => {
                                eprintln!("{:?}", e);
                                return;
                            }
                        };

                        let current_frame_storage = CurrentFrameStorage {
                            storage: &storage,
                            current_frame_view: current_frame_context.view(),
                        };

                        let mut encoder = renderer.create_encoder();

                        let commands =
                            egui_render_context.create_commands(egui_pipeline_id, &clipped);

                        {
                            let mut render_pass =
                                egui_phase.render_pass(&mut encoder, &current_frame_storage);
                            for command in commands {
                                command.execute(&mut render_pass, &current_frame_storage);
                            }
                        }

                        let commands = encoder.finish();
                        renderer.submit(std::iter::once(commands));
                        current_frame_context.present();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        }
    });
}
