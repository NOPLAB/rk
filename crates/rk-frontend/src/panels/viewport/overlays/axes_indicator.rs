//! Axes indicator overlay for the viewport

use glam::Vec3;

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
