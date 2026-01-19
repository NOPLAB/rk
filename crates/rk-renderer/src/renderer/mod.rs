//! Main renderer combining all sub-renderers.
//!
//! This module provides the main [`Renderer`] struct that orchestrates all
//! rendering operations. It uses a plugin-based architecture where sub-renderers
//! can be registered via [`RendererRegistry`].
//!
//! # Architecture
//!
//! The renderer is composed of several specialized components:
//! - [`CameraController`]: Camera and view matrix management
//! - [`LightingSystem`]: Lighting and shadow resources
//! - [`PartManager`]: Part mesh management
//! - [`CadBodyManager`]: CAD body mesh management
//! - [`PreviewManager`]: Preview mesh for operations like extrusion
//! - [`DisplayOptions`]: Visibility toggles for rendering elements

mod cad_body_manager;
mod camera_controller;
mod display_options;
mod gpu_resources;
mod lighting_system;
mod part_manager;
mod preview_manager;
mod render_pass;

pub use cad_body_manager::CadBodyManager;
pub use camera_controller::CameraController;
pub use display_options::DisplayOptions;
pub use lighting_system::LightingSystem;
pub use part_manager::PartManager;
pub use preview_manager::PreviewManager;

use glam::{Mat4, Vec3};
use uuid::Uuid;

use rk_core::Part;

use crate::camera::Camera;
use crate::config::{
    CameraConfig, GizmoConfig, GridConfig, LightingConfig, RendererConfig, ShadowConfig,
    ViewportConfig,
};
use crate::constants::viewport::{CLEAR_COLOR, SAMPLE_COUNT};
use crate::light::DirectionalLight;
use crate::plugin::RendererRegistry;
use crate::resources::MeshManager;
use crate::scene::Scene;
use crate::sub_renderers::{
    AxisInstance, AxisRenderer, CollisionRenderer, GizmoAxis, GizmoMode, GizmoRenderer, GizmoSpace,
    GridRenderer, MarkerInstance, MarkerRenderer, MeshData, MeshRenderer, PlaneSelectorRenderer,
    SketchRenderData, SketchRenderer,
};

/// Mesh entry with bind group.
pub struct MeshEntry {
    /// Mesh data including vertex/index buffers and instance data.
    pub data: MeshData,
    /// Bind group for instance-specific uniforms.
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
pub struct Renderer {
    // New architectural components (optional during migration)
    scene: Scene,
    mesh_manager: MeshManager,
    registry: RendererRegistry,

    // Core components
    camera_controller: CameraController,
    lighting_system: LightingSystem,
    display_options: DisplayOptions,

    // Data managers
    part_manager: PartManager,
    cad_body_manager: CadBodyManager,
    preview_manager: PreviewManager,

    // Depth/MSAA resources
    #[allow(dead_code)] // Held for GPU resource lifetime
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    msaa_texture: Option<wgpu::Texture>,
    msaa_view: Option<wgpu::TextureView>,

    // Sub-renderers
    grid_renderer: GridRenderer,
    mesh_renderer: MeshRenderer,
    axis_renderer: AxisRenderer,
    marker_renderer: MarkerRenderer,
    gizmo_renderer: GizmoRenderer,
    collision_renderer: CollisionRenderer,
    sketch_renderer: SketchRenderer,
    plane_selector_renderer: PlaneSelectorRenderer,

    // Configurable rendering settings
    clear_color: wgpu::Color,

    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl Renderer {
    /// Creates a new renderer with the specified device and configuration.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let depth_format = wgpu::TextureFormat::Depth32Float;

        // Initialize camera controller
        let camera_controller = CameraController::new(device, width, height);

        // Create depth and MSAA textures
        let (depth_texture, depth_view) = gpu_resources::create_depth_texture(device, width, height);
        let msaa_result = gpu_resources::create_msaa_texture(device, format, width, height);
        let (msaa_texture, msaa_view) = match msaa_result {
            Some((tex, view)) => (Some(tex), Some(view)),
            None => (None, None),
        };

        // Initialize sub-renderers
        let grid_renderer = GridRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let mesh_renderer = MeshRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        // Initialize lighting system (needs mesh_renderer for bind group layout)
        let lighting_system = LightingSystem::new(device, &mesh_renderer);

        let axis_renderer = AxisRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let marker_renderer = MarkerRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let gizmo_renderer = GizmoRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let collision_renderer = CollisionRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let mut sketch_renderer = SketchRenderer::new();
        sketch_renderer.init(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        let plane_selector_renderer = PlaneSelectorRenderer::new(
            device,
            format,
            depth_format,
            camera_controller.bind_group_layout(),
            camera_controller.buffer(),
        );

        // Initialize architectural components
        let scene = Scene::new();
        let mesh_manager = MeshManager::new();
        let registry = RendererRegistry::new();

        Self {
            scene,
            mesh_manager,
            registry,

            camera_controller,
            lighting_system,
            display_options: DisplayOptions::default(),

            part_manager: PartManager::new(),
            cad_body_manager: CadBodyManager::new(),
            preview_manager: PreviewManager::new(),

            depth_texture,
            depth_view,
            msaa_texture,
            msaa_view,

            grid_renderer,
            mesh_renderer,
            axis_renderer,
            marker_renderer,
            gizmo_renderer,
            collision_renderer,
            sketch_renderer,
            plane_selector_renderer,

            clear_color: CLEAR_COLOR,
            format,
            width,
            height,
        }
    }

    // ========== Camera accessors ==========

    /// Get a reference to the camera.
    pub fn camera(&self) -> &Camera {
        self.camera_controller.camera()
    }

    /// Get a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        self.camera_controller.camera_mut()
    }

    // ========== Light accessors ==========

    /// Get a reference to the directional light.
    pub fn light(&self) -> &DirectionalLight {
        self.lighting_system.light()
    }

    /// Get a mutable reference to the directional light.
    pub fn light_mut(&mut self) -> &mut DirectionalLight {
        self.lighting_system.light_mut()
    }

    /// Set light direction (convenience method).
    pub fn set_light_direction(&mut self, direction: Vec3) {
        self.lighting_system.set_direction(direction);
    }

    /// Set light color and intensity (convenience method).
    pub fn set_light_color(&mut self, color: Vec3, intensity: f32) {
        self.lighting_system.set_color(color, intensity);
    }

    /// Set ambient lighting (convenience method).
    pub fn set_ambient(&mut self, color: Vec3, strength: f32) {
        self.lighting_system.set_ambient(color, strength);
    }

    /// Enable or disable shadows.
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.lighting_system.set_shadows_enabled(enabled);
    }

    /// Check if shadows are enabled.
    pub fn shadows_enabled(&self) -> bool {
        self.lighting_system.shadows_enabled()
    }

    // ========== Display option accessors ==========

    /// Get whether the grid is visible.
    pub fn show_grid(&self) -> bool {
        self.display_options.show_grid()
    }

    /// Set whether the grid is visible.
    pub fn set_show_grid(&mut self, show: bool) {
        self.display_options.set_show_grid(show);
    }

    /// Get whether axes are visible.
    pub fn show_axes(&self) -> bool {
        self.display_options.show_axes()
    }

    /// Set whether axes are visible.
    pub fn set_show_axes(&mut self, show: bool) {
        self.display_options.set_show_axes(show);
    }

    /// Get whether markers are visible.
    pub fn show_markers(&self) -> bool {
        self.display_options.show_markers()
    }

    /// Set whether markers are visible.
    pub fn set_show_markers(&mut self, show: bool) {
        self.display_options.set_show_markers(show);
    }

    /// Get whether the gizmo rendering is enabled.
    pub fn is_gizmo_enabled(&self) -> bool {
        self.display_options.is_gizmo_enabled()
    }

    /// Set whether the gizmo rendering is enabled.
    pub fn set_gizmo_enabled(&mut self, enabled: bool) {
        self.display_options.set_gizmo_enabled(enabled);
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

    /// Resizes the renderer's textures for a new viewport size.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.camera_controller.update_aspect(width, height);

        let (depth_texture, depth_view) = gpu_resources::create_depth_texture(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        // Recreate MSAA texture
        let msaa_result = gpu_resources::create_msaa_texture(device, self.format, width, height);
        let (msaa_texture, msaa_view) = match msaa_result {
            Some((tex, view)) => (Some(tex), Some(view)),
            None => (None, None),
        };
        self.msaa_texture = msaa_texture;
        self.msaa_view = msaa_view;
    }

    /// Returns the texture format used by the renderer.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Add a part to the renderer.
    ///
    /// Returns the part's UUID for reference.
    pub fn add_part(&mut self, device: &wgpu::Device, part: &Part) -> Uuid {
        self.part_manager.add(device, &self.mesh_renderer, part)
    }

    /// Update a part's transform.
    pub fn update_part_transform(&mut self, queue: &wgpu::Queue, part_id: Uuid, transform: Mat4) {
        self.part_manager.update_transform(queue, part_id, transform);
    }

    /// Update a part's color.
    pub fn update_part_color(&mut self, queue: &wgpu::Queue, part_id: Uuid, color: [f32; 4]) {
        self.part_manager.update_color(queue, part_id, color);
    }

    /// Set selected part.
    pub fn set_selected_part(&mut self, queue: &wgpu::Queue, part_id: Option<Uuid>) {
        self.part_manager.set_selected(queue, part_id);
    }

    /// Get the currently selected part ID.
    pub fn selected_part(&self) -> Option<Uuid> {
        self.part_manager.selected()
    }

    /// Remove a part - O(1) operation with UUID-based storage.
    pub fn remove_part(&mut self, part_id: Uuid) {
        self.part_manager.remove(part_id);
    }

    /// Clear all parts.
    pub fn clear_parts(&mut self) {
        self.part_manager.clear();
    }

    /// Check if a part exists.
    pub fn has_part(&self, part_id: Uuid) -> bool {
        self.part_manager.has(part_id)
    }

    /// Get the number of parts.
    pub fn part_count(&self) -> usize {
        self.part_manager.count()
    }

    // ========== Preview mesh methods ==========

    /// Set a preview mesh for extrusion preview.
    ///
    /// The mesh will be rendered with a semi-transparent appearance.
    pub fn set_preview_mesh(
        &mut self,
        device: &wgpu::Device,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
        transform: Mat4,
    ) {
        self.preview_manager
            .set_preview_mesh(device, &self.mesh_renderer, vertices, normals, indices, transform);
    }

    /// Clear the preview mesh.
    pub fn clear_preview_mesh(&mut self) {
        self.preview_manager.clear();
    }

    /// Check if there's a preview mesh.
    pub fn has_preview_mesh(&self) -> bool {
        self.preview_manager.has_preview()
    }

    // ========== CAD body methods ==========

    /// Add a CAD body mesh for persistent display.
    ///
    /// CAD bodies are rendered like regular parts, but managed separately.
    #[allow(clippy::too_many_arguments)]
    pub fn add_cad_body(
        &mut self,
        device: &wgpu::Device,
        body_id: Uuid,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
        transform: Mat4,
        color: [f32; 4],
    ) {
        self.cad_body_manager.add(
            device,
            &self.mesh_renderer,
            body_id,
            vertices,
            normals,
            indices,
            transform,
            color,
        );
    }

    /// Remove a CAD body.
    pub fn remove_cad_body(&mut self, body_id: Uuid) {
        self.cad_body_manager.remove(body_id);
    }

    /// Clear all CAD bodies.
    pub fn clear_cad_bodies(&mut self) {
        self.cad_body_manager.clear();
    }

    /// Check if a CAD body exists.
    pub fn has_cad_body(&self, body_id: Uuid) -> bool {
        self.cad_body_manager.has(body_id)
    }

    /// Get the number of CAD bodies.
    pub fn cad_body_count(&self) -> usize {
        self.cad_body_manager.count()
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
        self.marker_renderer.update_selected_instances(queue, instances);
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
        let camera = self.camera_controller.camera();
        let camera_distance = (camera.position - gizmo_pos).length();
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

    /// Set gizmo coordinate space
    pub fn set_gizmo_space(&mut self, queue: &wgpu::Queue, space: GizmoSpace) {
        self.gizmo_renderer.set_space(queue, space);
    }

    /// Get current gizmo coordinate space
    pub fn gizmo_space(&self) -> GizmoSpace {
        self.gizmo_renderer.space()
    }

    /// Set object rotation for local coordinate space
    pub fn set_gizmo_object_rotation(&mut self, queue: &wgpu::Queue, rotation: glam::Quat) {
        self.gizmo_renderer.set_object_rotation(queue, rotation);
    }

    /// Get the current object rotation for the gizmo
    pub fn gizmo_object_rotation(&self) -> glam::Quat {
        self.gizmo_renderer.object_rotation()
    }

    /// Get axis direction based on current coordinate space
    pub fn gizmo_axis_direction(&self, axis: GizmoAxis) -> glam::Vec3 {
        self.gizmo_renderer.get_axis_direction(axis)
    }

    // ========== Collision renderer methods ==========

    /// Get mutable reference to collision renderer
    pub fn collision_renderer_mut(&mut self) -> &mut CollisionRenderer {
        &mut self.collision_renderer
    }

    /// Get reference to collision renderer
    pub fn collision_renderer(&self) -> &CollisionRenderer {
        &self.collision_renderer
    }

    // ========== Sketch renderer methods ==========

    /// Get mutable reference to sketch renderer
    pub fn sketch_renderer_mut(&mut self) -> &mut SketchRenderer {
        &mut self.sketch_renderer
    }

    /// Get reference to sketch renderer
    pub fn sketch_renderer(&self) -> &SketchRenderer {
        &self.sketch_renderer
    }

    /// Set sketch data to render
    pub fn set_sketch_data(&mut self, sketches: Vec<SketchRenderData>) {
        self.sketch_renderer.set_sketches(sketches);
    }

    /// Prepare sketch renderer (call before render)
    pub fn prepare_sketches(&mut self, device: &wgpu::Device) {
        self.sketch_renderer.prepare_with_device(device);
    }

    // ========== Plane selector renderer methods ==========

    /// Get mutable reference to plane selector renderer
    pub fn plane_selector_renderer_mut(&mut self) -> &mut PlaneSelectorRenderer {
        &mut self.plane_selector_renderer
    }

    /// Get reference to plane selector renderer
    pub fn plane_selector_renderer(&self) -> &PlaneSelectorRenderer {
        &self.plane_selector_renderer
    }

    /// Set plane selector visibility
    pub fn set_plane_selector_visible(&mut self, visible: bool) {
        self.plane_selector_renderer.set_visible(visible);
    }

    /// Get plane selector visibility
    pub fn plane_selector_visible(&self) -> bool {
        self.plane_selector_renderer.is_visible()
    }

    /// Set plane selector highlighted plane
    pub fn set_plane_selector_highlighted(&mut self, queue: &wgpu::Queue, plane_id: u32) {
        self.plane_selector_renderer.set_highlighted(queue, plane_id);
    }

    /// Get plane selector highlighted plane ID
    pub fn plane_selector_highlighted(&self) -> u32 {
        self.plane_selector_renderer.highlighted()
    }

    /// Render the scene.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) {
        self.camera_controller.update(queue);
        self.lighting_system.update(queue, self.camera_controller.camera().target);

        // Shadow pass
        let shadow_params = render_pass::ShadowPassParams {
            lighting: &self.lighting_system,
            parts: &self.part_manager,
            cad_bodies: &self.cad_body_manager,
            mesh_renderer: &self.mesh_renderer,
        };
        render_pass::render_shadow_pass(encoder, &shadow_params);

        // Main pass
        let main_params = render_pass::MainPassParams {
            display_options: &self.display_options,
            lighting: &self.lighting_system,
            parts: &self.part_manager,
            cad_bodies: &self.cad_body_manager,
            preview: &self.preview_manager,
            grid_renderer: &self.grid_renderer,
            mesh_renderer: &self.mesh_renderer,
            axis_renderer: &self.axis_renderer,
            marker_renderer: &self.marker_renderer,
            collision_renderer: &self.collision_renderer,
            sketch_renderer: &self.sketch_renderer,
            plane_selector_renderer: &self.plane_selector_renderer,
            gizmo_renderer: &self.gizmo_renderer,
            depth_view: &self.depth_view,
            msaa_view: self.msaa_view.as_ref(),
            clear_color: self.clear_color,
        };
        render_pass::render_main_pass(encoder, view, &main_params);
    }

    /// Get camera bind group layout for external use
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.camera_controller.bind_group_layout()
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
        self.display_options.set_show_grid(config.enabled);
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
        self.lighting_system.apply_shadow_config(config, device, &self.mesh_renderer);
    }

    /// Apply lighting configuration.
    pub fn apply_lighting_config(&mut self, config: &LightingConfig) {
        self.lighting_system.apply_lighting_config(config);
    }

    /// Apply camera configuration.
    pub fn apply_camera_config(&mut self, config: &CameraConfig) {
        self.camera_controller.apply_config(config);
    }

    /// Apply gizmo configuration.
    pub fn apply_gizmo_config(&mut self, config: &GizmoConfig, queue: &wgpu::Queue) {
        self.display_options.set_gizmo_enabled(config.enabled);
        // Apply axis colors from config
        self.gizmo_renderer.set_axis_colors(
            queue,
            config.x_axis_color,
            config.y_axis_color,
            config.z_axis_color,
        );
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
    }

    /// Get the current MSAA sample count.
    pub fn sample_count(&self) -> u32 {
        SAMPLE_COUNT
    }
}
