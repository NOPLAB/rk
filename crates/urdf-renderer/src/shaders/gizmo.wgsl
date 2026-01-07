// Transform gizmo shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) axis_id: u32, // 0=X, 1=Y, 2=Z
};

struct InstanceInput {
    @location(3) transform_0: vec4<f32>,
    @location(4) transform_1: vec4<f32>,
    @location(5) transform_2: vec4<f32>,
    @location(6) transform_3: vec4<f32>,
    @location(7) scale_highlight: vec4<f32>, // x=scale, y=highlighted_axis (-1=none, 0=X, 1=Y, 2=Z)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let transform = mat4x4<f32>(
        instance.transform_0,
        instance.transform_1,
        instance.transform_2,
        instance.transform_3,
    );
    let base_scale = instance.scale_highlight.x;
    let highlighted_axis = i32(instance.scale_highlight.y);

    // Get gizmo world position from transform matrix
    let gizmo_world_pos = transform[3].xyz;

    // Calculate distance from camera to gizmo
    let camera_distance = length(camera.eye.xyz - gizmo_world_pos);

    // Scale gizmo based on camera distance to maintain constant screen size
    let distance_scale = camera_distance * 0.15;
    let scale = base_scale * distance_scale;

    let scaled_pos = in.position * scale;
    let world_pos = transform * vec4<f32>(scaled_pos, 1.0);

    out.clip_position = camera.view_proj * world_pos;

    // Brighten if this axis is highlighted
    var color = in.color;
    if (highlighted_axis == i32(in.axis_id)) {
        color = vec4<f32>(1.0, 1.0, 0.5, 1.0); // Yellow highlight
    }
    out.color = color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
