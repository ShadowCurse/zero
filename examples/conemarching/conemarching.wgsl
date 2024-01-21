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
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
  let world_position = vertex.world_position.xyz;

  let ray_origin = camera.position;
  let ray_direction = normalize(world_position - ray_origin);

  let t_min: f32 = textureSample(t_depth, s_depth, vertex.tex_coords).r;
  // -1.0 means there are no objects in a way
  // we can skip the calculations
  if (t_min == -1.0) {
    return vec4<f32>(0.0);
  }

  let t_max: f32 = 20.0;

  var t: f32 = t_min;
  let steps: i32 = 100;
  var material_color = vec3<f32>(0.0);

  for (var i: i32 = 0; i < steps && t < t_max; i = i + 1) {
      let point = ray_origin + ray_direction * t;
      let result = sdf(point);
      t += result.a;
      if (abs(result.a) < (0.001 * t)) {
        material_color = result.rgb;
        break;
      }
  }

  var final_color = vec3<f32>(0.0);

  let point = ray_origin + ray_direction * t;
  let normal = normal(point);

  let ao_color = vec3<f32>(0.1, 0.1, 0.1);
  let ao = ambient_occlusion(point, normal);

  var ao_diff = sqrt(clamp(0.5 + 0.5 * normal.y, 0.0, 1.0 ));
  ao_diff *= ao;

  let light_direction = normalize(vec3<f32>(-0.5, 1.0, -1.3));
  let light_size = 0.5;
  let light_color = vec3<f32>(1.5, 1.0, 0.7);
  var light_diff = clamp(dot(normal, light_direction), 0.0, 1.0);

  let shadow = soft_shadow(point, light_direction, 0.02, 0.25, light_size);
  light_diff *= shadow;

  let reflection = reflect(ray_direction, normal);
  let reflectivness = 0.9;
  let reflection_color = vec3<f32>(1.0, 1.0, 1.0);
  var reflection_spe = smoothstep(-0.2, 0.2, reflection.y);
  reflection_spe *= ao_diff;
  reflection_spe *= 0.04 + 0.96 * pow(clamp(1.0 + dot(normal, ray_direction), 0.0, 1.0), 5.0);
  reflection_spe *= soft_shadow(point, reflection, 0.02, 2.5, light_size);

  // shadow
  final_color += material_color * 2.2 * light_diff * light_color;

  // ao
  final_color += material_color * 0.6 * ao_diff * ao_color;

  // reflection
  final_color += material_color * 2.2 * reflection_spe * reflection_color * reflectivness;

  final_color = clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0));

  return vec4<f32>(final_color, 1.0); 
}

fn normal(point: vec3<f32>) -> vec3<f32> {
    let delta = vec2<f32>(0.0001, 0.0);
    return normalize(
              vec3<f32>(
                sdf(point + delta.xyy).a - sdf(point - delta.xyy).a,
                sdf(point + delta.yxy).a - sdf(point - delta.yxy).a,
                sdf(point + delta.yyx).a - sdf(point - delta.yyx).a,
              )
          );
}

fn ambient_occlusion(point: vec3<f32>, normal: vec3<f32>) -> f32 {
    var occ: f32 = 0.0;
    var sca: f32 = 1.0;
    for (var i: i32 = 0; i < 5; i = i + 1) {
        let h = 0.01 + 0.01 * f32(i);
        let distance = sdf(point + normal * h).a;
        occ += (h - distance) * sca;
        sca *= 0.95;
        if (occ > 0.35) {
          break;
        }
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0) * (0.5 + 0.5 * normal.y);
}

fn soft_shadow(ray_origin: vec3<f32>, ray_direction: vec3<f32>, t_min: f32, t_max: f32, light_size: f32) -> f32 {
    var result = 1.0;
    var t = t_min;
    for (var i: i32 = 0; i < 24 && t < t_max; i = i + 1) {
        let distance = sdf(ray_origin + ray_direction * t).a;
        let s = clamp(distance / (t * light_size), 0.0, 1.0);
        result = min(s, result);
        t += clamp(distance, 0.01, 0.2);
        if (result < 0.005) {
          break;
        }
    }
    result = clamp(result, 0.0, 1.0);
    return result * result * (3.0 - 2.0 * result);
}

fn closest(object1: vec4<f32>, object2: vec4<f32>) -> vec4<f32> {
    if (object1.a < object2.a) {
        return object1;
    } else {
        return object2;
    }
}

fn sdf(point: vec3<f32>) -> vec4<f32> {
    let sphere_pos = vec3<f32>(0.0, 0.0, 5.0) * sin(time.time * 0.25);
    let sphere_radius = 0.6;
    let sphere_color = vec3<f32>(1.0, 0.0, 0.0);
    let sphere_distance = sphere(point - sphere_pos, sphere_radius);
    let sphere = vec4<f32>(sphere_color, sphere_distance);

    let box_pos = vec3<f32>(0.0, 0.0, 2.5);
    let box_dimentions = vec3<f32>(0.6, 0.6, 0.6);
    let box_color = vec3<f32>(0.0, 1.0, 0.0);
    let box_distance = box(point - box_pos, box_dimentions);
    let box = vec4<f32>(box_color, box_distance);

    let torus_pos = vec3<f32>(0.0, 0.0, -2.5);
    let torus_dimenttions = vec2<f32>(0.5, 0.18);
    let torus_color = vec3<f32>(0.79, 0.4, 0.1);
    let torus_distance = torus(point - torus_pos, torus_dimenttions);
    let torus = vec4<f32>(torus_color, torus_distance);

    let box_frame_pos = vec3<f32>(0.0, 0.0, 0.0);
    let box_frame_dimenttions = vec3<f32>(0.6, 0.6, 0.6);
    let box_frame_thickness = 0.08;
    let box_frame_color = vec3<f32>(0.89, 0.44, 0.44);
    let box_frame_distance = box_frame(point - box_frame_pos, box_frame_dimenttions, box_frame_thickness);
    let box_frame = vec4<f32>(box_frame_color, box_frame_distance);

    let plane_level = -0.7;
    let plane_color = vec3<f32>(0.1, 0.1, 0.1);
    let plane_distance = plane(point, plane_level);
    let plane = vec4<f32>(plane_color, plane_distance);

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

fn smooth_union(object1: vec4<f32>, object2: vec4<f32>, k: f32) -> vec4<f32> {
    let h = clamp(0.5 + 0.5 * (object2.a - object1.a) / k, 0.0, 1.0);
    return mix(object2, object1, h) - k * h * (1.0 - h);
}

fn smooth_subtraction(object1: vec4<f32>, object2: vec4<f32>, k: f32) -> vec4<f32> {
    let h = clamp(0.5 - 0.5 * (object2.a + object1.a) / k, 0.0, 1.0);
    return mix(object2, -object1, h) + k * h * (1.0 - h);
}

fn smooth_intersection(object1: vec4<f32>, object2: vec4<f32>, k: f32) -> vec4<f32> {
    let h = clamp(0.5 - 0.5 * (object2.a - object1.a) / k, 0.0, 1.0);
    return mix(object2, object1, h) - k * h * (1.0 - h);
}
