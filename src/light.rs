use crate::{impl_simple_buffer, render::prelude::*};
use cgmath::Vector3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectionalLightUniform {
    direction: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
    _pad2: u32,
}

impl From<&DirectionalLight> for DirectionalLightUniform {
    fn from(value: &DirectionalLight) -> Self {
        Self {
            direction: value.direction.into(),
            color: value.color.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct DirectionalLight {
    pub direction: Vector3<f32>,
    pub color: Vector3<f32>,
}

impl DirectionalLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(direction: P, color: C) -> Self {
        Self {
            direction: direction.into(),
            color: color.into(),
        }
    }
}

impl_simple_buffer!(
    DirectionalLight,
    DirectionalLightUniform,
    DirectionalLightResources,
    DirectionalLightHandle,
    DirectionalLightBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightUniform {
    position: [f32; 3],
    _pad1: u32,
    color: [f32; 3],
    _pad2: u32,
    constant: f32,
    linear: f32,
    quadratic: f32,
    _pad3: u32,
}

impl From<&PointLight> for PointLightUniform {
    fn from(value: &PointLight) -> Self {
        Self {
            position: value.position.into(),
            color: value.color.into(),
            constant: value.constant,
            linear: value.linear,
            quadratic: value.quadratic,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointLight {
    pub position: Vector3<f32>,
    pub color: Vector3<f32>,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new<P: Into<Vector3<f32>>, C: Into<Vector3<f32>>>(
        position: P,
        color: C,
        constant: f32,
        linear: f32,
        quadratic: f32,
    ) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
            constant,
            linear,
            quadratic,
        }
    }
}

impl_simple_buffer!(
    PointLight,
    PointLightUniform,
    PointLightResources,
    PointLightHandle,
    PointLightBindGroup,
    { BufferUsages::UNIFORM | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Uniform }
);

const MAX_LIGHTS: usize = 10;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightsUniform {
    // using i32 because of the wgsl
    lights_num: i32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    lights: [PointLightUniform; MAX_LIGHTS],
}

impl From<&PointLights> for PointLightsUniform {
    fn from(value: &PointLights) -> Self {
        // TODO refactor this
        let mut lights = [PointLightUniform::default(); MAX_LIGHTS];
        for (i, u) in value
            .lights
            .iter()
            .map(|light| light.into())
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            lights[i] = u;
        }
        Self {
            lights_num: value.lights.len() as i32,
            lights,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointLights {
    pub lights: Vec<PointLight>,
}

impl_simple_buffer!(
    PointLights,
    PointLightsUniform,
    PointLightsResources,
    PointLightsHandle,
    PointLightsBindGroup,
    { BufferUsages::STORAGE | BufferUsages::COPY_DST },
    { ShaderStages::VERTEX | ShaderStages::FRAGMENT },
    { BufferBindingType::Storage { read_only: true } }
);
