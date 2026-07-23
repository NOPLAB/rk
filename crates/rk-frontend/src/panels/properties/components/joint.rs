//! Joint component - child joint configuration editing

use egui::{DragValue, Ui};
use glam::Vec3;

use rk_core::{JointLimits, JointType, Pose};

use crate::panels::properties::helpers::{rotation_row, vector3_row};
use crate::panels::properties::{PropertyComponent, PropertyContext};
use crate::state::AppAction;

/// Joint component for editing joints to child parts
pub struct JointComponent {
    /// Currently expanded joint index (if any)
    expanded_index: Option<usize>,
}

impl JointComponent {
    pub fn new() -> Self {
        Self {
            expanded_index: None,
        }
    }
}

impl Default for JointComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for JointComponent {
    fn name(&self) -> &str {
        "Child Joints"
    }

    fn default_open(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        if ctx.child_joints.is_empty() {
            ui.weak("No child joints");
            return false;
        }

        let mut changed = false;

        ui.label(format!("{} joint(s)", ctx.child_joints.len()));
        ui.add_space(4.0);

        for (index, info) in ctx.child_joints.iter().enumerate() {
            let is_expanded = self.expanded_index == Some(index);
            let header_text = format!("{} -> {}", info.joint.name, info.child_part_name);

            // Selectable header with joint type badge
            ui.horizontal(|ui| {
                let response = ui.selectable_label(is_expanded, &header_text);
                if response.clicked() {
                    if is_expanded {
                        self.expanded_index = None;
                        ctx.pending_actions.push(AppAction::SetEditingJoint(None));
                    } else {
                        self.expanded_index = Some(index);
                        ctx.pending_actions
                            .push(AppAction::SetEditingJoint(Some(info.joint_id)));
                    }
                }
                ui.weak(format!("[{}]", info.joint.joint_type.display_name()));
            });

            // Show details when expanded
            if is_expanded {
                ui.indent(format!("joint_{}", index), |ui| {
                    // Joint Type selector
                    let current_type = info.joint.joint_type;
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        egui::ComboBox::from_id_salt(format!("joint_type_{}", index))
                            .selected_text(current_type.display_name())
                            .show_ui(ui, |ui| {
                                for jt in JointType::all() {
                                    if ui
                                        .selectable_label(current_type == *jt, jt.display_name())
                                        .clicked()
                                    {
                                        ctx.pending_actions.push(AppAction::UpdateJointType {
                                            joint_id: info.joint_id,
                                            joint_type: *jt,
                                        });
                                        changed = true;
                                    }
                                }
                            });
                    });

                    ui.add_space(4.0);

                    // Origin position
                    let mut pos = info.joint.origin.xyz;
                    if vector3_row(ui, "Position", &mut pos, 0.01) {
                        let origin = Pose::new(pos, info.joint.origin.rpy);
                        ctx.pending_actions.push(AppAction::UpdateJointOrigin {
                            joint_id: info.joint_id,
                            origin,
                        });
                        changed = true;
                    }

                    // Origin rotation
                    let mut rot_deg = [
                        info.joint.origin.rpy[0].to_degrees(),
                        info.joint.origin.rpy[1].to_degrees(),
                        info.joint.origin.rpy[2].to_degrees(),
                    ];
                    if rotation_row(ui, "Rotation", &mut rot_deg, 1.0) {
                        let rpy = [
                            rot_deg[0].to_radians(),
                            rot_deg[1].to_radians(),
                            rot_deg[2].to_radians(),
                        ];
                        let origin = Pose::new(info.joint.origin.xyz, rpy);
                        ctx.pending_actions.push(AppAction::UpdateJointOrigin {
                            joint_id: info.joint_id,
                            origin,
                        });
                        changed = true;
                    }

                    // Axis (for revolute/prismatic/continuous)
                    if info.joint.joint_type.has_axis() {
                        ui.add_space(4.0);
                        let mut axis = [info.joint.axis.x, info.joint.axis.y, info.joint.axis.z];
                        if vector3_row(ui, "Axis", &mut axis, 0.1) {
                            let new_axis = Vec3::new(axis[0], axis[1], axis[2]);
                            // Normalize if not zero
                            let new_axis = if new_axis.length_squared() > 0.0001 {
                                new_axis.normalize()
                            } else {
                                Vec3::Z // Default to Z if zero
                            };
                            ctx.pending_actions.push(AppAction::UpdateJointAxis {
                                joint_id: info.joint_id,
                                axis: new_axis,
                            });
                            changed = true;
                        }
                    }

                    // Limits (for revolute/prismatic)
                    if info.joint.joint_type.has_limits() {
                        ui.add_space(4.0);
                        ui.label("Limits:");

                        let limits = info.joint.limits.unwrap_or_else(|| {
                            if info.joint.joint_type == JointType::Prismatic {
                                JointLimits::default_prismatic()
                            } else {
                                JointLimits::default_revolute()
                            }
                        });

                        let mut lower = limits.lower;
                        let mut upper = limits.upper;
                        let mut effort = limits.effort;
                        let mut velocity = limits.velocity;

                        // Convert to degrees for revolute joints
                        let is_revolute = info.joint.joint_type == JointType::Revolute;
                        if is_revolute {
                            lower = lower.to_degrees();
                            upper = upper.to_degrees();
                        }

                        let suffix = if is_revolute { "°" } else { " m" };
                        let speed = if is_revolute { 1.0 } else { 0.01 };

                        let mut limits_changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Lower:");
                            if ui
                                .add(DragValue::new(&mut lower).speed(speed).suffix(suffix))
                                .changed()
                            {
                                limits_changed = true;
                            }
                            ui.label("Upper:");
                            if ui
                                .add(DragValue::new(&mut upper).speed(speed).suffix(suffix))
                                .changed()
                            {
                                limits_changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Effort:");
                            if ui
                                .add(DragValue::new(&mut effort).speed(1.0).range(0.0..=10000.0))
                                .changed()
                            {
                                limits_changed = true;
                            }
                            ui.label("Velocity:");
                            if ui
                                .add(DragValue::new(&mut velocity).speed(0.1).range(0.0..=100.0))
                                .changed()
                            {
                                limits_changed = true;
                            }
                        });

                        if limits_changed {
                            // Convert back to radians for revolute joints
                            if is_revolute {
                                lower = lower.to_radians();
                                upper = upper.to_radians();
                            }
                            ctx.pending_actions.push(AppAction::UpdateJointLimits {
                                joint_id: info.joint_id,
                                limits: Some(JointLimits {
                                    lower,
                                    upper,
                                    effort,
                                    velocity,
                                }),
                            });
                            changed = true;
                        }
                    }
                });
            }
        }

        changed
    }
}
