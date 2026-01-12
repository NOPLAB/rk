// Shadow map generation shader (depth-only pass)

struct LightUniform {
    light_view_proj: mat4x4<f32>,
    direction: vec4<f32>,
    color_intensity: vec4<f32>,
    ambient: vec4<f32>,
    shadow_params: vec4<f32>,
};

struct InstanceUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
};

@group(0) @binding(0)
var<uniform> light: LightUniform;

@group(1) @binding(0)
var<uniform> instance: InstanceUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    let world_pos = instance.model * vec4<f32>(in.position, 1.0);
    return light.light_view_proj * world_pos;
}

// Fragment shader is minimal for depth-only pass
// Some platforms require a fragment shader even for depth-only rendering
@fragment
fn fs_main() {
    // Depth is written automatically
}
