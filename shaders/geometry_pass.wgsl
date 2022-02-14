// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>;
  rotate: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> transform: TransformUniform;

struct CameraUniform {
  position: vec3<f32>;
  view_projection: mat4x4<f32>;
  vp_without_translation: mat4x4<f32>;
};
[[group(2), binding(0)]]
var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>;
  color: vec3<f32>;
  constant: f32;
  linear: f32;
  quadratic: f32;
};
[[group(3), binding(0)]]
var<uniform> light: LightUniform;

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
  [[location(1)]] tex_coords: vec2<f32>;
  [[location(2)]] normal: vec3<f32>;
  [[location(3)]] tangent: vec3<f32>;
  [[location(4)]] bitangent: vec3<f32>;
};

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] tex_coords: vec2<f32>;
  [[location(1)]] world_normal: vec3<f32>;
  [[location(2)]] world_tangent: vec3<f32>;
  [[location(3)]] world_bitangent: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  let world_normal = normalize(transform.rotate * vec4<f32>(vertex.normal, 1.0));
  let world_tangent = normalize(transform.rotate * vec4<f32>(vertex.tangent, 1.0));
  let world_bitangent = normalize(transform.rotate * vec4<f32>(vertex.bitangent, 1.0));

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  out.tex_coords = vertex.tex_coords;
  out.world_tangent = world_tangent.xyz;
  out.world_bitangent = world_bitangent.xyz;
  out.world_normal = world_normal.xyz;

  return out;
}

// Fragment shader

struct MaterialProperties {
    ambient: vec3<f32>;
    diffuse: vec3<f32>;
    specular: vec3<f32>;
    shininess: f32;
};
[[group(0), binding(4)]]
var<uniform> properties: MaterialProperties;

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;
[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;
[[group(0), binding(3)]]
var s_normal: sampler;

struct FragmentOut {
  [[location(0)]] position: vec4<f32>;
  [[location(1)]] normal: vec4<f32>;
  [[location(2)]] albedo: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> FragmentOut {
  let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, vertex.tex_coords);

  let tangent_to_world_matrix = mat3x3<f32>(
    vertex.world_tangent,
    vertex.world_bitangent,
    vertex.world_normal,
  );

  let object_normal: vec4<f32> = textureSample(t_normal, s_normal, vertex.tex_coords);

  let world_object_normal = tangent_to_world_matrix * object_normal.xyz;

  var out: FragmentOut;
  out.position = vertex.clip_position;
  out.normal = vec4<f32>(world_object_normal, 1.0);
  out.albedo = object_color;

  return out; 
}
