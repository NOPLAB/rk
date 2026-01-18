// Collision shape visualization shader
// Renders collision geometry with semi-transparent shading

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
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
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

    let model = mat4x4<f32>(
        instance.model_0,
        instance.model_1,
        instance.model_2,
        instance.model_3,
    );

    let world_pos = model * vec4<f32>(in.position, 1.0);

    // Transform normal (ignoring translation, using upper 3x3)
    let normal_matrix = mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz,
    );
    let world_normal = normalize(normal_matrix * in.normal);

    out.clip_position = camera.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = world_normal;
    out.color = instance.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting for depth perception
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.6));
    let view_dir = normalize(camera.eye.xyz - in.world_pos);
    let normal = normalize(in.world_normal);

    // Make normals face the camera for double-sided rendering
    let facing_normal = select(-normal, normal, dot(normal, view_dir) > 0.0);

    let ambient = 0.4;
    let diff = max(dot(facing_normal, light_dir), 0.0) * 0.5;

    // Fresnel effect for better edge visibility
    let fresnel = pow(1.0 - abs(dot(facing_normal, view_dir)), 2.0) * 0.3;

    let lighting = ambient + diff + fresnel;
    let color = in.color.rgb * lighting;

    return vec4<f32>(color, in.color.a);
}
