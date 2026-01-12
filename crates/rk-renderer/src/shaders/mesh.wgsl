// Mesh shader with Phong lighting and shadow mapping

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
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
};

struct LightUniform {
    light_view_proj: mat4x4<f32>,
    direction: vec4<f32>,      // xyz = direction (toward light), w = unused
    color_intensity: vec4<f32>, // rgb = color, a = intensity
    ambient: vec4<f32>,         // rgb = color, a = strength
    shadow_params: vec4<f32>,   // x = bias, y = normal_bias, z = softness, w = enabled
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> instance: InstanceUniform;

@group(2) @binding(0)
var<uniform> light: LightUniform;

@group(2) @binding(1)
var shadow_map: texture_depth_2d;

@group(2) @binding(2)
var shadow_sampler: sampler_comparison;

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
    @location(3) light_space_pos: vec4<f32>,
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

    // Transform position to light space for shadow mapping
    out.light_space_pos = light.light_view_proj * world_pos;

    return out;
}

// Calculate shadow factor using PCF (Percentage Closer Filtering)
fn calculate_shadow(light_space_pos: vec4<f32>, normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    // Check if shadows are enabled
    if (light.shadow_params.w < 0.5) {
        return 1.0;
    }

    // Perspective divide
    let proj_coords = light_space_pos.xyz / light_space_pos.w;

    // Transform from NDC [-1, 1] to texture coordinates [0, 1]
    let shadow_uv = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        -proj_coords.y * 0.5 + 0.5  // Flip Y for texture coordinates
    );

    // Current depth from light's perspective
    let current_depth = proj_coords.z;

    // Outside shadow map bounds - no shadow
    if (shadow_uv.x < 0.0 || shadow_uv.x > 1.0 ||
        shadow_uv.y < 0.0 || shadow_uv.y > 1.0 ||
        current_depth < 0.0 || current_depth > 1.0) {
        return 1.0;
    }

    // Calculate bias based on surface angle to light
    let cos_theta = max(dot(normal, light_dir), 0.0);
    let bias = max(light.shadow_params.x * (1.0 - cos_theta), light.shadow_params.x * 0.5);
    let biased_depth = current_depth - bias;

    // PCF filtering (3x3 kernel for soft shadows)
    let texel_size = 1.0 / 2048.0;  // Shadow map size
    var shadow = 0.0;

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size * light.shadow_params.z;
            shadow += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                shadow_uv + offset,
                biased_depth
            );
        }
    }
    shadow /= 9.0;

    return shadow;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(light.direction.xyz);
    let view_dir = normalize(camera.eye.xyz - in.world_pos);
    let normal = normalize(in.world_normal);

    // Calculate shadow factor
    let shadow = calculate_shadow(in.light_space_pos, normal, light_dir);

    // Ambient lighting (always visible, not affected by shadow)
    let ambient = light.ambient.rgb * light.ambient.a;

    // Diffuse lighting
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = diff * light.color_intensity.rgb * light.color_intensity.a * 0.6;

    // Specular lighting (Blinn-Phong)
    let halfway_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    let specular = spec * light.color_intensity.rgb * 0.3;

    // Combine: ambient is always visible, diffuse and specular are shadowed
    let lighting = ambient + (diffuse + specular) * shadow;

    var color = in.color.rgb * lighting;

    // Selection highlight
    if (instance.selected == 1u) {
        // Add orange tint for selected objects
        color = mix(color, vec3<f32>(1.0, 0.6, 0.2), 0.3);
    }

    return vec4<f32>(color, in.color.a);
}
