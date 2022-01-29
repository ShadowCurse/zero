// Vertex shader

struct CameraUniform {
  position: vec4<f32>;
  view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] position: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {

  // removing translation from the view_proj matrix
  let vp = mat3x3<f32>(camera.view_proj[0].xyz, camera.view_proj[1].xyz, camera.view_proj[2].xyz);
  let pos = vp * vertex.position;
  var out: VertexOutput;
  out.clip_position = vec4<f32>(pos, 1.0).xyww;
  out.position = pos;

  return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_cube: texture_cube<f32>;
[[group(0), binding(1)]]
var s_cube: sampler;

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  return textureSample(t_cube, s_cube, vertex.position);
}
