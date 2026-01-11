//! Joint Points component - joint point list management

use egui::{DragValue, Ui};
use glam::Vec3;
use uuid::Uuid;

use urdf_core::{JointLimits, JointPoint, JointType};

use crate::panels::properties::{JointPointAction, PropertyComponent, PropertyContext};

/// Maximum number of joint points per part
const MAX_JOINT_POINTS: usize = 8;

/// Joint points list component
pub struct JointPointsComponent {
    /// Pending selection action
    pending_select: Option<usize>,
}

impl JointPointsComponent {
    pub fn new() -> Self {
        Self {
            pending_select: None,
        }
    }

    /// Take pending selection action (if any)
    pub fn take_pending_select(&mut self) -> Option<usize> {
        self.pending_select.take()
    }
}

impl Default for JointPointsComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for JointPointsComponent {
    fn name(&self) -> &str {
        "Joint Points"
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        let mut changed = false;
        let selected_point = ctx.selected_joint_point;
        let part_id = ctx.part_id;

        // Header with count badge
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{}/{}", ctx.joint_points.len(), MAX_JOINT_POINTS));
            });
        });

        // Add button
        let can_add = ctx.joint_points.len() < MAX_JOINT_POINTS;
        ui.add_enabled_ui(can_add, |ui| {
            if ui.button("Add Joint Point").clicked() {
                let center = ctx.part.center();
                let point = JointPoint::new(
                    format!("point_{}", ctx.joint_points.len() + 1),
                    part_id,
                    center,
                );
                ctx.joint_point_actions.push(JointPointAction::Add(point));
                changed = true;
            }
        });

        ui.separator();

        // List of joint points
        let mut point_to_remove: Option<Uuid> = None;

        egui::ScrollArea::vertical()
            .id_salt("joint_points_scroll")
            .show(ui, |ui| {
                for (idx, point) in ctx.joint_points.iter_mut().enumerate() {
                    let is_selected = selected_point == Some(idx);
                    let point_id = point.id;

                    ui.push_id(point_id, |ui| {
                        let header = egui::CollapsingHeader::new(&point.name)
                            .default_open(is_selected)
                            .show(ui, |ui| {
                                // Name
                                ui.horizontal(|ui| {
                                    ui.label("Name:");
                                    if ui.text_edit_singleline(&mut point.name).changed() {
                                        ctx.joint_point_actions
                                            .push(JointPointAction::Update(point.clone()));
                                        changed = true;
                                    }
                                });

                                // Position
                                ui.horizontal(|ui| {
                                    ui.label("Position:");
                                });
                                ui.horizontal(|ui| {
                                    ui.label("X:");
                                    if ui
                                        .add(DragValue::new(&mut point.position.x).speed(0.01))
                                        .changed()
                                    {
                                        ctx.joint_point_actions
                                            .push(JointPointAction::Update(point.clone()));
                                        changed = true;
                                    }
                                    ui.label("Y:");
                                    if ui
                                        .add(DragValue::new(&mut point.position.y).speed(0.01))
                                        .changed()
                                    {
                                        ctx.joint_point_actions
                                            .push(JointPointAction::Update(point.clone()));
                                        changed = true;
                                    }
                                    ui.label("Z:");
                                    if ui
                                        .add(DragValue::new(&mut point.position.z).speed(0.01))
                                        .changed()
                                    {
                                        ctx.joint_point_actions
                                            .push(JointPointAction::Update(point.clone()));
                                        changed = true;
                                    }
                                });

                                // Joint type
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    egui::ComboBox::from_id_salt(format!(
                                        "joint_type_{}",
                                        point_id
                                    ))
                                    .selected_text(point.joint_type.display_name())
                                    .show_ui(ui, |ui| {
                                        for jt in JointType::all() {
                                            if ui
                                                .selectable_value(
                                                    &mut point.joint_type,
                                                    *jt,
                                                    jt.display_name(),
                                                )
                                                .changed()
                                            {
                                                ctx.joint_point_actions
                                                    .push(JointPointAction::Update(point.clone()));
                                                changed = true;
                                            }
                                        }
                                    });
                                });

                                // Axis (for revolute/continuous/prismatic)
                                if point.joint_type.has_axis() {
                                    ui.horizontal(|ui| {
                                        ui.label("Axis:");
                                        if ui.selectable_label(point.axis == Vec3::X, "X").clicked()
                                        {
                                            point.axis = Vec3::X;
                                            ctx.joint_point_actions
                                                .push(JointPointAction::Update(point.clone()));
                                            changed = true;
                                        }
                                        if ui.selectable_label(point.axis == Vec3::Y, "Y").clicked()
                                        {
                                            point.axis = Vec3::Y;
                                            ctx.joint_point_actions
                                                .push(JointPointAction::Update(point.clone()));
                                            changed = true;
                                        }
                                        if ui.selectable_label(point.axis == Vec3::Z, "Z").clicked()
                                        {
                                            point.axis = Vec3::Z;
                                            ctx.joint_point_actions
                                                .push(JointPointAction::Update(point.clone()));
                                            changed = true;
                                        }
                                    });
                                }

                                // Limits (for revolute/prismatic)
                                if point.joint_type.has_limits() {
                                    // Initialize limits if needed
                                    if point.limits.is_none() {
                                        point.limits = Some(JointLimits::default());
                                    }
                                    // Work with a copy to avoid borrow issues
                                    let mut limits_copy = point.limits.unwrap();
                                    let mut limits_changed = false;

                                    ui.collapsing("Limits", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Lower:");
                                            if ui
                                                .add(
                                                    DragValue::new(&mut limits_copy.lower)
                                                        .speed(0.01),
                                                )
                                                .changed()
                                            {
                                                limits_changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Upper:");
                                            if ui
                                                .add(
                                                    DragValue::new(&mut limits_copy.upper)
                                                        .speed(0.01),
                                                )
                                                .changed()
                                            {
                                                limits_changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Effort:");
                                            if ui
                                                .add(
                                                    DragValue::new(&mut limits_copy.effort)
                                                        .speed(1.0),
                                                )
                                                .changed()
                                            {
                                                limits_changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Velocity:");
                                            if ui
                                                .add(
                                                    DragValue::new(&mut limits_copy.velocity)
                                                        .speed(0.1),
                                                )
                                                .changed()
                                            {
                                                limits_changed = true;
                                            }
                                        });
                                    });

                                    if limits_changed {
                                        point.limits = Some(limits_copy);
                                        ctx.joint_point_actions
                                            .push(JointPointAction::Update(point.clone()));
                                        changed = true;
                                    }
                                } else {
                                    point.limits = None;
                                }

                                // Delete button
                                ui.separator();
                                if ui.button("Delete").clicked() {
                                    point_to_remove = Some(point_id);
                                }
                            });

                        if header.header_response.clicked() {
                            self.pending_select = Some(idx);
                        }
                    });
                }
            });

        // Handle remove action
        if let Some(point_id) = point_to_remove {
            ctx.joint_point_actions
                .push(JointPointAction::Remove(point_id));
            changed = true;
        }

        if ctx.joint_points.is_empty() {
            ui.weak("No joint points defined.\nClick 'Add Joint Point' to create one.");
        }

        changed
    }
}
