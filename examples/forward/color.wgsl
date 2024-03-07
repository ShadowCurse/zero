// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>,
  rotate: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> transform: TransformUniform;

struct CameraUniform {
  view: mat4x4<f32>,
  projection: mat4x4<f32>,
  view_projection: mat4x4<f32>,
  view_projection_inverse: mat4x4<f32>,
  view_projection_without_translation: mat4x4<f32>,
  position: vec3<f32>,
};
@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) tangent: vec3<f32>,
  @location(4) bitangent: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  return out;
}

// Fragment shader

struct MaterialProperties {
    ambient: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32,
};
@group(0) @binding(0)
var<uniform> properties: MaterialProperties;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(properties.diffuse, 1.0); 
}
