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

  let ray_origin = vec4<f32>(camera.position, 1.0);
  let ray_direction = normalize(world_position - ray_origin);

  let t_min: f32 = 1.0;
  let t_max: f32 = 20.0;

  var t: f32 = t_min;
  var material_color = vec3<f32>(0.0);

  for (var i: i32 = 0; i < 100 && t < t_max; i = i + 1) {
      let point = ray_origin + ray_direction * t;
      let result = sdf(point.xyz);
      t += result.a;
      if (abs(result.a) < (0.001 * t)) {
        material_color = result.rgb;
        break;
      }
  }

  var final_color = vec3<f32>(0.0);

  let point = ray_origin + ray_direction * t;
  let normal = normal(point.xyz);

  let ao_color = vec3<f32>(0.1, 0.1, 0.1);
  let ao = ambient_occlusion(point.xyz, normal);

  var ao_diff = sqrt(clamp(0.5 + 0.5 * normal.y, 0.0, 1.0 ));
  ao_diff *= ao;

  let light_direction = normalize(vec3<f32>(-0.5, 1.0, -1.3));
  let shadow = soft_shadow(point.xyz, light_direction, 0.02, 0.25);

  let light_color = vec3<f32>(1.5, 1.0, 0.7);
  var light_diff = clamp(dot(normal, light_direction), 0.0, 1.0);
  light_diff *= shadow;

  let reflection = reflect(ray_direction.xyz, normal);
  let reflectivness = 0.9;
  let reflection_color = vec3<f32>(1.0, 1.0, 1.0);
  var reflection_spe = smoothstep(-0.2, 0.2, reflection.y);
  reflection_spe *= ao_diff;
  reflection_spe *= 0.04 + 0.96 * pow(clamp(1.0 + dot(normal, ray_direction.xyz), 0.0, 1.0), 5.0);
  reflection_spe *= soft_shadow(point.xyz, reflection, 0.02, 2.5);

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

fn soft_shadow(ray_origin: vec3<f32>, ray_direction: vec3<f32>, t_min: f32, t_max: f32) -> f32 {
    var result = 1.0;
    var t = t_min;
    for (var i: i32 = 0; i < 24 && t < t_max; i = i + 1) {
        let distance = sdf(ray_origin + ray_direction * t).a;
        let s = clamp(8.0 * distance / t, 0.0, 1.0);
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
    let sphere_pos = vec3<f32>(0.0, 0.0, 5.0) * sin(time.time);
    let sphere_radius = 1.0;
    let sphere_color = vec3<f32>(1.0, 0.0, 0.0);
    let sphere_distance = sphere(point - sphere_pos, sphere_radius);
    let sphere = vec4<f32>(sphere_color, sphere_distance);

    let box_pos = vec3<f32>(0.0, 0.0, 2.0);
    let box_dimentions = vec3<f32>(1.0, 1.0, 1.0);
    let box_color = vec3<f32>(0.0, 1.0, 0.0);
    let box_distance = box(point - box_pos, box_dimentions);
    let box = vec4<f32>(box_color, box_distance);

    let torus_pos = vec3<f32>(0.0, 0.0, -2.0);
    let torus_dimenttions = vec2<f32>(0.7, 0.3);
    let torus_color = vec3<f32>(0.79, 0.4, 0.1);
    let torus_distance = torus(point - torus_pos, torus_dimenttions);
    let torus = vec4<f32>(torus_color, torus_distance);

    let plane_level = -1.0;
    let plane_color = vec3<f32>(0.1, 0.1, 0.1);
    let plane_distance = plane(point, plane_level);
    let plane = vec4<f32>(plane_color, plane_distance);

    let smooth_box_sphere = smooth_union(sphere, box, 0.5);
    let smooth_torus_sphere = smooth_subtraction(sphere, torus, 0.5);
    return closest(plane, closest(smooth_box_sphere, smooth_torus_sphere));
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

fn smooth_union(object1: vec4<f32>, object2: vec4<f32>, k: f32) -> vec4<f32> {
    let h = clamp(0.5 + 0.5 * (object2.a - object1.a) / k, 0.0, 1.0);
    return mix(object2, object1, h) - k * h * (1.0 - h);
}

fn smooth_subtraction(object1: vec4<f32>, object2: vec4<f32>, k: f32) -> vec4<f32> {
    let h = clamp(0.5 - 0.5 * (object2.a + object1.a) / k, 0.0, 1.0);
    return mix(object2, -object1, h) + k * h * (1.0 - h);
}
