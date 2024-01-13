// Vertex shader

struct TransformUniform {
  transform: mat4x4<f32>,
  rotate: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> transform: TransformUniform;

struct CameraUniform {
  position: vec3<f32>,
  view_projection: mat4x4<f32>,
  vp_without_translation: mat4x4<f32>,
};
@group(1) @binding(0)
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
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  out.world_position = world_position;
  return out;
}

// Fragment shader
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
  let world_position = vertex.world_position;

  let camera_position = vec4<f32>(camera.position, 1.0);
  let ray_direction = normalize(world_position - camera_position);
  var color = vec3<f32>(0.0);

  var t: f32 = 0.0;

  for (var i: i32 = 0; i < 100; i = i + 1) {
      let point = camera_position + ray_direction * t;
      let distance = sdf(point.xyz);
      t += distance;
      if (distance < 0.001 || t > 100.0) {
        break;
      }
  }

  color = vec3<f32>(t * 0.01);

  return vec4<f32>(color, 1.0); 
}

fn sdf(point: vec3<f32>) -> f32 {
    let sphere_pos = vec3<f32>(0.0, 0.0, 0.0);
    let sphere_radius = 1.0;
    let sphere = sphere(point - sphere_pos, sphere_radius);

    return sphere;
}

fn sphere(point: vec3<f32>, radius: f32) -> f32 {
    return length(point) - radius;
}

