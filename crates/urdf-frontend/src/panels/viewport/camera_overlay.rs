//! Camera settings overlay for the 3D viewport

use glam::Vec3;

use crate::state::SharedViewportState;

/// Render camera settings overlay in the top-right corner (Unity-style)
pub fn render_camera_settings(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
    show_camera_settings: &mut bool,
) {
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
                        .selected(*show_camera_settings)
                        .min_size(egui::vec2(24.0, 24.0));
                    if ui.add(button).on_hover_text("Camera Settings").clicked() {
                        *show_camera_settings = !*show_camera_settings;
                    }
                });
        });

    // Camera settings panel (below toggle button)
    if !*show_camera_settings {
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
                        let mut fov = vp.renderer.camera().fov_degrees();
                        if ui
                            .add(
                                egui::Slider::new(&mut fov, 10.0..=120.0)
                                    .fixed_decimals(0)
                                    .suffix("°")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_fov_degrees(fov);
                        }
                    });

                    // Near plane
                    ui.horizontal(|ui| {
                        ui.label("Near");
                        ui.add_space(ui.available_width() - 100.0);
                        let mut near = vp.renderer.camera().near;
                        if ui
                            .add(
                                egui::DragValue::new(&mut near)
                                    .speed(0.01)
                                    .range(0.001..=100.0)
                                    .suffix(" m"),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_near(near);
                        }
                    });

                    // Far plane
                    ui.horizontal(|ui| {
                        ui.label("Far");
                        ui.add_space(ui.available_width() - 100.0);
                        let mut far = vp.renderer.camera().far;
                        if ui
                            .add(
                                egui::DragValue::new(&mut far)
                                    .speed(100.0)
                                    .range(10.0..=1000000.0)
                                    .suffix(" m"),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_far(far);
                        }
                    });

                    ui.separator();

                    // Camera distance info
                    ui.horizontal(|ui| {
                        ui.label("Distance");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{:.2} m", vp.renderer.camera().distance));
                        });
                    });
                });
        });
}

/// Render axes indicator in the bottom-right corner
pub fn render_axes_indicator(ui: &mut egui::Ui, rect: egui::Rect, yaw: f32, pitch: f32) {
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
