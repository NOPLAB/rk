// Plane selector shader for reference plane visualization
// Renders semi-transparent planes with highlight on hover

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct PlaneUniform {
    // Which plane is highlighted (0 = none, 1 = XY, 2 = XZ, 3 = YZ)
    highlighted_plane: u32,
    // Plane size
    plane_size: f32,
    // Padding
    _padding: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> plane_data: PlaneUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) plane_id: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) @interpolate(flat) plane_id: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = vec4<f32>(in.position * plane_data.plane_size, 1.0);

    out.clip_position = camera.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = in.normal;
    out.color = in.color;
    out.plane_id = in.plane_id;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(camera.eye.xyz - in.world_pos);
    let normal = normalize(in.world_normal);

    // Make normals face the camera for double-sided rendering
    let facing_normal = select(-normal, normal, dot(normal, view_dir) > 0.0);

    // Simple lighting
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.6));
    let ambient = 0.5;
    let diff = max(dot(facing_normal, light_dir), 0.0) * 0.3;

    // Fresnel effect for edge visibility
    let fresnel = pow(1.0 - abs(dot(facing_normal, view_dir)), 2.0) * 0.2;

    let lighting = ambient + diff + fresnel;
    var color = in.color.rgb * lighting;

    // Alpha: increase if this plane is highlighted
    var alpha = in.color.a;
    if (in.plane_id == plane_data.highlighted_plane) {
        alpha = 0.5;
        // Brighten highlighted plane
        color = color * 1.3;
    }

    return vec4<f32>(color, alpha);
}
