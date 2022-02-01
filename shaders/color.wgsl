// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>;
  rotate: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> transform: TransformUniform;

struct CameraUniform {
  position: vec4<f32>;
  view_projection: mat4x4<f32>;
  vp_without_translation: mat4x4<f32>;
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
  [[location(1)]] world_position: vec3<f32>;
  [[location(2)]] world_normal: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);
  let world_normal = normalize(transform.rotate * vec4<f32>(vertex.normal, 1.0));

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  out.tex_coords = vertex.tex_coords;
  out.world_position = world_position.xyz;
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
[[group(0), binding(0)]]
var<uniform> properties: MaterialProperties;

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  let ambient_strength = 0.1;
  let ambient_color = properties.ambient * light.color * ambient_strength;

  let light_dir = normalize(light.position - vertex.world_position);
  let view_dir = normalize(camera.position.xyz - vertex.world_position);
  let half_dir = normalize(view_dir + light_dir);

  let diffuse_strength = max(dot(vertex.world_normal, light_dir), 0.0);
  let diffuse_color = properties.diffuse * light.color * diffuse_strength;

  let specular_strength = pow(max(dot(vertex.world_normal, half_dir), 0.0), properties.shininess);
  let specular_color = properties.specular * light.color * specular_strength;

  let result = ambient_color + diffuse_color + specular_color;

  return vec4<f32>(result, 1.0); 
}
