// Vertex shader

struct CameraUniform {
  view_pos: vec4<f32>;
  view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

struct VertexInput {
  [[location(0)]] position: vec3<f32>;
  [[location(1)]] tex_coords: vec2<f32>;
  [[location(2)]] normal: vec3<f32>;
};

struct InstanceInput {
  [[location(5)]] translation_0: vec4<f32>;
  [[location(6)]] translation_1: vec4<f32>;
  [[location(7)]] translation_2: vec4<f32>;
  [[location(8)]] translation_3: vec4<f32>;

  [[location(9)]] rotation_0: vec4<f32>;
  [[location(10)]] rotation_1: vec4<f32>;
  [[location(11)]] rotation_2: vec4<f32>;
  [[location(12)]] rotation_3: vec4<f32>;

  [[location(13)]] scale_0: vec4<f32>;
  [[location(14)]] scale_1: vec4<f32>;
  [[location(15)]] scale_2: vec4<f32>;
  [[location(16)]] scale_3: vec4<f32>;
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
  instance: InstanceInput,
) -> VertexOutput {
  let translation = mat4x4<f32>(
    instance.translation_0,
    instance.translation_1,
    instance.translation_2,
    instance.translation_3,
  );

  let rotation = mat4x4<f32>(
    instance.rotation_0,
    instance.rotation_1,
    instance.rotation_2,
    instance.rotation_3,
  );

  let scale = mat4x4<f32>(
    instance.scale_0,
    instance.scale_1,
    instance.scale_2,
    instance.scale_3,
  );

    var transform = translation * rotation * scale;
    var world_position = transform * vec4<f32>(vertex.position, 1.0);
    var world_normal = rotation * vec4<f32>(vertex.normal, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * transform * vec4<f32>(vertex.position, 1.0);
    out.tex_coords = vertex.tex_coords;
    out.world_position = world_position.xyz;
    out.world_normal = world_normal.xyz;
    
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

  let ambient_strength = 0.01;
  let ambient_color = vec3<f32>(1.0, 1.0, 1.0) * ambient_strength;

  let light_dir = normalize(vec3<f32>(5.0, 0.0, 0.0) - vertex.world_position);
  let view_dir = normalize(camera.view_pos.xyz - vertex.world_position);
  let half_dir = normalize(view_dir + light_dir);

  let diffuse_strength = max(dot(vertex.world_normal, light_dir), 0.0);
  let diffuse_color = vec3<f32>(1.0, 1.0, 1.0) * diffuse_strength;

  let specular_strength = pow(max(dot(vertex.world_normal, half_dir), 0.0), 32.0);
  let specular_color = vec3<f32>(1.0, 1.0, 1.0) * specular_strength;

  let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

  return vec4<f32>(result, object_color.a); 
}
