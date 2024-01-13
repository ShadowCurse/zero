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

struct TimeUniform {
  time: f32,
};
@group(2) @binding(0)
var<uniform> time: TimeUniform;

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
    let sphere_pos = vec3<f32>(0.0, 0.0, 2.0) * sin(time.time);
    let sphere_radius = 1.0;
    let sphere = sphere(point - sphere_pos, sphere_radius);

    let box_pos = vec3<f32>(0.0, -1.0, 0.0);
    let box_dimentions = vec3<f32>(5.0, 0.5, 5.0);
    let box = box(point - box_pos, box_dimentions);

    return smooth_union(sphere, box, 0.5);
}

fn sphere(point: vec3<f32>, radius: f32) -> f32 {
    return length(point) - radius;
}

fn box(point: vec3<f32>, dimentioins: vec3<f32>) -> f32 {
  let q = abs(point) - dimentioins;
  return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}


fn smooth_union(distance_1: f32, distance_2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (distance_2 - distance_1) / k, 0.0, 1.0);
    return mix(distance_2, distance_1, h) - k * h * (1.0 - h);
}
