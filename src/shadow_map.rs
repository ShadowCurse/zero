use crate::camera;
use crate::render_phase::RenderResources;
use crate::render_phase::RenderStorage;
use crate::render_phase::ResourceId;
use crate::renderer::{self, GpuAsset, RenderAsset, Renderer};
use crate::texture::GpuTexture;
use cgmath::{ortho, EuclideanSpace, InnerSpace, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;

#[derive(Debug, Default)]
pub struct ShadowMapTexture;

impl ShadowMapTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl GpuAsset for ShadowMapTexture {
    type GpuType = GpuTexture;

    fn build(&self, renderer: &renderer::Renderer) -> Self::GpuType {
        let size = wgpu::Extent3d {
            width: renderer.config.width,
            height: renderer.config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("depth_map_texture"),
        };
        let texture = renderer.device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None, //Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self::GpuType {
            texture,
            view,
            sampler,
        }
    }
}

// #[derive(Debug)]
// pub struct RenderShadowMap {
//     pub shadow_map: GpuTexture,
//     bind_group: wgpu::BindGroup,
// }
//
// impl RenderResource for RenderShadowMap {
//     fn bind_group(&self) -> &wgpu::BindGroup {
//         &self.bind_group
//     }
// }

#[derive(Debug, Default)]
pub struct ShadowMap {
    pub shadow_map: ShadowMapTexture,
}

impl RenderAsset for ShadowMap {
    // type RenderType = RenderShadowMap;
    const ASSET_NAME: &'static str = "ShadowMap";

    fn bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("shadow_map_bind_group_layout"),
            })
    }

    fn build(&self, renderer: &Renderer, layout: &wgpu::BindGroupLayout) -> RenderResources {
        let shadow_map = self.shadow_map.build(renderer);

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                    },
                ],
                label: None,
            });
        RenderResources {
            textures: vec![shadow_map],
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowMapDLightUniform {
    view_projection: [[f32; 4]; 4],
}

// #[derive(Debug)]
// pub struct RenderShadowMapDLight {
//     buffer: wgpu::Buffer,
//     bind_group: wgpu::BindGroup,
// }
//
// impl renderer::RenderResource for RenderShadowMapDLight {
//     fn bind_group(&self) -> &wgpu::BindGroup {
//         &self.bind_group
//     }
// }

#[derive(Debug)]
pub struct ShadowMapDLight {
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl ShadowMapDLight {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P: Into<Point3<f32>>, D: Into<Vector3<f32>>>(
        position: P,
        direction: D,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position: position.into(),
            direction: direction.into(),
            left,
            right,
            bottom,
            top,
            near,
            far,
        }
    }

    fn view(&self) -> Matrix4<f32> {
        camera::OPENGL_TO_WGPU_MATRIX
            * Matrix4::look_to_rh(
                self.position,
                (self.position.to_vec() + self.direction).normalize(),
                Vector3::unit_y(),
            )
    }

    fn projection(&self) -> Matrix4<f32> {
        camera::OPENGL_TO_WGPU_MATRIX
            * ortho(
                self.left,
                self.right,
                self.bottom,
                self.top,
                self.near,
                self.far,
            )
    }

    fn to_uniform(&self) -> ShadowMapDLightUniform {
        ShadowMapDLightUniform {
            view_projection: (self.projection() * self.view()).into(),
        }
    }
}

impl renderer::RenderAsset for ShadowMapDLight {
    // type RenderType = RenderShadowMapDLight;
    const ASSET_NAME: &'static str = "ShadowMapDLight";

    fn bind_group_layout(renderer: &renderer::Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("shadow_map_binding_group_layout"),
            })
    }

    fn build(
        &self,
        renderer: &renderer::Renderer,
        layout: &wgpu::BindGroupLayout,
    ) -> RenderResources {
        let uniform = self.to_uniform();

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow_map_dlight_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("comera_bind_group"),
            });

        RenderResources {
            buffers: vec![buffer],
            bind_group: Some(bind_group),
            ..Default::default()
        }
    }

    fn update(&self, renderer: &Renderer, id: ResourceId, storage: &RenderStorage) {
        renderer.queue.write_buffer(
            &storage.get_buffers(id)[0],
            0,
            bytemuck::cast_slice(&[self.to_uniform()]),
        );
    }
    // fn update(&self, renderer: &renderer::Renderer, : &Self::RenderType) {
    //     renderer.queue.write_buffer(
    //         &render_type.buffer,
    //         0,
    //         bytemuck::cast_slice(&[self.to_uniform()]),
    //     );
    // }
}

// #[derive(Debug)]
// pub struct ShadowMapRenderCommand<'a> {
//     pub pipeline: &'a wgpu::RenderPipeline,
//     pub mesh: &'a model::GpuMesh,
//     pub transform: &'a transform::RenderTransform,
//     pub dlight: &'a RenderShadowMapDLight,
// }
//
// impl<'a> renderer::RenderCommand<'a> for ShadowMapRenderCommand<'a> {
//     fn execute<'b>(&self, render_pass: &mut wgpu::RenderPass<'b>)
//     where
//         'a: 'b,
//     {
//         render_pass.set_pipeline(self.pipeline);
//         render_pass.set_bind_group(0, self.transform.bind_group(), &[]);
//         render_pass.set_bind_group(1, self.dlight.bind_group(), &[]);
//         render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//         render_pass.draw_indexed(0..self.mesh.num_elements, 0, 0..1);
//     }
// }
