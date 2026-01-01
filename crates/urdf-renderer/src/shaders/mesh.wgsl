// Mesh shader with Phong lighting

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
};

struct InstanceUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
    _padding: vec3<u32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> instance: InstanceUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = instance.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_pos = world_pos.xyz;

    // Transform normal (use inverse transpose for non-uniform scaling)
    let normal_matrix = mat3x3<f32>(
        instance.model[0].xyz,
        instance.model[1].xyz,
        instance.model[2].xyz,
    );
    out.world_normal = normalize(normal_matrix * in.normal);

    // Use instance color if set, otherwise vertex color
    out.color = instance.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple Phong lighting
    let light_dir = normalize(vec3<f32>(0.5, 0.5, 1.0));
    let view_dir = normalize(camera.eye.xyz - in.world_pos);
    let normal = normalize(in.world_normal);

    // Ambient
    let ambient = 0.3;

    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);

    // Specular
    let reflect_dir = reflect(-light_dir, normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.3;

    let lighting = ambient + diff * 0.6 + spec;

    var color = in.color.rgb * lighting;

    // Selection highlight
    if (instance.selected == 1u) {
        // Add orange tint for selected objects
        color = mix(color, vec3<f32>(1.0, 0.6, 0.2), 0.3);
    }

    return vec4<f32>(color, in.color.a);
}
