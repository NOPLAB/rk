// Marker (sphere) shader

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
};

struct InstanceInput {
    @location(1) position_radius: vec4<f32>,  // xyz = position, w = radius
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let instance_pos = instance.position_radius.xyz;
    let radius = instance.position_radius.w;
    let world_pos = in.position * radius + instance_pos;

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.world_normal = normalize(in.position); // Unit sphere, so position is normal
    out.color = instance.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple Phong lighting
    let light_dir = normalize(vec3<f32>(0.5, 0.5, 1.0));
    let view_dir = normalize(camera.eye.xyz - in.world_pos);
    let normal = normalize(in.world_normal);

    let ambient = 0.3;
    let diff = max(dot(normal, light_dir), 0.0);
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.5;

    let lighting = ambient + diff * 0.5 + spec;
    let color = in.color.rgb * lighting;

    return vec4<f32>(color, in.color.a);
}
