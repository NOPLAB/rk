//! Lighting and shadow system for the renderer.

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::config::{LightingConfig, ShadowConfig};
use crate::constants::shadow::SHADOW_MAP_SIZE;
use crate::light::DirectionalLight;
use crate::sub_renderers::MeshRenderer;

use super::gpu_resources;

/// Manages lighting and shadow resources.
pub struct LightingSystem {
    /// The directional light.
    light: DirectionalLight,
    /// GPU buffer for light uniforms.
    light_buffer: wgpu::Buffer,
    /// Shadow map texture.
    #[allow(dead_code)] // Held for GPU resource lifetime
    shadow_texture: wgpu::Texture,
    /// Shadow map texture view.
    shadow_view: wgpu::TextureView,
    /// Shadow map sampler.
    #[allow(dead_code)] // Held for GPU resource lifetime
    shadow_sampler: wgpu::Sampler,
    /// Bind group for main pass (light uniform + shadow map + sampler).
    light_bind_group: wgpu::BindGroup,
    /// Bind group for shadow pass (light uniform only).
    shadow_light_bind_group: wgpu::BindGroup,
    /// Shadow map size.
    shadow_map_size: u32,
}

impl LightingSystem {
    /// Create a new lighting system.
    pub fn new(device: &wgpu::Device, mesh_renderer: &MeshRenderer) -> Self {
        let light = DirectionalLight::new();
        let light_uniform = light.uniform(Vec3::ZERO);
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (shadow_texture, shadow_view) = gpu_resources::create_shadow_texture(device, SHADOW_MAP_SIZE);
        let shadow_sampler = gpu_resources::create_shadow_sampler(device);

        let light_bind_group = gpu_resources::create_light_bind_group(
            device,
            mesh_renderer.light_bind_group_layout(),
            &light_buffer,
            &shadow_view,
            &shadow_sampler,
        );

        let shadow_light_bind_group_layout =
            gpu_resources::create_shadow_light_bind_group_layout(device);
        let shadow_light_bind_group = gpu_resources::create_shadow_light_bind_group(
            device,
            &shadow_light_bind_group_layout,
            &light_buffer,
        );

        Self {
            light,
            light_buffer,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            light_bind_group,
            shadow_light_bind_group,
            shadow_map_size: SHADOW_MAP_SIZE,
        }
    }

    /// Get a reference to the directional light.
    pub fn light(&self) -> &DirectionalLight {
        &self.light
    }

    /// Get a mutable reference to the directional light.
    pub fn light_mut(&mut self) -> &mut DirectionalLight {
        &mut self.light
    }

    /// Get the light bind group (for main pass).
    pub fn light_bind_group(&self) -> &wgpu::BindGroup {
        &self.light_bind_group
    }

    /// Get the shadow light bind group (for shadow pass).
    pub fn shadow_light_bind_group(&self) -> &wgpu::BindGroup {
        &self.shadow_light_bind_group
    }

    /// Get the shadow view.
    pub fn shadow_view(&self) -> &wgpu::TextureView {
        &self.shadow_view
    }

    /// Get the shadow map size.
    pub fn shadow_map_size(&self) -> u32 {
        self.shadow_map_size
    }

    /// Set light direction.
    pub fn set_direction(&mut self, direction: Vec3) {
        self.light.set_direction(direction);
    }

    /// Set light color and intensity.
    pub fn set_color(&mut self, color: Vec3, intensity: f32) {
        self.light.color = color;
        self.light.intensity = intensity;
    }

    /// Set ambient lighting.
    pub fn set_ambient(&mut self, color: Vec3, strength: f32) {
        self.light.ambient_color = color;
        self.light.ambient_strength = strength;
    }

    /// Enable or disable shadows.
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.light.shadows_enabled = enabled;
    }

    /// Check if shadows are enabled.
    pub fn shadows_enabled(&self) -> bool {
        self.light.shadows_enabled
    }

    /// Update the light buffer on the GPU.
    pub fn update(&self, queue: &wgpu::Queue, scene_center: Vec3) {
        let light_uniform = self.light.uniform(scene_center);
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[light_uniform]));
    }

    /// Apply lighting configuration.
    pub fn apply_lighting_config(&mut self, config: &LightingConfig) {
        self.light.set_direction(Vec3::from_array(config.direction));
        self.light.color = Vec3::from_array(config.color);
        self.light.intensity = config.intensity;
        self.light.ambient_color = Vec3::from_array(config.ambient_color);
        self.light.ambient_strength = config.ambient_strength;
    }

    /// Apply shadow configuration.
    pub fn apply_shadow_config(
        &mut self,
        config: &ShadowConfig,
        device: &wgpu::Device,
        mesh_renderer: &MeshRenderer,
    ) {
        self.light.shadows_enabled = config.enabled;
        self.light.shadow_bias = config.bias;
        self.light.shadow_normal_bias = config.normal_bias;
        self.light.shadow_softness = config.softness;

        // Resize shadow map if size changed
        if config.map_size != self.shadow_map_size {
            self.resize_shadow_map(device, mesh_renderer, config.map_size);
        }
    }

    /// Resize shadow map texture.
    fn resize_shadow_map(
        &mut self,
        device: &wgpu::Device,
        mesh_renderer: &MeshRenderer,
        size: u32,
    ) {
        let size = size.clamp(256, 8192);
        self.shadow_map_size = size;

        let (shadow_texture, shadow_view) = gpu_resources::create_shadow_texture(device, size);
        self.shadow_texture = shadow_texture;
        self.shadow_view = shadow_view;

        // Recreate light bind group with new shadow view
        self.light_bind_group = gpu_resources::create_light_bind_group(
            device,
            mesh_renderer.light_bind_group_layout(),
            &self.light_buffer,
            &self.shadow_view,
            &self.shadow_sampler,
        );
    }
}
