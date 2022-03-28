// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>;
  rotate: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> transform: TransformUniform;

struct ShadowDLightUniform {
  view_projection: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> d_light: ShadowDLightUniform;

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
  [[location(1)]] tex_coords: vec2<f32>;
  [[location(2)]] normal: vec3<f32>;
  [[location(3)]] tangent: vec3<f32>;
  [[location(4)]] bitangent: vec3<f32>;
};

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  var out: VertexOutput;
  out.clip_position = d_light.view_projection * world_position;
  return out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) {}
