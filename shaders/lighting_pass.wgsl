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

struct Lights {
  lights: array<LightUniform>;
};

[[group(1), binding(0)]]
var<storage, read> lights: Lights;

[[group(0), binding(0)]]
var t_position: texture_2d<f32>;
[[group(0), binding(1)]]
var s_position: sampler;
[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;
[[group(0), binding(3)]]
var s_normal: sampler;
[[group(0), binding(4)]]
var t_albedo: texture_2d<f32>;
[[group(0), binding(5)]]
var s_albedo: sampler;

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  let vertex_position: vec4<f32> = textureSample(t_position, s_position, vertex.tex_coords);
  let vertex_normal: vec4<f32> = textureSample(t_normal, s_normal, vertex.tex_coords);
  let vertex_albedo: vec4<f32> = textureSample(t_albedo, s_albedo, vertex.tex_coords);

  var result: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0); 
  for(var i: i32 = 0; i < 2; i = i + 1) {
    
    let distance = distance(lights.lights[i].position, vertex_position.xyz);
    let attenuation = 1.0 / (lights.lights[i].constant + lights.lights[i].linear * distance + 
                      lights.lights[i].quadratic * (distance * distance));  

    //let ambient_strength = 0.1;
    //let ambient_color = properties.ambient * light.color * ambient_strength;

    let normal = vertex_normal.xyz * 2.0 - 1.0;
    let light_dir = normalize(lights.lights[i].position - vertex_position.xyz);
    let view_dir = normalize(camera.position - vertex_position.xyz);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(vertex_normal.xyz, light_dir), 0.0);
    let diffuse_color = vertex_albedo.xyz * lights.lights[i].color * diffuse_strength;

    //let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), properties.shininess);
    //let specular_color = properties.specular * lights.lights[i].color * specular_strength;

    result = result + diffuse_color * attenuation;
  }

  return vec4<f32>(result, vertex_albedo.a); 
}
