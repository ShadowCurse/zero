// Vertex shader

struct CameraUniform {
  position: vec4<f32>,
  view_projection: mat4x4<f32>,
  vp_without_translation: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec3<f32>,
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {

  let position = camera.vp_without_translation* vec4<f32>(vertex.position, 1.0);
  var out: VertexOutput;
  out.clip_position = position.xyww;
  out.uv = vertex.position.xyz;

  return out;
}

// Fragment shader

@group(0) @binding(0)
var t_cube: texture_cube<f32>;
@group(0) @binding(1)
var s_cube: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(t_cube, s_cube, vertex.uv);
}
