//! Gizmo mode toggle overlay for the viewport

use rk_renderer::{GizmoMode, GizmoSpace};

use crate::state::SharedViewportState;

/// Render gizmo mode toggle in the top-left corner (floating UI)
/// Returns true if the collapsed state was toggled
pub fn render_gizmo_toggle(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
    collapsed: &mut bool,
) -> bool {
    let panel_margin = 10.0;
    let mut toggled = false;

    // When collapsed, show only the expand arrow button
    if *collapsed {
        let arrow_pos = egui::pos2(rect.left() + panel_margin, rect.top() + panel_margin);

        egui::Area::new(egui::Id::new("gizmo_expand_btn"))
            .fixed_pos(arrow_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .corner_radius(4.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        // Expand button (right arrow)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Right pointing arrow
                        let tip = egui::pos2(center.x + arrow_width, center.y);
                        let top = egui::pos2(center.x - arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x - arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = false;
                            toggled = true;
                        }

                        response.on_hover_text("Show gizmo tools");
                    });
            });

        return toggled;
    }

    // When expanded, show the full toolbar
    let toggle_pos = egui::pos2(rect.left() + panel_margin, rect.top() + panel_margin);

    egui::Area::new(egui::Id::new("gizmo_toggle"))
        .fixed_pos(toggle_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(2.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;

                        let mut vp = viewport_state.lock();
                        let current_mode = vp.renderer.gizmo_mode();
                        let current_space = vp.renderer.gizmo_space();

                        // Mode buttons
                        let translate_btn = egui::Button::new("↔")
                            .selected(current_mode == GizmoMode::Translate)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(translate_btn).on_hover_text("Move (T)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Translate);
                        }

                        let rotate_btn = egui::Button::new("⟳")
                            .selected(current_mode == GizmoMode::Rotate)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(rotate_btn).on_hover_text("Rotate (R)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Rotate);
                        }

                        let scale_btn = egui::Button::new("⤢")
                            .selected(current_mode == GizmoMode::Scale)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(scale_btn).on_hover_text("Scale (S)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Scale);
                        }

                        // Separator
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Coordinate space toggle
                        let (space_icon, space_text, next_space) = match current_space {
                            GizmoSpace::Global => ("🌐", "Global (G)", GizmoSpace::Local),
                            GizmoSpace::Local => ("📦", "Local (G)", GizmoSpace::Global),
                        };
                        let space_btn =
                            egui::Button::new(space_icon).min_size(egui::vec2(24.0, 24.0));
                        if ui.add(space_btn).on_hover_text(space_text).clicked() {
                            let queue = vp.queue.clone();
                            vp.renderer.set_gizmo_space(&queue, next_space);
                        }

                        drop(vp);

                        // Separator before collapse button
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Collapse toggle button (thin arrow)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        // Button background on hover
                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        // Draw arrow
                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Arrow direction: left when expanded (to collapse)
                        let tip = egui::pos2(center.x - arrow_width, center.y);
                        let top = egui::pos2(center.x + arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x + arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        // Draw thin arrow (just lines)
                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = true;
                            toggled = true;
                        }

                        response.on_hover_text("Hide gizmo tools");
                    });
                });
        });

    toggled
}
