// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>;
  rotate: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> transform: TransformUniform;

struct CameraUniform {
  position: vec4<f32>;
  view_proj: mat4x4<f32>;
};
[[group(2), binding(0)]]
var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>;
  color: vec3<f32>;
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
  [[location(1)]] tangent_position: vec3<f32>;
  [[location(2)]] tangent_light: vec3<f32>;
  [[location(3)]] tangent_view: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  let world_normal = normalize(transform.rotate * vec4<f32>(vertex.normal, 1.0));
  let world_tangent = normalize(transform.rotate * vec4<f32>(vertex.tangent, 1.0));
  let world_bitangent = normalize(transform.rotate * vec4<f32>(vertex.bitangent, 1.0));
  let tangent_matrix = transpose(mat3x3<f32>(
    world_tangent.xyz,
    world_bitangent.xyz,
    world_normal.xyz,
  ));

  var out: VertexOutput;
  out.clip_position = camera.view_proj * world_position;
  out.tex_coords = vertex.tex_coords;
  out.tangent_position = tangent_matrix * world_position.xyz;
  out.tangent_light = tangent_matrix * light.position;
  out.tangent_view = tangent_matrix * camera.position.xyz;

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

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, vertex.tex_coords);
  let object_normal: vec4<f32> = textureSample(t_normal, s_normal, vertex.tex_coords);

  let ambient_strength = 0.1;
  let ambient_color = properties.ambient * light.color * ambient_strength;

  let tangent_normal = object_normal.xyz * 2.0 - 1.0;
  let light_dir = normalize(vertex.tangent_light - vertex.tangent_position);
  let view_dir = normalize(vertex.tangent_view - vertex.tangent_position);
  let half_dir = normalize(view_dir + light_dir);

  let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
  let diffuse_color = properties.diffuse * light.color * diffuse_strength;

  let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), properties.shininess);
  let specular_color = properties.specular * light.color * specular_strength;

  let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

  return vec4<f32>(result, object_color.a); 
}
