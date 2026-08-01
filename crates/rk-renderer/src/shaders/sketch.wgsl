// Sketch shader - for rendering 2D sketch elements on a plane

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Sketch plane transform (sketch space to world space)
struct SketchUniform {
    transform: mat4x4<f32>,
    plane_color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> sketch: SketchUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) flags: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) @interpolate(flat) flags: u32,
    @location(2) world_pos: vec3<f32>,
};

// Flag bits
const FLAG_SELECTED: u32 = 1u;
const FLAG_HOVERED: u32 = 2u;
const FLAG_CONSTRUCTION: u32 = 4u;
const FLAG_CONSTRAINED: u32 = 8u;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Transform from sketch space to world space
    let world_pos = (sketch.transform * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = in.color;
    out.flags = in.flags;
    out.world_pos = world_pos;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

    // Apply state-based color modifications
    if ((in.flags & FLAG_SELECTED) != 0u) {
        // Selected: bright highlight
        color = vec4<f32>(1.0, 0.6, 0.0, 1.0);
    } else if ((in.flags & FLAG_HOVERED) != 0u) {
        // Hovered: slight highlight
        color = mix(color, vec4<f32>(1.0, 1.0, 0.0, 1.0), 0.3);
    }

    // Construction geometry: more transparent
    if ((in.flags & FLAG_CONSTRUCTION) != 0u) {
        color.a *= 0.5;
    }

    // Constrained geometry: fully saturated
    if ((in.flags & FLAG_CONSTRAINED) != 0u) {
        color = vec4<f32>(0.0, 0.8, 0.0, color.a);
    }

    // Distance-based fade
    let dist = length(in.world_pos - camera.eye.xyz);
    let fade = 1.0 - smoothstep(20.0, 50.0, dist);
    color.a *= fade;

    return color;
}

// Point rendering (separate entry points for point primitives)
@vertex
fn vs_point(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = (sketch.transform * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = in.color;
    out.flags = in.flags;
    out.world_pos = world_pos;

    return out;
}

@fragment
fn fs_point(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

    // Points are always more visible
    if ((in.flags & FLAG_SELECTED) != 0u) {
        color = vec4<f32>(1.0, 0.4, 0.0, 1.0);
    } else if ((in.flags & FLAG_HOVERED) != 0u) {
        color = vec4<f32>(1.0, 0.8, 0.0, 1.0);
    }

    return color;
}
