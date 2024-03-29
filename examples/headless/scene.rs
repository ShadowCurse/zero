use zero::prelude::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() {
    env_logger::init();

    let renderer = pollster::block_on(Renderer::new(WIDTH, HEIGHT));
    let mut render_system = RenderSystem::default();
    let mut storage = RenderStorage::default();

    storage.register_bind_group_layout::<CameraBindGroup>(&renderer);
    storage.register_bind_group_layout::<MaterialBindGroup>(&renderer);
    storage.register_bind_group_layout::<ColorMaterialBindGroup>(&renderer);
    storage.register_bind_group_layout::<GBufferBindGroup>(&renderer);
    storage.register_bind_group_layout::<PointLightBindGroup>(&renderer);
    storage.register_bind_group_layout::<PointLightsBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowMapBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowMapDLightBindGroup>(&renderer);
    storage.register_bind_group_layout::<ShadowBindGroup>(&renderer);
    storage.register_bind_group_layout::<SkyboxBindGroup>(&renderer);
    storage.register_bind_group_layout::<TransformBindGroup>(&renderer);

    let g_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<MaterialBindGroup>(),
            storage.get_bind_group_layout::<TransformBindGroup>(),
            storage.get_bind_group_layout::<CameraBindGroup>(),
        ],
        vertex_layouts: vec![MeshVertex::layout()],
        shader_path: "./shaders/geometry_pass.wgsl",
        write_depth: true,
        color_targets: Some(vec![TextureFormat::Rgba32Float; 3]),
        ..Default::default()
    }
    .build(&renderer);
    let g_pipeline_id = storage.insert_pipeline(g_pipeline);

    let g_color_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<ColorMaterialBindGroup>(),
            storage.get_bind_group_layout::<TransformBindGroup>(),
            storage.get_bind_group_layout::<CameraBindGroup>(),
        ],
        vertex_layouts: vec![MeshVertex::layout()],
        shader_path: "./shaders/geometry_color_pass.wgsl",
        write_depth: true,
        color_targets: Some(vec![TextureFormat::Rgba32Float; 3]),
        ..Default::default()
    }
    .build(&renderer);
    let g_color_pipeline_id = storage.insert_pipeline(g_color_pipeline);

    let shadow_map_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<TransformBindGroup>(),
            storage.get_bind_group_layout::<ShadowMapDLightBindGroup>(),
        ],
        vertex_layouts: vec![MeshVertex::layout()],
        shader_path: "./shaders/shadow_map.wgsl",
        write_depth: true,
        cull_mode: Face::Front,
        ..Default::default()
    }
    .build(&renderer);
    let shadow_map_pipeline_id = storage.insert_pipeline(shadow_map_pipeline);

    let lighting_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<GBufferBindGroup>(),
            storage.get_bind_group_layout::<PointLightsBindGroup>(),
            storage.get_bind_group_layout::<CameraBindGroup>(),
            storage.get_bind_group_layout::<ShadowBindGroup>(),
        ],
        vertex_layouts: vec![TextureVertex::layout()],
        shader_path: "./shaders/lighting_pass.wgsl",
        depth_enabled: false,
        color_targets: Some(vec![renderer.surface_format()]),
        ..Default::default()
    }
    .build(&renderer);
    let lighting_pipeline_id = storage.insert_pipeline(lighting_pipeline);

    let skybox_pipeline = PipelineBuilder {
        bind_group_layouts: vec![
            storage.get_bind_group_layout::<SkyboxBindGroup>(),
            storage.get_bind_group_layout::<CameraBindGroup>(),
        ],
        vertex_layouts: vec![SkyboxVertex::layout()],
        shader_path: "./shaders/skybox.wgsl",
        write_depth: false,
        color_targets: Some(vec![renderer.surface_format()]),
        ..Default::default()
    }
    .build(&renderer);
    let skybox_pipeline_id = storage.insert_pipeline(skybox_pipeline);

    let depth_texture_id = storage.insert_texture(EmptyTexture::default().build(&renderer));
    let shadow_map_handle =
        ShadowMapHandle::new(&mut storage, ShadowMap::default().build(&renderer));

    let g_buffer = GBuffer::new(TextureFormat::Rgba32Float);
    let g_buffer_handle = GBufferHandle::new(&mut storage, g_buffer.build(&renderer));
    let g_buffer_bind_group = GBufferBindGroup::new(&renderer, &mut storage, &g_buffer_handle);

    let geometry_phase = RenderPhase::new(
        vec![
            ColorAttachment {
                view_id: g_buffer_handle.position_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: true,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.normal_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: true,
                },
            },
            ColorAttachment {
                view_id: g_buffer_handle.albedo_texture_id,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: true,
                },
            },
        ],
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

    let shadow_phase = RenderPhase::new(
        vec![],
        Some(DepthStencil {
            view_id: shadow_map_handle.texture_id,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    );
    render_system.add_phase("shadow", shadow_phase);

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

    let camera = Camera::new(
        (-10.0, 2.0, 0.0),
        Deg(0.0),
        Deg(0.0),
        renderer.size().width,
        renderer.size().height,
        Deg(90.0),
        0.1,
        100.0,
    );
    let camera_handle = CameraHandle::new(&mut storage, camera.build(&renderer));
    let camera_bind_group = CameraBindGroup::new(&renderer, &mut storage, &camera_handle);

    let light = PointLight::new((-1.0, 9.0, 5.0), (1.0, 1.0, 1.0), 1.0, 0.109, 0.032);
    let light_2 = PointLight::new((-2.0, 1.0, -2.0), (0.8, 0.1, 0.1), 1.0, 0.109, 0.032);
    let light_3 = PointLight::new((-2.0, 1.0, 0.0), (0.1, 0.8, 0.1), 1.0, 0.209, 0.032);
    let light_4 = PointLight::new((-2.0, 1.0, 2.0), (0.1, 0.1, 0.8), 1.0, 0.209, 0.032);
    let lights = PointLights {
        lights: vec![light, light_2, light_3, light_4],
    };
    let lights_handle = PointLightsHandle::new(&mut storage, lights.build(&renderer));
    let lights_bind_group = PointLightsBindGroup::new(&renderer, &mut storage, &lights_handle);

    let shadow_d_light = ShadowMapDLight::new(
        (-2.0, 9.0, 8.0),
        (1.0, -3.0, -3.0),
        -10.0,
        10.0,
        -10.0,
        10.0,
        0.1,
        8.0,
    );
    let shadow_d_light_handle =
        ShadowMapDLightHandle::new(&mut storage, shadow_d_light.build(&renderer));
    let shadow_d_light_bind_group =
        ShadowMapDLightBindGroup::new(&renderer, &mut storage, &shadow_d_light_handle);

    let shadow_bind_group = ShadowBindGroup::new(
        &renderer,
        &mut storage,
        &(shadow_map_handle, shadow_d_light_handle),
    );

    let box_mesh: Mesh = Cube::new(9.0, 1.0, 5.0).into();
    let box_id = storage.insert_mesh(box_mesh.build(&renderer));

    let box_transform = Transform {
        translation: (0.0, 0.0, 0.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (3.0, 1.0, 3.0).into(),
    };
    let box_transform_handle = TransformHandle::new(&mut storage, box_transform.build(&renderer));
    let box_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &box_transform_handle);

    let box2_mesh: Mesh = Cube::new(1.0, 1.0, 1.0).into();
    let box2_id = storage.insert_mesh(box2_mesh.build(&renderer));

    let box2_transform = Transform {
        translation: (0.0, 1.0, 1.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), Deg(0.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let box2_transform_handle = TransformHandle::new(&mut storage, box2_transform.build(&renderer));
    let box2_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &box2_transform_handle);

    let grey_material = ColorMaterial {
        ambient: [0.4, 0.4, 0.4],
        diffuse: [0.6, 0.6, 0.6],
        specular: [1.0, 1.0, 1.0],
        shininess: 32.0,
    };
    let grey_material_handle =
        ColorMaterialHandle::new(&mut storage, grey_material.build(&renderer));
    let grey_material_bind_group =
        ColorMaterialBindGroup::new(&renderer, &mut storage, &grey_material_handle);

    let green_material = ColorMaterial {
        ambient: [0.4, 0.9, 0.4],
        diffuse: [0.4, 0.9, 0.4],
        specular: [0.1, 0.1, 0.1],
        shininess: 1.0,
    };
    let green_material_handle =
        ColorMaterialHandle::new(&mut storage, green_material.build(&renderer));
    let green_material_bind_group =
        ColorMaterialBindGroup::new(&renderer, &mut storage, &green_material_handle);

    let cube_model = Model::load("./res/cube/cube.obj").unwrap();
    let (cube_model_handler, _cube_model_materials) = cube_model.build(&renderer, &mut storage);

    let cube_transform = Transform {
        translation: (2.0, 2.0, 4.0).into(),
        rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(69.0)),
        scale: (1.0, 1.0, 1.0).into(),
    };
    let cube_transform_handle = TransformHandle::new(&mut storage, cube_transform.build(&renderer));
    let cube_transform_bind_group =
        TransformBindGroup::new(&renderer, &mut storage, &cube_transform_handle);

    let skybox = Skybox::load([
        "./res/skybox/right.jpg",
        "./res/skybox/left.jpg",
        "./res/skybox/top.jpg",
        "./res/skybox/bottom.jpg",
        "./res/skybox/front.jpg",
        "./res/skybox/back.jpg",
    ])
    .unwrap();
    let skybox_handle = SkyboxHandle::new(&mut storage, skybox.build(&renderer));
    let skybox_bind_group = SkyboxBindGroup::new(&renderer, &mut storage, &skybox_handle);

    let box1 = RenderCommand::new(
        g_color_pipeline_id,
        box_id,
        vec![
            grey_material_bind_group.0,
            box_transform_bind_group.0,
            camera_bind_group.0,
        ],
    );
    let box2 = RenderCommand::new(
        g_color_pipeline_id,
        box2_id,
        vec![
            green_material_bind_group.0,
            box2_transform_bind_group.0,
            camera_bind_group.0,
        ],
    );
    let cube = RenderCommand::new(
        g_pipeline_id,
        cube_model_handler[0].mesh_id,
        vec![
            cube_model_handler[0].material_bind_group.0,
            cube_transform_bind_group.0,
            camera_bind_group.0,
        ],
    );
    render_system.add_phase_commands("geometry", vec![box1, box2, cube]);

    let box1 = RenderCommand::new(
        shadow_map_pipeline_id,
        box_id,
        vec![box_transform_bind_group.0, shadow_d_light_bind_group.0],
    );
    let box2 = RenderCommand::new(
        shadow_map_pipeline_id,
        box2_id,
        vec![box2_transform_bind_group.0, shadow_d_light_bind_group.0],
    );
    let cube = RenderCommand::new(
        shadow_map_pipeline_id,
        cube_model_handler[0].mesh_id,
        vec![cube_transform_bind_group.0, shadow_d_light_bind_group.0],
    );
    render_system.add_phase_commands("shadow", vec![box1, box2, cube]);

    let command = RenderCommand::new(
        lighting_pipeline_id,
        g_buffer_handle.mesh_id,
        vec![
            g_buffer_bind_group.0,
            lights_bind_group.0,
            camera_bind_group.0,
            shadow_bind_group.0,
        ],
    );
    render_system.add_phase_commands("lighting", vec![command]);

    let command = RenderCommand::new(
        skybox_pipeline_id,
        skybox_handle.mesh_id,
        vec![skybox_bind_group.0, camera_bind_group.0],
    );
    render_system.add_phase_commands("skybox", vec![command]);

    render_system.run(&renderer, &storage);

    let texture_buffer = TextureBuffer::new(&renderer, WIDTH, HEIGHT);
    texture_buffer.copy_render_surface_to_texture(&renderer);

    pollster::block_on(texture_buffer.get_image_buffer(&renderer))
        .unwrap()
        .save("image.png")
        .unwrap();
}
