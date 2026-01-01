//! 3D Viewport panel

use glam::Vec3;

use crate::app_state::SharedAppState;
use crate::panels::Panel;
use crate::viewport_state::SharedViewportState;

/// 3D viewport panel
pub struct ViewportPanel {
    last_size: egui::Vec2,
}

impl ViewportPanel {
    pub fn new() -> Self {
        Self {
            last_size: egui::Vec2::ZERO,
        }
    }

    fn render_axes_indicator(&self, ui: &mut egui::Ui, rect: egui::Rect, yaw: f32, pitch: f32) {
        let painter = ui.painter();
        let axes_center = rect.right_bottom() - egui::vec2(50.0, 50.0);
        let axis_len = 30.0;

        // Calculate camera basis vectors
        let cos_yaw = yaw.cos();
        let sin_yaw = yaw.sin();
        let cos_pitch = pitch.cos();
        let sin_pitch = pitch.sin();

        let forward = Vec3::new(-cos_pitch * cos_yaw, -cos_pitch * sin_yaw, -sin_pitch);
        let world_up = Vec3::Z;
        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward).normalize();

        let project_axis = |world_axis: Vec3| -> (egui::Vec2, f32) {
            let x = world_axis.dot(right);
            let y = world_axis.dot(up);
            let z = world_axis.dot(forward);
            (egui::vec2(x * axis_len, -y * axis_len), z)
        };

        let (x_dir, x_depth) = project_axis(Vec3::X);
        let (y_dir, y_depth) = project_axis(Vec3::Y);
        let (z_dir, z_depth) = project_axis(Vec3::Z);

        let mut axes = [
            (x_depth, x_dir, "X", egui::Color32::from_rgb(255, 68, 68)),
            (y_depth, y_dir, "Y", egui::Color32::from_rgb(68, 255, 68)),
            (z_depth, z_dir, "Z", egui::Color32::from_rgb(68, 68, 255)),
        ];
        axes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (_depth, dir, label, color) in axes {
            painter.line_segment(
                [axes_center, axes_center + dir],
                egui::Stroke::new(2.0, color),
            );

            let label_offset = dir.normalized() * 8.0;
            painter.text(
                axes_center + dir + label_offset,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::default(),
                color,
            );
        }
    }
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for ViewportPanel {
    fn name(&self) -> &str {
        "3D Viewport"
    }

    fn needs_render_context(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, _app_state: &SharedAppState) {
        // Fallback when no render context
        let available_size = ui.available_size();
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        painter.rect_filled(response.rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            "3D Viewport\n(WebGPU not available)",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );

        self.last_size = available_size;
    }

    fn ui_with_render_context(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &SharedAppState,
        render_state: &egui_wgpu::RenderState,
        viewport_state: &SharedViewportState,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("View:");
            if ui.button("Top").clicked() {
                viewport_state.lock().renderer.camera.set_top_view();
            }
            if ui.button("Front").clicked() {
                viewport_state.lock().renderer.camera.set_front_view();
            }
            if ui.button("Side").clicked() {
                viewport_state.lock().renderer.camera.set_side_view();
            }
            if ui.button("Fit All").clicked() {
                viewport_state.lock().renderer.camera.fit_all(Vec3::ZERO, 2.0);
            }

            ui.separator();

            let mut state = viewport_state.lock();
            ui.checkbox(&mut state.renderer.show_grid, "Grid");
            ui.checkbox(&mut state.renderer.show_axes, "Axes");
            ui.checkbox(&mut state.renderer.show_markers, "Markers");
        });

        // Main viewport area
        let available_size = ui.available_size();
        let width = available_size.x as u32;
        let height = available_size.y as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Ensure texture and render
        let texture_id = {
            let mut state = viewport_state.lock();
            let mut egui_renderer = render_state.renderer.write();
            let tex_id = state.ensure_texture(width, height, &mut egui_renderer);
            state.render();
            tex_id
        };

        // Display the rendered texture
        let response = ui.add(
            egui::Image::new(egui::load::SizedTexture::new(
                texture_id,
                [available_size.x, available_size.y],
            ))
            .sense(egui::Sense::click_and_drag()),
        );

        // Handle camera input
        let mut vp_state = viewport_state.lock();

        // Middle mouse button for orbit/pan
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            if ui.input(|i| i.modifiers.shift) {
                // Pan
                vp_state.renderer.camera.pan(delta.x, delta.y);
            } else {
                // Orbit
                let sensitivity = 0.005;
                vp_state.renderer.camera.orbit(-delta.x * sensitivity, delta.y * sensitivity);
            }
        }

        // Right mouse button for orbit as well
        if response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            let sensitivity = 0.005;
            vp_state.renderer.camera.orbit(-delta.x * sensitivity, delta.y * sensitivity);
        }

        // Zoom with scroll
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                vp_state.renderer.camera.zoom(scroll_delta * 0.01);
            }
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Reset View").clicked() {
                vp_state.renderer.camera.fit_all(Vec3::ZERO, 2.0);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Top View").clicked() {
                vp_state.renderer.camera.set_top_view();
                ui.close_menu();
            }
            if ui.button("Front View").clicked() {
                vp_state.renderer.camera.set_front_view();
                ui.close_menu();
            }
            if ui.button("Side View").clicked() {
                vp_state.renderer.camera.set_side_view();
                ui.close_menu();
            }
        });

        // Get camera state for axes indicator
        let yaw = vp_state.renderer.camera.yaw;
        let pitch = vp_state.renderer.camera.pitch;
        drop(vp_state);

        // Draw axes indicator overlay
        self.render_axes_indicator(ui, response.rect, yaw, pitch);

        self.last_size = available_size;
    }
}
