//! 3D Viewport panel

use glam::Vec3;
use urdf_renderer::GizmoAxis;

use crate::app_state::SharedAppState;
use crate::panels::Panel;
use crate::viewport_state::SharedViewportState;

/// 3D viewport panel
pub struct ViewportPanel {
    last_size: egui::Vec2,
    hovered_axis: GizmoAxis,
    show_camera_settings: bool,
}

impl ViewportPanel {
    pub fn new() -> Self {
        Self {
            last_size: egui::Vec2::ZERO,
            hovered_axis: GizmoAxis::None,
            show_camera_settings: false,
        }
    }

    /// Render camera settings overlay in the top-right corner (Unity-style)
    fn render_camera_settings(&mut self, ui: &mut egui::Ui, rect: egui::Rect, viewport_state: &SharedViewportState) {
        let panel_width = 180.0;
        let panel_margin = 10.0;

        // Toggle button at top-right
        let toggle_pos = egui::pos2(
            rect.right() - panel_margin - 28.0,
            rect.top() + panel_margin,
        );

        egui::Area::new(egui::Id::new("camera_toggle"))
            .fixed_pos(toggle_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .rounding(4.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        let button = egui::Button::new("📷")
                            .selected(self.show_camera_settings)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(button).on_hover_text("Camera Settings").clicked() {
                            self.show_camera_settings = !self.show_camera_settings;
                        }
                    });
            });

        // Camera settings panel (below toggle button)
        if !self.show_camera_settings {
            return;
        }

        let panel_pos = egui::pos2(
            rect.right() - panel_width - panel_margin,
            rect.top() + panel_margin + 36.0,
        );

        egui::Area::new(egui::Id::new("camera_settings_overlay"))
            .fixed_pos(panel_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .rounding(4.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(panel_width - 16.0);

                        ui.horizontal(|ui| {
                            ui.strong("Camera");
                        });

                        ui.separator();

                        let mut vp = viewport_state.lock();

                        // FOV slider
                        ui.horizontal(|ui| {
                            ui.label("FOV");
                            ui.add_space(ui.available_width() - 100.0);
                            let mut fov = vp.renderer.camera.fov_degrees();
                            if ui.add(
                                egui::Slider::new(&mut fov, 10.0..=120.0)
                                    .fixed_decimals(0)
                                    .suffix("°")
                                    .show_value(true)
                            ).changed() {
                                vp.renderer.camera.set_fov_degrees(fov);
                            }
                        });

                        // Near plane
                        ui.horizontal(|ui| {
                            ui.label("Near");
                            ui.add_space(ui.available_width() - 100.0);
                            let mut near = vp.renderer.camera.near;
                            if ui.add(
                                egui::DragValue::new(&mut near)
                                    .speed(0.01)
                                    .range(0.001..=100.0)
                                    .suffix(" m")
                            ).changed() {
                                vp.renderer.camera.set_near(near);
                            }
                        });

                        // Far plane
                        ui.horizontal(|ui| {
                            ui.label("Far");
                            ui.add_space(ui.available_width() - 100.0);
                            let mut far = vp.renderer.camera.far;
                            if ui.add(
                                egui::DragValue::new(&mut far)
                                    .speed(100.0)
                                    .range(10.0..=1000000.0)
                                    .suffix(" m")
                            ).changed() {
                                vp.renderer.camera.set_far(far);
                            }
                        });

                        ui.separator();

                        // Camera distance info
                        ui.horizontal(|ui| {
                            ui.label("Distance");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{:.2} m", vp.renderer.camera.distance));
                            });
                        });
                    });
            });
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

        // Get mouse position relative to viewport
        let mouse_pos = response.hover_pos().or(response.interact_pointer_pos());
        let local_mouse = mouse_pos.map(|p| p - response.rect.min);

        // Handle camera input
        let mut vp_state = viewport_state.lock();

        // Gizmo interaction (left mouse button)
        let mut gizmo_delta: Option<Vec3> = None;

        if let Some(pos) = local_mouse {
            // Check for gizmo hover
            if !vp_state.is_dragging_gizmo() {
                let hit_axis = vp_state.gizmo_hit_test(pos.x, pos.y, available_size.x, available_size.y);
                if hit_axis != self.hovered_axis {
                    self.hovered_axis = hit_axis;
                    let queue = vp_state.queue.clone();
                    vp_state.renderer.set_gizmo_highlight(&queue, hit_axis);
                }
            }

            // Start drag on left click
            if response.drag_started_by(egui::PointerButton::Primary) {
                if self.hovered_axis != GizmoAxis::None {
                    vp_state.start_gizmo_drag(self.hovered_axis, pos.x, pos.y, available_size.x, available_size.y);
                }
            }

            // Update drag
            if vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Primary) {
                gizmo_delta = vp_state.update_gizmo_drag(pos.x, pos.y, available_size.x, available_size.y);
            }

            // End drag
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                vp_state.end_gizmo_drag();
            }
        }

        // Apply gizmo delta to part transform or joint point position
        if let Some(delta) = gizmo_delta {
            if let Some(part_id) = vp_state.gizmo.part_id {
                let selected_joint_point = vp_state.gizmo.selected_joint_point;
                let queue = vp_state.queue.clone();
                drop(vp_state);

                let mut app = app_state.lock();

                if let Some(joint_idx) = selected_joint_point {
                    // Moving a joint point - update joint point position in local space
                    if let Some(part) = app.get_part_mut(part_id) {
                        if let Some(jp) = part.joint_points.get_mut(joint_idx) {
                            // Convert world delta to local delta using inverse transform
                            let inv_transform = part.origin_transform.inverse();
                            let local_delta = inv_transform.transform_vector3(delta);
                            jp.position += local_delta;
                        }
                    }
                    drop(app);
                } else {
                    // Moving the whole part - update part transform
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let translation = part.origin_transform.to_scale_rotation_translation().2;
                        let new_translation = translation + delta;
                        part.origin_transform = glam::Mat4::from_translation(new_translation);
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer.update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }

                // Re-lock viewport state for rest of handling
                vp_state = viewport_state.lock();
            }
        }

        // Middle mouse button for orbit/pan (only if not dragging gizmo)
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Middle) {
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
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Secondary) {
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

        // Draw camera settings overlay (top-right, Unity-style)
        self.render_camera_settings(ui, response.rect, viewport_state);

        self.last_size = available_size;
    }
}
