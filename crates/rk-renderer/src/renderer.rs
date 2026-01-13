//! Main renderer combining all sub-renderers
//!
//! This module provides the main [`Renderer`] struct that orchestrates all
//! rendering operations. It uses a plugin-based architecture where sub-renderers
//! can be registered via [`RendererRegistry`].
//!
//! # Architecture
//!
//! The renderer uses the following components:
//! - [`RenderContext`]: Encapsulates GPU resources
//! - [`Scene`]: Manages renderable objects
//! - [`MeshManager`]: Handles GPU mesh resources
//! - [`RendererRegistry`]: Manages sub-renderer plugins

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use uuid::Uuid;
use wgpu::util::DeviceExt;

use rk_core::Part;

use crate::camera::Camera;
use crate::config::{
    CameraConfig, GizmoConfig, GridConfig, LightingConfig, RendererConfig, ShadowConfig,
    ViewportConfig,
};
use crate::constants::shadow::{SHADOW_MAP_FORMAT, SHADOW_MAP_SIZE};
use crate::constants::viewport::{CLEAR_COLOR, SAMPLE_COUNT};
use crate::light::DirectionalLight;
use crate::plugin::RendererRegistry;
use crate::resources::MeshManager;
use crate::scene::Scene;
use crate::sub_renderers::{
    AxisInstance, AxisRenderer, GizmoAxis, GizmoMode, GizmoRenderer, GridRenderer, MarkerInstance,
    MarkerRenderer, MeshData, MeshRenderer,
};

/// Mesh entry with bind group
pub struct MeshEntry {
    pub data: MeshData,
    pub bind_group: wgpu::BindGroup,
}

/// Main renderer combining all sub-renderers.
///
/// The renderer provides both a legacy API for backward compatibility
/// and access to new architectural components for advanced use cases.
///
/// # Legacy API
///
/// Methods like [`add_part`], [`update_part_transform`], etc. work the same
/// as before for existing code.
///
/// # New Architecture
///
/// For advanced use cases, access the new components:
/// - [`registry`]/[`registry_mut`]: Access the plugin system
/// - [`scene`]/[`scene_mut`]: Access the scene graph
/// - [`mesh_manager`]/[`mesh_manager_mut`]: Access mesh resources
/// - [`context`]: Access the render context
pub struct Renderer {
    // New architectural components (optional during migration)
    // These will be fully integrated in future versions
    scene: Scene,
    mesh_manager: MeshManager,
    registry: RendererRegistry,

    // Legacy components (kept for backward compatibility)
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    // MSAA color texture (for multisampling)
    msaa_texture: Option<wgpu::Texture>,
    msaa_view: Option<wgpu::TextureView>,

    // Lighting and shadow resources
    light: DirectionalLight,
    light_buffer: wgpu::Buffer,
    #[allow(dead_code)] // Held for GPU resource lifetime
    shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    #[allow(dead_code)] // Held for GPU resource lifetime
    shadow_sampler: wgpu::Sampler,
    /// Bind group for main pass (light uniform + shadow map + sampler)
    light_bind_group: wgpu::BindGroup,
    /// Bind group for shadow pass (light uniform only)
    shadow_light_bind_group: wgpu::BindGroup,

    // Sub-renderers (legacy - will migrate to registry)
    grid_renderer: GridRenderer,
    mesh_renderer: MeshRenderer,
    axis_renderer: AxisRenderer,
    marker_renderer: MarkerRenderer,
    gizmo_renderer: GizmoRenderer,

    // Data - UUID-keyed storage for O(1) lookup and removal
    meshes: HashMap<Uuid, MeshEntry>,
    selected_part: Option<Uuid>,

    // Display options
    show_grid: bool,
    show_axes: bool,
    show_markers: bool,
    show_gizmo: bool,

    // Configurable rendering settings
    clear_color: wgpu::Color,
    shadow_map_size: u32,

    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let depth_format = wgpu::TextureFormat::Depth32Float;

        let camera = Camera::new(width as f32 / height as f32);
        let camera_uniform = camera.uniform();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let (depth_texture, depth_view) = Self::create_depth_texture(device, width, height);
        let msaa_result = Self::create_msaa_texture(device, format, width, height);
        let (msaa_texture, msaa_view) = match msaa_result {
            Some((tex, view)) => (Some(tex), Some(view)),
            None => (None, None),
        };

        // Initialize lighting
        let light = DirectionalLight::new();
        let light_uniform = light.uniform(Vec3::ZERO);
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create shadow map texture
        let (shadow_texture, shadow_view) = Self::create_shadow_texture(device, SHADOW_MAP_SIZE);
        let shadow_sampler = Self::create_shadow_sampler(device);

        let grid_renderer = GridRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let mesh_renderer = MeshRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        // Create light bind groups after mesh_renderer is created
        let light_bind_group = Self::create_light_bind_group(
            device,
            mesh_renderer.light_bind_group_layout(),
            &light_buffer,
            &shadow_view,
            &shadow_sampler,
        );

        // Shadow pass bind group (light uniform only, for shadow.wgsl group 0)
        let shadow_light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Light Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shadow_light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Light Bind Group"),
            layout: &shadow_light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let axis_renderer = AxisRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let marker_renderer = MarkerRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let gizmo_renderer = GizmoRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        // Initialize new architectural components
        let scene = Scene::new();
        let mesh_manager = MeshManager::new();
        let registry = RendererRegistry::new();

        Self {
            // New components
            scene,
            mesh_manager,
            registry,

            // Legacy components
            camera,
            camera_buffer,
            camera_bind_group_layout,
            depth_texture,
            depth_view,
            msaa_texture,
            msaa_view,

            // Lighting
            light,
            light_buffer,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            light_bind_group,
            shadow_light_bind_group,

            grid_renderer,
            mesh_renderer,
            axis_renderer,
            marker_renderer,
            gizmo_renderer,
            meshes: HashMap::new(),
            selected_part: None,
            show_grid: true,
            show_axes: true,
            show_markers: true,
            show_gizmo: true,
            clear_color: CLEAR_COLOR,
            shadow_map_size: SHADOW_MAP_SIZE,
            format,
            width,
            height,
        }
    }

    // ========== Camera accessors ==========

    /// Get a reference to the camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    // ========== Light accessors ==========

    /// Get a reference to the directional light.
    pub fn light(&self) -> &DirectionalLight {
        &self.light
    }

    /// Get a mutable reference to the directional light.
    pub fn light_mut(&mut self) -> &mut DirectionalLight {
        &mut self.light
    }

    /// Set light direction (convenience method).
    pub fn set_light_direction(&mut self, direction: Vec3) {
        self.light.set_direction(direction);
    }

    /// Set light color and intensity (convenience method).
    pub fn set_light_color(&mut self, color: Vec3, intensity: f32) {
        self.light.color = color;
        self.light.intensity = intensity;
    }

    /// Set ambient lighting (convenience method).
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

    // ========== Display option accessors ==========

    /// Get whether the grid is visible.
    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    /// Set whether the grid is visible.
    pub fn set_show_grid(&mut self, show: bool) {
        self.show_grid = show;
    }

    /// Get whether axes are visible.
    pub fn show_axes(&self) -> bool {
        self.show_axes
    }

    /// Set whether axes are visible.
    pub fn set_show_axes(&mut self, show: bool) {
        self.show_axes = show;
    }

    /// Get whether markers are visible.
    pub fn show_markers(&self) -> bool {
        self.show_markers
    }

    /// Set whether markers are visible.
    pub fn set_show_markers(&mut self, show: bool) {
        self.show_markers = show;
    }

    /// Get whether the gizmo rendering is enabled.
    pub fn is_gizmo_enabled(&self) -> bool {
        self.show_gizmo
    }

    /// Set whether the gizmo rendering is enabled.
    pub fn set_gizmo_enabled(&mut self, enabled: bool) {
        self.show_gizmo = enabled;
    }

    // ========== Gizmo delegate methods ==========

    /// Get gizmo visibility state.
    pub fn gizmo_visible(&self) -> bool {
        self.gizmo_renderer.visible
    }

    /// Get gizmo highlighted axis.
    pub fn gizmo_highlighted_axis(&self) -> GizmoAxis {
        self.gizmo_renderer.highlighted_axis
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_msaa_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Option<(wgpu::Texture, wgpu::TextureView)> {
        if SAMPLE_COUNT <= 1 {
            return None;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Color Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some((texture, view))
    }

    fn create_shadow_texture(
        device: &wgpu::Device,
        size: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = size.clamp(256, 8192);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Texture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1, // No MSAA for shadow map
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_MAP_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_shadow_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        })
    }

    fn create_light_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        light_buffer: &wgpu::Buffer,
        shadow_view: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(shadow_sampler),
                },
            ],
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.camera.update_aspect(width as f32 / height as f32);
        let (depth_texture, depth_view) = Self::create_depth_texture(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        // Recreate MSAA texture
        let msaa_result = Self::create_msaa_texture(device, self.format, width, height);
        let (msaa_texture, msaa_view) = match msaa_result {
            Some((tex, view)) => (Some(tex), Some(view)),
            None => (None, None),
        };
        self.msaa_texture = msaa_texture;
        self.msaa_view = msaa_view;
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn update_camera(&self, queue: &wgpu::Queue) {
        let camera_uniform = self.camera.uniform();
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    fn update_light(&self, queue: &wgpu::Queue) {
        // Use camera target as scene center for shadow projection
        let scene_center = self.camera.target;
        let light_uniform = self.light.uniform(scene_center);
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[light_uniform]),
        );
    }

    /// Add a part to the renderer.
    ///
    /// Returns the part's UUID for reference.
    pub fn add_part(&mut self, device: &wgpu::Device, part: &Part) -> Uuid {
        tracing::info!("Renderer::add_part called for '{}'", part.name);
        let data = MeshData::from_part(device, part);
        let bind_group = self.mesh_renderer.create_instance_bind_group(device, &data);

        self.meshes.insert(part.id, MeshEntry { data, bind_group });
        tracing::info!("Renderer now has {} meshes", self.meshes.len());
        part.id
    }

    /// Update a part's transform.
    pub fn update_part_transform(&mut self, queue: &wgpu::Queue, part_id: Uuid, transform: Mat4) {
        if let Some(entry) = self.meshes.get_mut(&part_id) {
            entry.data.update_transform(queue, transform);
        }
    }

    /// Update a part's color.
    pub fn update_part_color(&mut self, queue: &wgpu::Queue, part_id: Uuid, color: [f32; 4]) {
        if let Some(entry) = self.meshes.get_mut(&part_id) {
            entry.data.update_color(queue, color);
        }
    }

    /// Set selected part.
    pub fn set_selected_part(&mut self, queue: &wgpu::Queue, part_id: Option<Uuid>) {
        // Deselect previous
        if let Some(prev_id) = self.selected_part
            && let Some(entry) = self.meshes.get_mut(&prev_id)
        {
            entry.data.set_selected(queue, false);
        }

        // Select new
        self.selected_part = part_id;
        if let Some(id) = part_id
            && let Some(entry) = self.meshes.get_mut(&id)
        {
            entry.data.set_selected(queue, true);
        }
    }

    /// Get the currently selected part ID.
    pub fn selected_part(&self) -> Option<Uuid> {
        self.selected_part
    }

    /// Remove a part - O(1) operation with UUID-based storage.
    pub fn remove_part(&mut self, part_id: Uuid) {
        self.meshes.remove(&part_id);
        if self.selected_part == Some(part_id) {
            self.selected_part = None;
        }
    }

    /// Clear all parts.
    pub fn clear_parts(&mut self) {
        self.meshes.clear();
        self.selected_part = None;
    }

    /// Check if a part exists.
    pub fn has_part(&self, part_id: Uuid) -> bool {
        self.meshes.contains_key(&part_id)
    }

    /// Get the number of parts.
    pub fn part_count(&self) -> usize {
        self.meshes.len()
    }

    /// Update axis display
    pub fn update_axes(&mut self, queue: &wgpu::Queue, instances: &[AxisInstance]) {
        self.axis_renderer.update_instances(queue, instances);
    }

    /// Update marker display
    pub fn update_markers(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        self.marker_renderer.update_instances(queue, instances);
    }

    /// Update selected marker display (rendered on top)
    pub fn update_selected_markers(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        self.marker_renderer
            .update_selected_instances(queue, instances);
    }

    /// Show gizmo at position
    pub fn show_gizmo(&mut self, queue: &wgpu::Queue, position: glam::Vec3, scale: f32) {
        self.gizmo_renderer.show(queue, position, scale);
    }

    /// Hide gizmo
    pub fn hide_gizmo(&mut self) {
        self.gizmo_renderer.hide();
    }

    /// Set gizmo highlighted axis
    pub fn set_gizmo_highlight(&mut self, queue: &wgpu::Queue, axis: GizmoAxis) {
        self.gizmo_renderer.set_highlighted(queue, axis);
    }

    /// Hit test gizmo
    pub fn gizmo_hit_test(
        &self,
        ray_origin: glam::Vec3,
        ray_dir: glam::Vec3,
        gizmo_pos: glam::Vec3,
        scale: f32,
    ) -> GizmoAxis {
        // Calculate distance-based scale to match shader behavior
        let camera_distance = (self.camera.position - gizmo_pos).length();
        let distance_scale = camera_distance * 0.15;
        let adjusted_scale = scale * distance_scale;
        self.gizmo_renderer
            .hit_test(ray_origin, ray_dir, gizmo_pos, adjusted_scale)
    }

    /// Set gizmo mode
    pub fn set_gizmo_mode(&mut self, mode: GizmoMode) {
        self.gizmo_renderer.set_mode(mode);
    }

    /// Get current gizmo mode
    pub fn gizmo_mode(&self) -> GizmoMode {
        self.gizmo_renderer.mode
    }

    /// Render the scene.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) {
        self.update_camera(queue);
        self.update_light(queue);

        // === SHADOW PASS ===
        // Render scene from light's perspective to generate shadow map
        if self.light.shadows_enabled && !self.meshes.is_empty() {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            shadow_pass.set_viewport(
                0.0,
                0.0,
                self.shadow_map_size as f32,
                self.shadow_map_size as f32,
                0.0,
                1.0,
            );

            for entry in self.meshes.values() {
                self.mesh_renderer.render_shadow(
                    &mut shadow_pass,
                    &entry.data,
                    &entry.bind_group,
                    &self.shadow_light_bind_group,
                );
            }
        }

        // === MAIN PASS ===
        // Set up color attachment with MSAA if enabled
        let color_attachment = if let Some(msaa_view) = &self.msaa_view {
            // MSAA enabled: render to multisample texture, resolve to output
            wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            }
        } else {
            // MSAA disabled: render directly to output
            wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            }
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render grid
        if self.show_grid {
            self.grid_renderer.render(&mut render_pass);
        }

        // Render meshes with lighting and shadows
        for entry in self.meshes.values() {
            self.mesh_renderer.render(
                &mut render_pass,
                &entry.data,
                &entry.bind_group,
                &self.light_bind_group,
            );
        }

        // Render axes
        if self.show_axes {
            self.axis_renderer.render(&mut render_pass);
        }

        // Render markers
        if self.show_markers {
            self.marker_renderer.render(&mut render_pass);
        }

        // Render gizmo (always on top)
        if self.show_gizmo {
            self.gizmo_renderer.render(&mut render_pass);
        }
    }

    /// Get camera bind group layout for external use
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    // ========== New Architecture Accessors ==========

    /// Get a reference to the scene.
    ///
    /// The scene manages all renderable objects. Use this for advanced
    /// scene manipulation beyond the convenience methods.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Get a mutable reference to the scene.
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Get a reference to the mesh manager.
    ///
    /// The mesh manager handles GPU mesh resources. Use this for
    /// advanced mesh manipulation or to share meshes between objects.
    pub fn mesh_manager(&self) -> &MeshManager {
        &self.mesh_manager
    }

    /// Get a mutable reference to the mesh manager.
    pub fn mesh_manager_mut(&mut self) -> &mut MeshManager {
        &mut self.mesh_manager
    }

    /// Get a reference to the renderer registry.
    ///
    /// The registry manages sub-renderer plugins. Use this to
    /// register custom renderers or modify rendering behavior.
    pub fn registry(&self) -> &RendererRegistry {
        &self.registry
    }

    /// Get a mutable reference to the renderer registry.
    pub fn registry_mut(&mut self) -> &mut RendererRegistry {
        &mut self.registry
    }

    // ========== Configuration Methods ==========

    /// Apply a full renderer configuration.
    ///
    /// This updates all renderer settings from the provided config.
    /// Note: Some settings like MSAA require a restart to take effect.
    pub fn apply_config(
        &mut self,
        config: &RendererConfig,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.apply_grid_config(&config.grid, device);
        self.apply_viewport_config(&config.viewport);
        self.apply_shadow_config(&config.shadow, device);
        self.apply_lighting_config(&config.lighting);
        self.apply_camera_config(&config.camera);
        self.apply_gizmo_config(&config.gizmo, queue);
    }

    /// Apply grid configuration.
    pub fn apply_grid_config(&mut self, config: &GridConfig, device: &wgpu::Device) {
        self.show_grid = config.enabled;
        // Rebuild grid with new parameters
        self.grid_renderer.rebuild(
            device,
            config.size,
            config.spacing,
            config.line_color,
            config.x_axis_color,
            config.y_axis_color,
        );
    }

    /// Apply shadow configuration.
    pub fn apply_shadow_config(&mut self, config: &ShadowConfig, device: &wgpu::Device) {
        self.light.shadows_enabled = config.enabled;
        self.light.shadow_bias = config.bias;
        self.light.shadow_normal_bias = config.normal_bias;
        self.light.shadow_softness = config.softness;

        // Resize shadow map if size changed
        if config.map_size != self.shadow_map_size {
            self.resize_shadow_map(device, config.map_size);
        }
    }

    /// Resize shadow map texture.
    fn resize_shadow_map(&mut self, device: &wgpu::Device, size: u32) {
        let size = size.clamp(256, 8192);
        self.shadow_map_size = size;

        // Recreate shadow texture
        let (shadow_texture, shadow_view) = Self::create_shadow_texture(device, size);
        self.shadow_texture = shadow_texture;
        self.shadow_view = shadow_view;

        // Recreate light bind group with new shadow view
        self.light_bind_group = Self::create_light_bind_group(
            device,
            self.mesh_renderer.light_bind_group_layout(),
            &self.light_buffer,
            &self.shadow_view,
            &self.shadow_sampler,
        );
    }

    /// Apply lighting configuration.
    pub fn apply_lighting_config(&mut self, config: &LightingConfig) {
        self.light.set_direction(Vec3::from_array(config.direction));
        self.light.color = Vec3::from_array(config.color);
        self.light.intensity = config.intensity;
        self.light.ambient_color = Vec3::from_array(config.ambient_color);
        self.light.ambient_strength = config.ambient_strength;
    }

    /// Apply camera configuration.
    pub fn apply_camera_config(&mut self, config: &CameraConfig) {
        self.camera.set_fov_degrees(config.fov_degrees);
        self.camera.set_near(config.near_plane);
        self.camera.set_far(config.far_plane);
        // Note: sensitivity values are used by the frontend, not stored here
    }

    /// Apply gizmo configuration.
    pub fn apply_gizmo_config(&mut self, config: &GizmoConfig, queue: &wgpu::Queue) {
        self.show_gizmo = config.enabled;
        // Apply axis colors from config
        self.gizmo_renderer.set_axis_colors(
            queue,
            config.x_axis_color,
            config.y_axis_color,
            config.z_axis_color,
        );
        // Note: gizmo scale is applied per-instance when showing the gizmo
    }

    /// Apply viewport configuration.
    ///
    /// Note: MSAA changes require renderer recreation and are not applied here.
    pub fn apply_viewport_config(&mut self, config: &ViewportConfig) {
        // Apply background color
        self.clear_color = wgpu::Color {
            r: config.background_color[0] as f64,
            g: config.background_color[1] as f64,
            b: config.background_color[2] as f64,
            a: config.background_color[3] as f64,
        };
        // Note: MSAA changes require recreation of pipelines and textures
    }

    /// Get the current MSAA sample count.
    pub fn sample_count(&self) -> u32 {
        SAMPLE_COUNT
    }
}
