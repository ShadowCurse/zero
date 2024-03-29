use crate::render::prelude::*;
use crate::{cgmath_imports::*, impl_simple_buffer};
use cgmath::SquareMatrix;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    view_projection: [[f32; 4]; 4],
    view_projection_inverse: [[f32; 4]; 4],
    view_projection_without_translation: [[f32; 4]; 4],
    position: [f32; 3],
    _pad: f32,
}

impl From<&Camera> for CameraUniform {
    fn from(value: &Camera) -> Self {
        let view = value.view();
        let projection = value.projection();
        let position = match value {
            Camera::Perspective(c) => c.position,
            Camera::Orthogonal(c) => c.position,
        };
        let vp = projection * view;
        Self {
            view: view.into(),
            projection: projection.into(),
            view_projection: vp.into(),
            view_projection_inverse: vp.invert().unwrap().into(),
            view_projection_without_translation: (projection * value.view_without_translation())
                .into(),
            position: position.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct PerspectiveCamera {
    pub position: Point3<f32>,
    pub yaw: Rad<f32>,
    pub pitch: Rad<f32>,
    pub aspect: f32,
    pub fovy: Rad<f32>,
    pub znear: f32,
    pub zfar: f32,
}

#[derive(Debug)]
pub struct OrthogonalCamera {
    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug)]
pub enum Camera {
    Perspective(PerspectiveCamera),
    Orthogonal(OrthogonalCamera),
}

impl Camera {
    pub fn resize(&mut self, width: u32, height: u32) {
        match self {
            Camera::Perspective(c) => c.aspect = width as f32 / height as f32,
            Camera::Orthogonal(_) => {}
        }
    }

    pub fn view_without_translation(&self) -> Matrix4<f32> {
        let view = self.view();
        Matrix4::from(Matrix3::from_cols(
            view[0].truncate(),
            view[1].truncate(),
            view[2].truncate(),
        ))
    }

    pub fn view(&self) -> Matrix4<f32> {
        let matrix = match self {
            Camera::Perspective(c) => Matrix4::look_to_rh(
                c.position,
                Vector3::new(c.yaw.0.cos(), c.pitch.0.sin(), c.yaw.0.sin()).normalize(),
                Vector3::unit_y(),
            ),
            Camera::Orthogonal(c) => {
                Matrix4::look_to_rh(c.position, c.direction, Vector3::unit_y())
            }
        };
        OPENGL_TO_WGPU_MATRIX * matrix
    }

    pub fn projection(&self) -> Matrix4<f32> {
        let matrix = match self {
            Camera::Perspective(c) => perspective(c.fovy, c.aspect, c.znear, c.zfar),
            Camera::Orthogonal(c) => ortho(c.left, c.right, c.bottom, c.top, c.near, c.far),
        };
        OPENGL_TO_WGPU_MATRIX * matrix
    }
}

impl_simple_buffer!(
    Camera,
    CameraUniform,
    CameraResources,
    CameraHandle,
    CameraBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);

#[derive(Debug, Default)]
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
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

    pub fn process_key(&mut self, key: Key, state: ElementState) -> bool {
        let pressed = if state == ElementState::Pressed { 1 } else { 0 };
        match key {
            Key::Named(NamedKey::Space) => {
                self.up = pressed;
                true
            }
            Key::Named(NamedKey::Shift) => {
                self.down = pressed;
                true
            }
            Key::Character(c) => match c.as_str() {
                "w" | "W" => {
                    self.forward = pressed;
                    true
                }
                "s" | "S" => {
                    self.backward = pressed;
                    true
                }
                "a" | "A" => {
                    self.left = pressed;
                    true
                }
                "d" | "D" => {
                    self.right = pressed;
                    true
                }
                _ => false,
            },
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
        match camera {
            Camera::Perspective(camera) => {
                let dt = dt.as_secs_f32();

                let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
                let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
                let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

                camera.position +=
                    forward * (self.forward - self.backward) as f32 * self.speed * dt;
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
            Camera::Orthogonal(_) => {}
        }
    }
}
