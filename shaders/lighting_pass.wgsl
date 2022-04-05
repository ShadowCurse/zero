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

[[group(3), binding(0)]]
var t_shadow: texture_depth_2d;
[[group(3), binding(1)]]
var s_shadow: sampler;

struct ShadowDLightUniform {
  view_projection: mat4x4<f32>;
};
[[group(4), binding(0)]]
var<uniform> d_light: ShadowDLightUniform;

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
  lights_num: i32;
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

fn shadow_calculations(frag_pos_light_space: vec4<f32>, bias: f32) -> f32 {
  // XY is in (-1, 1) space, Z is in (0, 1) space
  let proj_coords = (frag_pos_light_space.xyz / frag_pos_light_space.w).xyz;
  // Convert XY to (0, 1)
  // Y is flipped because texture coords are Y-down.
  let coords = proj_coords * vec3<f32>(0.5, -0.5, 1.0) + vec3<f32>(0.5, 0.5, 0.0);
  let dimentions = textureDimensions(t_shadow);
  let tex_size = vec2<f32>(1.0, 1.0) / vec2<f32>(f32(dimentions.x), f32(dimentions.y));
  var shadow: f32 = 0.0;
  for (var x: i32 = -1; x <= 1; x = x + 1) {
      for (var y: i32 = -1; y <= 1; y = y + 1) {
        let depth: f32 = textureSample(t_shadow,
                                       s_shadow,
                                       coords.xy + vec2<f32>(f32(x), f32(y)) * tex_size);
        if (coords.z - bias > depth) {
          shadow = shadow + 1.0;
        }
      }
  }
  shadow = shadow / 9.0;
  if (coords.z > 1.0) {
      return 0.0;
  }
  return shadow;
}

[[stage(fragment)]]
fn fs_main(vertex: VertexOutput) -> [[location(0)]] vec4<f32> {
  let vertex_position: vec4<f32> = textureSample(t_position, s_position, vertex.tex_coords);
  let vertex_normal: vec4<f32> = textureSample(t_normal, s_normal, vertex.tex_coords);
  let vertex_albedo: vec4<f32> = textureSample(t_albedo, s_albedo, vertex.tex_coords);

  let albedo_color = vertex_albedo.rgb;
  let shininess = vertex_albedo.a;

  let pos_in_light = d_light.view_projection * vertex_position;

  var result: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0); 
  for(var i: i32 = 0; i < lights.lights_num; i = i + 1) {
    let distance = distance(lights.lights[i].position, vertex_position.xyz);
    let attenuation = 1.0 / (lights.lights[i].constant + lights.lights[i].linear * distance + 
                      lights.lights[i].quadratic * (distance * distance));  

    let light_dir = normalize(lights.lights[i].position - vertex_position.xyz);
    let view_dir = normalize(camera.position - vertex_position.xyz);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(vertex_normal.xyz, light_dir), 0.0);
    let diffuse_color = albedo_color * lights.lights[i].color * diffuse_strength;

    let specular_strength = pow(max(dot(vertex_normal.xyz, half_dir), 0.0), shininess);
    let specular_color = lights.lights[i].color * specular_strength;
    
    let bias = max(0.001 * (1.0 - dot(vertex_normal.xyz, light_dir)), 0.0001);
    let shadow = shadow_calculations(pos_in_light, bias);

    result = (1.0 - shadow) * (result + (diffuse_color + specular_color) * attenuation);
  }
  return vec4<f32>(result, 1.0); 
}
