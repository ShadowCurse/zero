use gltf::Gltf;
use wgpu::StoreOp;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};
use zero::{const_vec, prelude::*};

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
    // env_logger::init();
    //
    // let event_loop = EventLoop::new().unwrap();
    // let window = WindowBuilder::new().build(&event_loop).unwrap();
    //
    // let mut renderer = pollster::block_on(Renderer::new(&window));
    // let mut render_system = RenderSystem::default();
    // let mut storage = RenderStorage::default();
    //
    // storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    // storage.register_bind_group_layout::<StandartMaterialBindGroup>(&renderer);
    // storage.register_bind_group_layout::<PointLightBindGroup>(&renderer);
    // storage.register_bind_group_layout::<TransformBindGroup>(&renderer);

    // let standart_material_pipeline = PipelineBuilder {
    //     shader_path: "./examples/forward/standart_material.wgsl",
    //     label: Some("standart_material_pipeline"),
    //     layout_descriptor: Some(&PipelineLayoutDescriptor {
    //         label: None,
    //         bind_group_layouts: &[
    //             storage.get_bind_group_layout::<StandartMaterialBindGroup>(),
    //             storage.get_bind_group_layout::<TransformBindGroup>(),
    //             storage.get_bind_group_layout::<CameraBindGroup>(),
    //         ],
    //         push_constant_ranges: &[],
    //     }),
    //     vertex_layouts: &[MeshVertex::layout()],
    //     vertex_entry_point: "vs_main",
    //     color_targets: Some(&[Some(ColorTargetState {
    //         format: renderer.surface_format(),
    //         blend: None,
    //         write_mask: ColorWrites::ALL,
    //     })]),
    //     fragment_entry_point: "fs_main",
    //     primitive: PrimitiveState {
    //         topology: PrimitiveTopology::TriangleList,
    //         strip_index_format: None,
    //         front_face: FrontFace::Ccw,
    //         cull_mode: Some(Face::Back),
    //         polygon_mode: PolygonMode::Fill,
    //         unclipped_depth: false,
    //         conservative: false,
    //     },
    //     depth_stencil: Some(DepthStencilState {
    //         format: DepthTexture::DEPTH_FORMAT,
    //         depth_write_enabled: true,
    //         depth_compare: CompareFunction::LessEqual,
    //         stencil: StencilState::default(),
    //         bias: DepthBiasState::default(),
    //     }),
    //     multisample: MultisampleState::default(),
    //     multiview: None,
    // }
    // .build(&renderer);
    // let standart_material_pipeline_id = storage.insert_pipeline(standart_material_pipeline);

    // let depth_texture_id = storage.insert_texture(DepthTexture::default().build(&renderer));
    //
    // let phase = RenderPhase::new(
    //     const_vec![ColorAttachment {
    //         view_id: ResourceId::WINDOW_VIEW_ID,
    //         ops: Operations {
    //             load: LoadOp::Clear(Color::TRANSPARENT),
    //             store: StoreOp::Store,
    //         },
    //     },],
    //     Some(DepthStencil {
    //         view_id: depth_texture_id,
    //         depth_ops: Some(Operations {
    //             load: LoadOp::Clear(1.0),
    //             store: StoreOp::Store,
    //         }),
    //         stencil_ops: None,
    //     }),
    // );
    // let phase_id = render_system.add_phase(phase);
    //
    // let model = Model::load("./Main.1_Sponza/textures/NewSponza_Main_obj.obj").unwrap();
    let gltf = Gltf::open("./Main.1_Sponza/NewSponza_Main_glTF_002.gltf").unwrap();
    for scene in gltf.scenes() {
        println!("scene: {:#?}", scene.name());
        for node in scene.nodes() {
            println!("Node index: {}", node.index(),);
            println!("Node name: {:?}", node.name(),);
            println!("Node children: {}", node.children().count());
            if let Some(camera) = node.camera() {
                println!("Node camera name: {:?}", camera.name());
            }
            if let Some(mesh) = node.mesh() {
                println!("Node mesh name: {:?}", mesh.name());
                if let Some(name) = mesh.name() {
                    if name == "master_material" {
                        for p in mesh.primitives() {
                            if let Some(base_texture) =
                                p.material().pbr_metallic_roughness().base_color_texture()
                            {
                                println!(
                                    "Mesh material base texture name: {:?}",
                                    base_texture.texture().name()
                                );
                            }
                            // for a in p.attributes() {
                            //     println!("Mesh attr: {:#?}", a);
                            // }
                        }
                    }
                }
            }
        }
    }
    for material in gltf.materials() {
        println!("Material index: {:?}", material.index());
        println!("Material name: {:?}", material.name());
    }

    for buffer in gltf.buffers() {
        println!("Buffer index: {:?}", buffer.index());
        println!("Buffer name: {:?}", buffer.name());
        println!("Buffer length: {:?}", buffer.length());
        println!("Buffer source: {:?}", buffer.source());
    }

    for view in gltf.views() {
        println!("View index: {}", view.index());
        println!("View buffer index: {}", view.buffer().index());
    }

    // for accessor in gltf.accessors() {
    //     println!("Accessor index: {}", accessor.index());
    //     if let Some(v) = accessor.view() {
    //         println!("Accessor view index: {:?}", v.index());
    //     }
    // }

    // println!("loaded model with {} meshes", model.meshes.len());
    // println!("loaded model with {} materials", model.materials.len());
    // let (model_handler, _model_materials) = model.build(&renderer, &mut storage);
    //
    // let mut transform = Transform {
    //     translation: (2.0, 2.0, 4.0).into(),
    //     rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(69.0)),
    //     scale: (1.0, 1.0, 1.0).into(),
    // };
    // let transform_handle = TransformHandle::new(&mut storage, transform.build(&renderer));
    // let transform_bind_group = TransformBindGroup::new(&renderer, &mut storage, &transform_handle);
    //
    // let mut camera = Camera::new(
    //     (-10.0, 2.0, 0.0),
    //     Deg(0.0),
    //     Deg(0.0),
    //     renderer.size().width,
    //     renderer.size().height,
    //     Deg(90.0),
    //     0.1,
    //     100.0,
    // );
    // let camera_handle = CameraHandle::new(&mut storage, camera.build(&renderer));
    // let camera_bind_group = CameraBindGroup::new(&renderer, &mut storage, &camera_handle);
    //
    // let mut camera_controller = CameraController::new(5.0, 0.7);
    //
    // let light = PointLight::new((-1.0, 9.0, 5.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    // let light_handle = PointLightHandle::new(&mut storage, light.build(&renderer));
    // let light_bind_group = PointLightBindGroup::new(&renderer, &mut storage, &light_handle);
    //
    // let mut last_render_time = std::time::Instant::now();
    // let mut fps_logger = FpsLogger::new();
    // _ = event_loop.run(move |event, target| {
    //     target.set_control_flow(ControlFlow::Poll);
    //     match event {
    //         Event::DeviceEvent { ref event, .. } => match event {
    //             DeviceEvent::MouseMotion { delta } => {
    //                 camera_controller.process_mouse(delta.0, delta.1);
    //             }
    //             _ => {}
    //         },
    //         Event::WindowEvent {
    //             ref event,
    //             window_id,
    //         } if window_id == window.id() => match event {
    //             WindowEvent::CloseRequested => target.exit(),
    //             WindowEvent::MouseInput {
    //                 state,
    //                 button: MouseButton::Left,
    //                 ..
    //             } => camera_controller.set_mouse_active(*state == ElementState::Pressed),
    //             WindowEvent::KeyboardInput {
    //                 event:
    //                     KeyEvent {
    //                         logical_key: key,
    //                         state,
    //                         ..
    //                     },
    //                 ..
    //             } => match key {
    //                 Key::Named(NamedKey::Escape) => target.exit(),
    //                 k => _ = camera_controller.process_key(k.clone(), *state),
    //             },
    //             WindowEvent::Resized(physical_size) => {
    //                 camera.resize(physical_size.width, physical_size.height);
    //                 renderer.resize(Some(*physical_size));
    //                 storage.replace_texture(
    //                     depth_texture_id,
    //                     DepthTexture::default().build(&renderer),
    //                 );
    //             }
    //             WindowEvent::RedrawRequested => {
    //                 let now = std::time::Instant::now();
    //                 let dt = now - last_render_time;
    //                 last_render_time = now;
    //
    //                 fps_logger.log(now, dt);
    //
    //                 camera_controller.update_camera(&mut camera, dt);
    //                 camera_handle.update(&renderer, &storage, &camera);
    //
    //                 match render_system.run(&renderer, &storage) {
    //                     Ok(_) => {}
    //                     Err(SurfaceError::Lost) => renderer.resize(None),
    //                     Err(SurfaceError::OutOfMemory) => target.exit(),
    //                     Err(e) => eprintln!("{:?}", e),
    //                 }
    //             }
    //             _ => {}
    //         },
    //         Event::AboutToWait => window.request_redraw(),
    //         _ => {}
    //     }
    // });
}
