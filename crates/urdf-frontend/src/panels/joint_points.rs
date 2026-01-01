//! Joint points panel

use glam::Vec3;
use uuid::Uuid;

use urdf_core::{JointLimits, JointPoint, JointType};

use crate::app_state::SharedAppState;
use crate::panels::Panel;

/// Joint points panel
pub struct JointPointsPanel;

impl JointPointsPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JointPointsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for JointPointsPanel {
    fn name(&self) -> &str {
        "Joint Points"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        let mut state = app_state.lock();

        let Some(selected_id) = state.selected_part else {
            ui.weak("No part selected");
            return;
        };

        let selected_point = state.selected_joint_point.map(|(_, idx)| idx);

        let Some(part) = state.parts.get_mut(&selected_id) else {
            ui.weak("Selected part not found");
            return;
        };

        ui.horizontal(|ui| {
            ui.heading("Joint Points");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{}/8", part.joint_points.len()));
            });
        });

        ui.separator();

        // Add button
        let can_add = part.joint_points.len() < urdf_core::MAX_JOINT_POINTS;
        ui.add_enabled_ui(can_add, |ui| {
            if ui.button("Add Joint Point").clicked() {
                // Add at part center
                let center = part.center();
                let point = JointPoint::new(
                    format!("point_{}", part.joint_points.len() + 1),
                    center,
                );
                let _ = part.add_joint_point(point);
            }
        });

        ui.separator();

        // List of joint points
        let mut point_to_remove: Option<Uuid> = None;
        let mut point_to_select: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, point) in part.joint_points.iter_mut().enumerate() {
                let is_selected = selected_point == Some(idx);

                ui.push_id(point.id, |ui| {
                    let header = egui::CollapsingHeader::new(&point.name)
                        .default_open(is_selected)
                        .show(ui, |ui| {
                            // Name
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut point.name);
                            });

                            // Position
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                            });
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.add(egui::DragValue::new(&mut point.position.x).speed(0.01));
                                ui.label("Y:");
                                ui.add(egui::DragValue::new(&mut point.position.y).speed(0.01));
                                ui.label("Z:");
                                ui.add(egui::DragValue::new(&mut point.position.z).speed(0.01));
                            });

                            // Joint type
                            ui.horizontal(|ui| {
                                ui.label("Type:");
                                egui::ComboBox::from_id_salt("joint_type")
                                    .selected_text(point.joint_type.display_name())
                                    .show_ui(ui, |ui| {
                                        for jt in JointType::all() {
                                            ui.selectable_value(
                                                &mut point.joint_type,
                                                *jt,
                                                jt.display_name(),
                                            );
                                        }
                                    });
                            });

                            // Axis (for revolute/continuous/prismatic)
                            if point.joint_type.has_axis() {
                                ui.horizontal(|ui| {
                                    ui.label("Axis:");
                                    if ui.selectable_label(point.axis == Vec3::X, "X").clicked() {
                                        point.axis = Vec3::X;
                                    }
                                    if ui.selectable_label(point.axis == Vec3::Y, "Y").clicked() {
                                        point.axis = Vec3::Y;
                                    }
                                    if ui.selectable_label(point.axis == Vec3::Z, "Z").clicked() {
                                        point.axis = Vec3::Z;
                                    }
                                });
                            }

                            // Limits (for revolute/prismatic)
                            if point.joint_type.has_limits() {
                                let limits = point.limits.get_or_insert(JointLimits::default());
                                ui.collapsing("Limits", |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Lower:");
                                        ui.add(egui::DragValue::new(&mut limits.lower).speed(0.01));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Upper:");
                                        ui.add(egui::DragValue::new(&mut limits.upper).speed(0.01));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Effort:");
                                        ui.add(egui::DragValue::new(&mut limits.effort).speed(1.0));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Velocity:");
                                        ui.add(egui::DragValue::new(&mut limits.velocity).speed(0.1));
                                    });
                                });
                            } else {
                                point.limits = None;
                            }

                            // Delete button
                            ui.separator();
                            if ui.button("Delete").clicked() {
                                point_to_remove = Some(point.id);
                            }
                        });

                    if header.header_response.clicked() {
                        point_to_select = Some(idx);
                    }
                });
            }
        });

        // Handle actions
        if let Some(point_id) = point_to_remove {
            part.remove_joint_point(point_id);
        }

        let is_empty = part.joint_points.is_empty();

        // Update selection - drop state first
        drop(state);
        if let Some(idx) = point_to_select {
            app_state.lock().select_joint_point(selected_id, idx);
        }

        if is_empty {
            ui.weak("No joint points defined.\nClick 'Add Joint Point' to create one.");
        }
    }
}
