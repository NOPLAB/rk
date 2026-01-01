//! Viewport rendering state

use std::sync::Arc;

use glam::Mat4;
use parking_lot::Mutex;
use uuid::Uuid;

use urdf_core::Part;
use urdf_renderer::{axis::AxisInstance, marker::MarkerInstance, Renderer};

/// Render texture for viewport
struct RenderTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    egui_texture_id: egui::TextureId,
    width: u32,
    height: u32,
}

/// Viewport rendering state
pub struct ViewportState {
    pub renderer: Renderer,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    render_texture: Option<RenderTexture>,
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
    pub fn add_part(&mut self, part: &Part) -> usize {
        self.renderer.add_part(&self.device, part)
    }

    /// Update a part's transform
    pub fn update_part_transform(&mut self, part_id: Uuid, transform: Mat4) {
        self.renderer.update_part_transform(&self.queue, part_id, transform);
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
            _padding: [0.0; 3],
        };
        self.renderer.update_axes(&self.queue, &[instance]);
    }

    /// Update joint point markers for a part
    pub fn update_markers_for_part(&mut self, part: &Part, selected_point: Option<usize>) {
        let instances: Vec<MarkerInstance> = part
            .joint_points
            .iter()
            .enumerate()
            .map(|(i, jp)| {
                let world_pos = part.origin_transform.transform_point3(jp.position);
                let color = if Some(i) == selected_point {
                    [1.0, 0.8, 0.2, 1.0] // Gold for selected
                } else {
                    match jp.joint_type {
                        urdf_core::JointType::Fixed => [0.5, 0.5, 1.0, 1.0],     // Blue
                        urdf_core::JointType::Revolute => [0.2, 1.0, 0.2, 1.0],  // Green
                        urdf_core::JointType::Continuous => [0.2, 0.8, 1.0, 1.0], // Cyan
                        urdf_core::JointType::Prismatic => [1.0, 0.5, 0.2, 1.0], // Orange
                        _ => [0.8, 0.8, 0.8, 1.0],                               // Gray
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
    }
}

pub type SharedViewportState = Arc<Mutex<ViewportState>>;
