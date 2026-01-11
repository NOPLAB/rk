//! Viewport rendering state

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use parking_lot::Mutex;
use uuid::Uuid;

use urdf_core::Part;
use urdf_renderer::{GizmoAxis, GizmoMode, Renderer, axis::AxisInstance, marker::MarkerInstance};

/// Render texture for viewport
struct RenderTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    egui_texture_id: egui::TextureId,
    width: u32,
    height: u32,
}

/// Gizmo transform result from drag operation
pub enum GizmoTransform {
    Translation(Vec3),
    Rotation(Quat),
}

/// Gizmo interaction state
#[derive(Default)]
pub struct GizmoInteraction {
    pub dragging: bool,
    pub drag_axis: GizmoAxis,
    pub drag_start_pos: Vec3,
    pub drag_start_angle: f32,
    pub part_start_transform: Mat4,
    pub part_id: Option<Uuid>,
    pub selected_joint_point: Option<usize>,
    pub gizmo_position: Vec3,
    pub gizmo_scale: f32,
}

/// Viewport rendering state
pub struct ViewportState {
    pub renderer: Renderer,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    render_texture: Option<RenderTexture>,
    pub gizmo: GizmoInteraction,
}

impl ViewportState {
    /// Create a new viewport state
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let renderer = Renderer::new(&device, format, 800, 600);
        Self {
            renderer,
            device,
            queue,
            render_texture: None,
            gizmo: GizmoInteraction::default(),
        }
    }

    /// Ensure the render texture matches the requested size
    pub fn ensure_texture(
        &mut self,
        width: u32,
        height: u32,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> egui::TextureId {
        let width = width.max(1);
        let height = height.max(1);

        let needs_recreate = self
            .render_texture
            .as_ref()
            .is_none_or(|t| t.width != width || t.height != height);

        if needs_recreate {
            // Free old texture if exists
            if let Some(old) = self.render_texture.take() {
                egui_renderer.free_texture(&old.egui_texture_id);
            }

            // Create new texture
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Viewport Render Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.renderer.format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Register with egui
            let egui_texture_id = egui_renderer.register_native_texture(
                &self.device,
                &view,
                wgpu::FilterMode::Linear,
            );

            // Resize renderer
            self.renderer.resize(&self.device, width, height);

            self.render_texture = Some(RenderTexture {
                texture,
                view,
                egui_texture_id,
                width,
                height,
            });
        }

        self.render_texture.as_ref().unwrap().egui_texture_id
    }

    /// Render the 3D scene to the texture
    pub fn render(&mut self) {
        let Some(ref rt) = self.render_texture else {
            return;
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Viewport Render Encoder"),
            });

        self.renderer.render(&mut encoder, &rt.view, &self.queue);

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Add a part to the viewport
    pub fn add_part(&mut self, part: &Part) -> Uuid {
        self.renderer.add_part(&self.device, part)
    }

    /// Update a part's transform
    pub fn update_part_transform(&mut self, part_id: Uuid, transform: Mat4) {
        self.renderer
            .update_part_transform(&self.queue, part_id, transform);
    }

    /// Update a part's color
    pub fn update_part_color(&mut self, part_id: Uuid, color: [f32; 4]) {
        self.renderer.update_part_color(&self.queue, part_id, color);
    }

    /// Set selected part
    pub fn set_selected_part(&mut self, part_id: Option<Uuid>) {
        self.renderer.set_selected_part(&self.queue, part_id);
    }

    /// Remove a part
    pub fn remove_part(&mut self, part_id: Uuid) {
        self.renderer.remove_part(part_id);
    }

    /// Clear all parts
    pub fn clear_parts(&mut self) {
        self.renderer.clear_parts();
    }

    /// Update axes display for a part
    pub fn update_axes_for_part(&mut self, part: &Part) {
        let instance = AxisInstance {
            transform: part.origin_transform.to_cols_array_2d(),
            scale: 0.3,
            _pad: [0.0; 3],
        };
        self.renderer.update_axes(&self.queue, &[instance]);
    }

    /// Update joint point markers for a part
    pub fn update_markers_for_part(
        &mut self,
        part: &Part,
        joint_points: &[urdf_core::JointPoint],
        selected_point: Option<usize>,
    ) {
        let instances: Vec<MarkerInstance> = joint_points
            .iter()
            .enumerate()
            .map(|(i, jp)| {
                let world_pos = part.origin_transform.transform_point3(jp.position);
                let color = if Some(i) == selected_point {
                    [1.0, 0.8, 0.2, 1.0] // Gold for selected
                } else {
                    match jp.joint_type {
                        urdf_core::JointType::Fixed => [0.5, 0.5, 1.0, 1.0], // Blue
                        urdf_core::JointType::Revolute => [0.2, 1.0, 0.2, 1.0], // Green
                        urdf_core::JointType::Continuous => [0.2, 0.8, 1.0, 1.0], // Cyan
                        urdf_core::JointType::Prismatic => [1.0, 0.5, 0.2, 1.0], // Orange
                        _ => [0.8, 0.8, 0.8, 1.0],                           // Gray
                    }
                };
                MarkerInstance::new(world_pos, 0.02, color)
            })
            .collect();

        self.renderer.update_markers(&self.queue, &instances);
    }

    /// Clear axes and markers
    pub fn clear_overlays(&mut self) {
        self.renderer.update_axes(&self.queue, &[]);
        self.renderer.update_markers(&self.queue, &[]);
        self.renderer.hide_gizmo();
    }

    /// Show gizmo for a part
    pub fn show_gizmo_for_part(&mut self, part: &Part) {
        // Calculate center from bounding box
        let center = Vec3::new(
            (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
            (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
            (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
        );
        // Transform center by part's origin transform
        let world_center = part.origin_transform.transform_point3(center);

        // Use fixed scale - shader handles distance-based scaling for constant screen size
        let scale = 1.0;

        // Store gizmo state
        self.gizmo.gizmo_position = world_center;
        self.gizmo.gizmo_scale = scale;
        self.gizmo.part_id = Some(part.id);
        self.gizmo.selected_joint_point = None; // Not a joint point
        self.gizmo.part_start_transform = part.origin_transform;

        self.renderer.show_gizmo(&self.queue, world_center, scale);
    }

    /// Show gizmo for a joint point
    pub fn show_gizmo_for_joint_point(
        &mut self,
        part: &Part,
        joint_points: &[urdf_core::JointPoint],
        point_idx: usize,
    ) {
        if let Some(jp) = joint_points.get(point_idx) {
            // Transform joint point position by part's origin transform
            let world_pos = part.origin_transform.transform_point3(jp.position);

            // Use fixed scale - shader handles distance-based scaling for constant screen size
            let scale = 1.0;

            // Store gizmo state
            self.gizmo.gizmo_position = world_pos;
            self.gizmo.gizmo_scale = scale;
            self.gizmo.part_id = Some(part.id);
            self.gizmo.selected_joint_point = Some(point_idx); // Track which joint point
            self.gizmo.part_start_transform = part.origin_transform;

            self.renderer.show_gizmo(&self.queue, world_pos, scale);
        }
    }

    /// Hide gizmo
    pub fn hide_gizmo(&mut self) {
        self.renderer.hide_gizmo();
        self.gizmo.part_id = None;
    }

    /// Test if a screen position hits the gizmo
    pub fn gizmo_hit_test(
        &self,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) -> GizmoAxis {
        if self.gizmo.part_id.is_none() {
            return GizmoAxis::None;
        }

        let (ray_origin, ray_dir) = self
            .renderer
            .camera()
            .screen_to_ray(screen_x, screen_y, width, height);
        self.renderer.gizmo_hit_test(
            ray_origin,
            ray_dir,
            self.gizmo.gizmo_position,
            self.gizmo.gizmo_scale,
        )
    }

    /// Start dragging the gizmo
    pub fn start_gizmo_drag(
        &mut self,
        axis: GizmoAxis,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) {
        if axis == GizmoAxis::None {
            return;
        }

        let (ray_origin, ray_dir) = self
            .renderer
            .camera()
            .screen_to_ray(screen_x, screen_y, width, height);

        let mode = self.renderer.gizmo_mode();

        match mode {
            GizmoMode::Translate => {
                // Calculate intersection point with the axis plane
                let plane_normal = self.get_drag_plane_normal(axis);

                if let Some(point) = ray_plane_intersection(
                    ray_origin,
                    ray_dir,
                    self.gizmo.gizmo_position,
                    plane_normal,
                ) {
                    self.gizmo.dragging = true;
                    self.gizmo.drag_axis = axis;
                    self.gizmo.drag_start_pos = point;
                    self.renderer.set_gizmo_highlight(&self.queue, axis);
                }
            }
            GizmoMode::Rotate => {
                // For rotation, intersect with the plane perpendicular to the rotation axis
                let rotation_axis = axis.direction();

                if let Some(point) = ray_plane_intersection(
                    ray_origin,
                    ray_dir,
                    self.gizmo.gizmo_position,
                    rotation_axis,
                ) {
                    self.gizmo.dragging = true;
                    self.gizmo.drag_axis = axis;
                    self.gizmo.drag_start_pos = point;
                    // Calculate initial angle from gizmo center
                    let offset = point - self.gizmo.gizmo_position;
                    self.gizmo.drag_start_angle = self.angle_on_plane(offset, axis);
                    self.renderer.set_gizmo_highlight(&self.queue, axis);
                }
            }
        }
    }

    /// Update gizmo drag - returns the transform delta if dragging
    pub fn update_gizmo_drag(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) -> Option<GizmoTransform> {
        if !self.gizmo.dragging {
            return None;
        }

        let mode = self.renderer.gizmo_mode();

        match mode {
            GizmoMode::Translate => self.update_translate_drag(screen_x, screen_y, width, height),
            GizmoMode::Rotate => self.update_rotate_drag(screen_x, screen_y, width, height),
        }
    }

    fn update_translate_drag(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) -> Option<GizmoTransform> {
        let (ray_origin, ray_dir) = self
            .renderer
            .camera()
            .screen_to_ray(screen_x, screen_y, width, height);
        let plane_normal = self.get_drag_plane_normal(self.gizmo.drag_axis);

        if let Some(current_point) =
            ray_plane_intersection(ray_origin, ray_dir, self.gizmo.gizmo_position, plane_normal)
        {
            let delta = current_point - self.gizmo.drag_start_pos;

            // Project delta onto the axis
            let axis_dir = self.gizmo.drag_axis.direction();
            let projected_delta = axis_dir * delta.dot(axis_dir);

            // Update gizmo position
            self.gizmo.gizmo_position += projected_delta;
            self.gizmo.drag_start_pos = current_point;

            // Update gizmo visual
            self.renderer.show_gizmo(
                &self.queue,
                self.gizmo.gizmo_position,
                self.gizmo.gizmo_scale,
            );

            return Some(GizmoTransform::Translation(projected_delta));
        }

        None
    }

    fn update_rotate_drag(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
    ) -> Option<GizmoTransform> {
        let (ray_origin, ray_dir) = self
            .renderer
            .camera()
            .screen_to_ray(screen_x, screen_y, width, height);
        let rotation_axis = self.gizmo.drag_axis.direction();

        if let Some(current_point) = ray_plane_intersection(
            ray_origin,
            ray_dir,
            self.gizmo.gizmo_position,
            rotation_axis,
        ) {
            let offset = current_point - self.gizmo.gizmo_position;
            let current_angle = self.angle_on_plane(offset, self.gizmo.drag_axis);
            let angle_delta = current_angle - self.gizmo.drag_start_angle;

            // Update start angle for next frame
            self.gizmo.drag_start_angle = current_angle;

            // Create rotation quaternion around the axis
            let rotation = Quat::from_axis_angle(rotation_axis, angle_delta);

            return Some(GizmoTransform::Rotation(rotation));
        }

        None
    }

    /// Calculate angle of a point on a plane perpendicular to the given axis
    fn angle_on_plane(&self, offset: Vec3, axis: GizmoAxis) -> f32 {
        match axis {
            GizmoAxis::X => offset.z.atan2(offset.y),
            GizmoAxis::Y => offset.x.atan2(offset.z),
            GizmoAxis::Z => offset.y.atan2(offset.x),
            GizmoAxis::None => 0.0,
        }
    }

    /// End gizmo drag
    pub fn end_gizmo_drag(&mut self) {
        self.gizmo.dragging = false;
        self.gizmo.drag_axis = GizmoAxis::None;
        self.renderer
            .set_gizmo_highlight(&self.queue, GizmoAxis::None);
    }

    /// Get the plane normal for dragging on an axis
    fn get_drag_plane_normal(&self, axis: GizmoAxis) -> Vec3 {
        let camera = self.renderer.camera();
        let camera_forward = (camera.target - camera.position).normalize();

        match axis {
            GizmoAxis::X => {
                // Use plane with normal most perpendicular to X
                if camera_forward.y.abs() > camera_forward.z.abs() {
                    Vec3::Y
                } else {
                    Vec3::Z
                }
            }
            GizmoAxis::Y => {
                if camera_forward.x.abs() > camera_forward.z.abs() {
                    Vec3::X
                } else {
                    Vec3::Z
                }
            }
            GizmoAxis::Z => {
                if camera_forward.x.abs() > camera_forward.y.abs() {
                    Vec3::X
                } else {
                    Vec3::Y
                }
            }
            GizmoAxis::None => camera_forward,
        }
    }

    /// Check if currently dragging
    pub fn is_dragging_gizmo(&self) -> bool {
        self.gizmo.dragging
    }
}

/// Ray-plane intersection
fn ray_plane_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let denom = ray_dir.dot(plane_normal);
    if denom.abs() < 1e-6 {
        return None;
    }

    let t = (plane_point - ray_origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return None;
    }

    Some(ray_origin + ray_dir * t)
}

pub type SharedViewportState = Arc<Mutex<ViewportState>>;
