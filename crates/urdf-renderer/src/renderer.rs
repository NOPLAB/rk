//! Main renderer combining all sub-renderers

use std::collections::HashMap;

use glam::Mat4;
use uuid::Uuid;
use wgpu::util::DeviceExt;

use urdf_core::Part;

use crate::axis::{AxisInstance, AxisRenderer};
use crate::camera::Camera;
use crate::constants::viewport::CLEAR_COLOR;
use crate::gizmo::{GizmoAxis, GizmoMode, GizmoRenderer};
use crate::grid::GridRenderer;
use crate::marker::{MarkerInstance, MarkerRenderer};
use crate::mesh::{MeshData, MeshRenderer};

/// Mesh entry with bind group
pub struct MeshEntry {
    pub data: MeshData,
    pub bind_group: wgpu::BindGroup,
}

/// Main renderer
pub struct Renderer {
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // Sub-renderers
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

        Self {
            camera,
            camera_buffer,
            camera_bind_group_layout,
            depth_texture,
            depth_view,
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
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
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

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
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

        // Render meshes (iteration order doesn't matter for rendering)
        for entry in self.meshes.values() {
            self.mesh_renderer
                .render(&mut render_pass, &entry.data, &entry.bind_group);
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
}
