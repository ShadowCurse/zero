// Shader is from bevy_gizmos crate: https://github.com/bevyengine/bevy/blob/main/crates/bevy_gizmos/src/lines.wgsl
// Vertex shader

struct CameraUniform {
  view: mat4x4<f32>,
  projection: mat4x4<f32>,
  view_projection: mat4x4<f32>,
  view_projection_inverse: mat4x4<f32>,
  view_projection_without_translation: mat4x4<f32>,
  position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LineGizmoUniform {
    line_width: f32,
    depth_bias: f32,
}

struct ScreenUniform {
    width: f32,
    height: f32,
}

@group(1) @binding(0) var<uniform> screen: ScreenUniform;

struct VertexInput {
    @location(0) position_a: vec3<f32>,
    @location(1) position_b: vec3<f32>,
    @location(2) color_a: vec4<f32>,
    @location(3) color_b: vec4<f32>,
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

const EPSILON: f32 = 4.88e-04;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
  var line_gizmo = LineGizmoUniform(1.0, 0.0);

  var positions = array<vec3<f32>, 6>(
        vec3(0.0, -0.5, 0.),
        vec3(0.0, -0.5, 1.),
        vec3(0.0, 0.5, 1.),
        vec3(0.0, -0.5, 0.),
        vec3(0.0, 0.5, 1.),
        vec3(0.0, 0.5, 0.)
    );
    let position = positions[vertex.index];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    var clip_a = camera.view_projection * vec4(vertex.position_a, 1.);
    var clip_b = camera.view_projection * vec4(vertex.position_b, 1.);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip_a = clip_near_plane(clip_a, clip_b);
    clip_b = clip_near_plane(clip_b, clip_a);

    let clip = mix(clip_a, clip_b, position.z);

    let resolution = vec2<f32>(screen.width, screen.height);
    let screen_a = resolution * (0.5 * clip_a.xy / clip_a.w + 0.5);
    let screen_b = resolution * (0.5 * clip_b.xy / clip_b.w + 0.5);

    let x_basis = normalize(screen_a - screen_b);
    let y_basis = vec2(-x_basis.y, x_basis.x);

    var color = mix(vertex.color_a, vertex.color_b, position.z);

    var line_width = line_gizmo.line_width;
    var alpha = 1.0;

    line_width /= clip.w;

    // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
    if line_width > 0.0 && line_width < 1. {
        color.a *= line_width;
        line_width = 1.;
    }

    let offset = line_width * (position.x * x_basis + position.y * y_basis);
    let screen = mix(screen_a, screen_b, position.z) + offset;

    var depth: f32;
    if line_gizmo.depth_bias >= 0. {
        depth = clip.z * (1. - line_gizmo.depth_bias);
    } else {
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0
        // clip.w represents the near plane in homogeneous clip space, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convenient for them
        depth = clip.z * exp2(-line_gizmo.depth_bias * log2(clip.w / clip.z - EPSILON));
    }

    var clip_position = vec4(clip.w * ((2.0 * screen) / resolution - 1.0), depth, clip.w);

    return VertexOutput(clip_position, color);
}

fn clip_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    // Move a if a is behind the near plane and b is in front. 
    if a.z > a.w && b.z <= b.w {
        // Interpolate a towards b until it's at the near plane.
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        // Add an epsilon to the interpolator to ensure that the point is
        // not just behind the clip plane due to floating-point imprecision.
        let t = distance_a / (distance_a - distance_b) + EPSILON;
        return mix(a, b, t);
    }
    return a;
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fs_main(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.color);
}
