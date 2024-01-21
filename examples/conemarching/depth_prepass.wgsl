// Vertex shader

struct CameraUniform {
  position: vec3<f32>,
  view_projection: mat4x4<f32>,
  vp_without_translation: mat4x4<f32>,
  vp_inverse: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct TimeUniform {
  time: f32,
};
@group(1) @binding(0)
var<uniform> time: TimeUniform;

@group(2) @binding(0)
var t_depth: texture_2d<f32>;
@group(2) @binding(1)
var s_depth: sampler;

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
  @location(1) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  var p = vec4<f32>(vertex.position.xy, 1.0, 1.0);

  var out: VertexOutput;
  out.clip_position = p;

  let world_pos = camera.vp_inverse * p;
  out.world_position = world_pos / world_pos.w;
  out.tex_coords = vertex.tex_coords;

  return out;
}

// Fragment shader
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) f32 {
  let world_position = vertex.world_position.xyz;

  let ray_origin = camera.position;
  let ray_direction = normalize(world_position - ray_origin);

  let t_min: f32 = textureSample(t_depth, s_depth, vertex.tex_coords).r;
  // -1.0 means there are no objects in a way
  // we can skip the calculations
  if (t_min == -1.0) {
    return -1.0;
  }

  let t_max: f32 = 20.0;

  var t: f32 = t_min;
  let steps: i32 = 100;

  let texture_size: vec2<u32> = textureDimensions(t_depth);
  let angle_deg = 90.0 / f32(texture_size.x);
  let angle_rad = radians(angle_deg);
  let tan_angle = tan(angle_rad);

  for (var i: i32 = 0; i < steps && t < t_max; i = i + 1) {
      let point = ray_origin + ray_direction * t;
      let result = sdf(point);
      let half_cone_size = t * tan_angle;
      if (result < half_cone_size) {
        return t;
      }
      t += result;
  }

  return -1.0;
}

fn closest(object1: f32, object2: f32) -> f32 {
    if (object1 < object2) {
        return object1;
    } else {
        return object2;
    }
}

fn sdf(point: vec3<f32>) -> f32 {
    let sphere_pos = vec3<f32>(0.0, 0.0, 5.0) * sin(time.time * 0.25);
    let sphere_radius = 0.6;
    let sphere = sphere(point - sphere_pos, sphere_radius);

    let box_pos = vec3<f32>(0.0, 0.0, 2.5);
    let box_dimentions = vec3<f32>(0.6, 0.6, 0.6);
    let box = box(point - box_pos, box_dimentions);

    let torus_pos = vec3<f32>(0.0, 0.0, -2.5);
    let torus_dimenttions = vec2<f32>(0.5, 0.18);
    let torus = torus(point - torus_pos, torus_dimenttions);

    let box_frame_pos = vec3<f32>(0.0, 0.0, 0.0);
    let box_frame_dimenttions = vec3<f32>(0.6, 0.6, 0.6);
    let box_frame_thickness = 0.08;
    let box_frame = box_frame(point - box_frame_pos, box_frame_dimenttions, box_frame_thickness);

    let plane_level = -0.7;
    let plane = plane(point, plane_level);

    let smooth_box_sphere = smooth_union(sphere, box, 0.5);
    let smooth_torus_sphere = smooth_subtraction(sphere, torus, 0.5);
    let smooth_box_frame_sphere = smooth_intersection(sphere, box_frame, 0.5);

    return closest(plane, closest(smooth_box_sphere, closest(smooth_torus_sphere, smooth_box_frame_sphere)));
}

fn plane(point: vec3<f32>, level: f32) -> f32 {
    return point.y - level;
}

fn sphere(point: vec3<f32>, radius: f32) -> f32 {
    return length(point) - radius;
}

fn box(point: vec3<f32>, dimentioins: vec3<f32>) -> f32 {
  let q = abs(point) - dimentioins;
  return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn torus(point: vec3<f32>, dimentioins: vec2<f32>) -> f32 {
  let q = vec2<f32>(length(point.zy) - dimentioins.x, point.x);
  return length(q) - dimentioins.y;
}

fn box_frame(point: vec3<f32>, dimentioins: vec3<f32>, thickness: f32) -> f32 {
  let p = abs(point) - dimentioins;
  let q = abs(p + thickness) - thickness;
  return min(min(
      length(max(vec3<f32>(p.x, q.y, q.z), vec3<f32>(0.0))) + min(max(p.x, max(q.y, q.z)), 0.0),
      length(max(vec3<f32>(q.x, p.y, q.z), vec3<f32>(0.0))) + min(max(q.x, max(p.y, q.z)), 0.0)),
      length(max(vec3<f32>(q.x, q.y, p.z), vec3<f32>(0.0))) + min(max(q.x, max(q.y, p.z)), 0.0));
}

fn smooth_union(object1: f32, object2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (object2 - object1) / k, 0.0, 1.0);
    return mix(object2, object1, h) - k * h * (1.0 - h);
}

fn smooth_subtraction(object1: f32, object2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (object2 + object1) / k, 0.0, 1.0);
    return mix(object2, -object1, h) + k * h * (1.0 - h);
}

fn smooth_intersection(object1: f32, object2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (object2 - object1) / k, 0.0, 1.0);
    return mix(object2, object1, h) - k * h * (1.0 - h);
}
