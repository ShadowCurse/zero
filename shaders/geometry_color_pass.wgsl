// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>,
  rotate: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> transform: TransformUniform;

struct CameraUniform {
  position: vec3<f32>,
  view_projection: mat4x4<f32>,
  vp_without_translation: mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) tangent: vec3<f32>,
  @location(4) bitangent: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) world_position: vec4<f32>,
  @location(1) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);
  let world_normal = normalize(transform.rotate * vec4<f32>(vertex.normal, 1.0));

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  out.world_position = world_position;
  out.world_normal = world_normal.xyz;

  return out;
}

// Fragment shader

struct MaterialProperties {
    ambient: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32,
};
@group(0) @binding(0)
var<uniform> properties: MaterialProperties;

struct FragmentOut {
  @location(0) position: vec4<f32>,
  @location(1) normal: vec4<f32>,
  @location(2) albedo: vec4<f32>,
};

@fragment
fn fs_main(vertex: VertexOutput) -> FragmentOut {
  var out: FragmentOut;
  out.position = vertex.world_position;
  out.normal = vec4<f32>(vertex.world_normal, 1.0);
  out.albedo = vec4<f32>(properties.ambient, 1.0);

  return out; 
}
