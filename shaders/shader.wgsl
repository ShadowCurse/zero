// Vertex shader

struct CameraUniform {
  view_pos: vec4<f32>;
  view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>;
  color: vec3<f32>;
};
[[group(2), binding(0)]]
var<uniform> light: LightUniform;

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
  [[location(1)]] tex_coords: vec2<f32>;
  [[location(2)]] normal: vec3<f32>;
  [[location(3)]] tangent: vec3<f32>;
  [[location(4)]] bitangent: vec3<f32>;
};

struct InstanceInput {
  [[location(5)]] transform_0: vec4<f32>;
  [[location(6)]] transform_1: vec4<f32>;
  [[location(7)]] transform_2: vec4<f32>;
  [[location(8)]] transform_3: vec4<f32>;

  [[location(9)]]  rotate_0: vec3<f32>;
  [[location(10)]] rotate_1: vec3<f32>;
  [[location(11)]] rotate_2: vec3<f32>;
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
  instance: InstanceInput,
) -> VertexOutput {
  let transform = mat4x4<f32>(
    instance.transform_0,
    instance.transform_1,
    instance.transform_2,
    instance.transform_3,
  );

  let rotate = mat3x3<f32>(
    instance.rotate_0,
    instance.rotate_1,
    instance.rotate_2,
  );

  let world_position = transform * vec4<f32>(vertex.position, 1.0);

  let world_normal = normalize(rotate * vertex.normal);
  let world_tangent = normalize(rotate * vertex.tangent);
  let world_bitangent = normalize(rotate * vertex.bitangent);
  let tangent_matrix = transpose(mat3x3<f32>(
    world_tangent.xyz,
    world_bitangent.xyz,
    world_normal.xyz,
  ));

  var out: VertexOutput;
  out.clip_position = camera.view_proj * transform * vec4<f32>(vertex.position, 1.0);
  out.tex_coords = vertex.tex_coords;
  out.tangent_position = tangent_matrix * world_position.xyz;
  out.tangent_light = tangent_matrix * light.position;
  out.tangent_view = tangent_matrix * camera.view_pos.xyz;
  
  return out;
}

// Fragment shader

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
  let ambient_color = light.color * ambient_strength;

  let tangent_normal = object_normal.xyz * 2.0 - 1.0;
  let light_dir = normalize(vertex.tangent_light - vertex.tangent_position);
  let view_dir = normalize(vertex.tangent_view - vertex.tangent_position);
  let half_dir = normalize(view_dir + light_dir);

  let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
  let diffuse_color = light.color * diffuse_strength;

  let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
  let specular_color = light.color * specular_strength;

  let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

  return vec4<f32>(result, object_color.a); 
}
