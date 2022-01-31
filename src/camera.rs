use cgmath::{perspective, InnerSpace, Matrix3, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::event::{ElementState, VirtualKeyCode};

use crate::renderer;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>, F: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: P,
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn to_uniform(&self) -> CameraUniform {
        let projection = self.projection();
        CameraUniform {
            position: self.position.to_homogeneous().into(),
            view_projection: (projection * self.view()).into(),
            vp_without_translation: (projection * self.view_without_translation()).into(),
        }
    }

    fn view_without_translation(&self) -> Matrix4<f32> {
        let view = self.view();
        Matrix4::from(Matrix3::from_cols(
            Vector3::from(view[0].truncate()),
            Vector3::from(view[1].truncate()),
            Vector3::from(view[2].truncate()),
        ))
    }

    fn view(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX
            * Matrix4::look_to_rh(
                self.position,
                Vector3::new(self.yaw.0.cos(), self.pitch.0.sin(), self.yaw.0.sin()).normalize(),
                Vector3::unit_y(),
            )
    }

    fn projection(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    position: [f32; 4],
    view_projection: [[f32; 4]; 4],
    vp_without_translation: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn update(&mut self, camera: &Camera) {
        *self = camera.to_uniform();
    }
}

pub struct RenderCamera {
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderCamera {
    pub fn new(renderer: &renderer::Renderer, camera: &Camera) -> Self {
        let uniform = camera.to_uniform();

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_binding_group_layout"),
                });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("comera_bind_group"),
            });

        Self {
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, renderer: &renderer::Renderer, camera: &Camera) {
        self.uniform.update(camera);
        renderer
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}

#[derive(Debug, Default)]
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    forward: i8,
    backward: i8,
    left: i8,
    right: i8,
    up: i8,
    down: i8,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    mouse_active: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            ..Default::default()
        }
    }

    pub fn process_key(&mut self, keycode: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = if state == ElementState::Pressed { 1 } else { 0 };
        match keycode {
            VirtualKeyCode::W => {
                self.forward = pressed;
                true
            }
            VirtualKeyCode::S => {
                self.backward = pressed;
                true
            }
            VirtualKeyCode::A => {
                self.left = pressed;
                true
            }
            VirtualKeyCode::D => {
                self.right = pressed;
                true
            }
            VirtualKeyCode::Space => {
                self.up = pressed;
                true
            }
            VirtualKeyCode::LShift => {
                self.down = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn set_mouse_active(&mut self, active: bool) {
        self.mouse_active = active;
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.mouse_active {
            self.rotate_horizontal = mouse_dx as f32;
            self.rotate_vertical = -mouse_dy as f32;
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        camera.position += forward * (self.forward - self.backward) as f32 * self.speed * dt;
        camera.position += right * (self.right - self.left) as f32 * self.speed * dt;

        camera.position.y += (self.up - self.down) as f32 * self.speed * dt;

        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(self.rotate_vertical) * self.sensitivity * dt;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}
