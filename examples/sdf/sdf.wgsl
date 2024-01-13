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

struct ScreenInfoUniform {
  size: vec2<f32>,
};
@group(2) @binding(0)
var<uniform> screen_info: ScreenInfoUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) tangent: vec3<f32>,
  @location(4) bitangent: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
  vertex: VertexInput,
) -> VertexOutput {
  let world_position = transform.transform * vec4<f32>(vertex.position, 1.0);

  var out: VertexOutput;
  out.clip_position = camera.view_projection * world_position;
  return out;
}

// Fragment shader
@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
  let uv = (vertex.clip_position.xy * 2.0 - screen_info.size) / screen_info.size.y;

  let ro = vec3<f32>(0.0, 0.0, -3.0);
  let rd = normalize(vec3<f32>(uv, 1.0));
  var col = vec3<f32>(0.0);

  var t: f32 = 0.0;

  for (var i: i32 = 0; i < 80; i = i + 1) {
      let p = ro + rd * t;
      let d = sdf(p);
      t += d;
      if (d < 0.001 || t > 100.0) {
        break;
      }
  }

  col = vec3<f32>(t * 0.2);

  return vec4<f32>(col, 1.0); 
}

fn sdf(point: vec3<f32>) -> f32 {
    let sphere_pos = vec3<f32>(0.0, 0.0, 1.0);
    let sphere_radius = 1.0;
    let sphere = sphere(point -sphere_pos, sphere_radius);

    return sphere;
}

fn sphere(point: vec3<f32>, radius: f32) -> f32 {
    return length(point) - radius;
}

