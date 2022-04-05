// Vertex shader

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
  [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] tex_coords: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  var out: VertexOutput;
  out.clip_position = vec4<f32>(vertex.position, 1.0);
  out.tex_coords = vertex.tex_coords;
  return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var s_sampler: sampler;

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  let color: f32 = textureSample(t_texture, s_sampler, vertex.tex_coords).r;
  return vec4<f32>(color, color, color, 1.0);
}
