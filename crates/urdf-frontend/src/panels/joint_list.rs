//! Joint list panel with sliders for controlling joint positions

use egui::Ui;

use urdf_core::JointType;

use crate::panels::Panel;
use crate::state::{AngleDisplayMode, AppAction, SharedAppState};

/// Joint list panel for controlling joint positions
pub struct JointListPanel {
    // Panel has no persistent state - angle mode is in AppState
}

impl JointListPanel {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for JointListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for JointListPanel {
    fn name(&self) -> &str {
        "Joints"
    }

    fn ui(&mut self, ui: &mut Ui, app_state: &SharedAppState) {
        let state = app_state.lock();
        let joints: Vec<_> = state.project.assembly.joints.values().cloned().collect();
        let joint_positions = state.joint_positions.clone();
        let angle_mode = state.angle_display_mode;
        drop(state);

        if joints.is_empty() {
            ui.weak("No joints in assembly.\nConnect parts to create joints.");
            return;
        }

        // Header with Reset All and unit toggle
        ui.horizontal(|ui| {
            if ui.button("Reset All").clicked() {
                app_state
                    .lock()
                    .queue_action(AppAction::ResetAllJointPositions);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mode_text = match angle_mode {
                    AngleDisplayMode::Degrees => "deg",
                    AngleDisplayMode::Radians => "rad",
                };
                if ui
                    .button(mode_text)
                    .on_hover_text("Toggle angle unit")
                    .clicked()
                {
                    app_state.lock().angle_display_mode.toggle();
                }
            });
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("joint_list_scroll")
            .show(ui, |ui| {
                for joint in &joints {
                    self.render_joint_control(ui, joint, &joint_positions, angle_mode, app_state);
                }
            });
    }
}

impl JointListPanel {
    fn render_joint_control(
        &self,
        ui: &mut Ui,
        joint: &urdf_core::Joint,
        joint_positions: &std::collections::HashMap<uuid::Uuid, f32>,
        angle_mode: AngleDisplayMode,
        app_state: &SharedAppState,
    ) {
        let current_value_rad = joint_positions.get(&joint.id).copied().unwrap_or(0.0);

        ui.push_id(joint.id, |ui| {
            // Joint name with type indicator
            ui.horizontal(|ui| {
                let type_label = match joint.joint_type {
                    JointType::Fixed => "[Fixed]",
                    JointType::Revolute => "[Rev]",
                    JointType::Continuous => "[Cont]",
                    JointType::Prismatic => "[Prism]",
                    JointType::Floating => "[Float]",
                    JointType::Planar => "[Planar]",
                };
                ui.label(format!("{} {}", type_label, joint.name));
            });

            match joint.joint_type {
                JointType::Fixed => {
                    // Fixed joints have no control - show disabled slider
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            false,
                            egui::Slider::new(&mut 0.0f32, 0.0..=0.0).text("fixed"),
                        );
                    });
                }
                JointType::Continuous => {
                    // Continuous: full rotation, use +/- 180 degrees or PI
                    let (min_display, max_display) = match angle_mode {
                        AngleDisplayMode::Degrees => (-180.0, 180.0),
                        AngleDisplayMode::Radians => (-std::f32::consts::PI, std::f32::consts::PI),
                    };
                    let mut display_value = angle_mode.from_radians(current_value_rad);

                    ui.horizontal(|ui| {
                        let slider =
                            egui::Slider::new(&mut display_value, min_display..=max_display)
                                .suffix(angle_mode.suffix())
                                .clamping(egui::SliderClamping::Never);
                        if ui.add(slider).changed() {
                            let new_rad = angle_mode.to_radians(display_value);
                            app_state
                                .lock()
                                .queue_action(AppAction::UpdateJointPosition {
                                    joint_id: joint.id,
                                    position: new_rad,
                                });
                        }
                        if ui.button("R").on_hover_text("Reset to 0").clicked() {
                            app_state
                                .lock()
                                .queue_action(AppAction::ResetJointPosition { joint_id: joint.id });
                        }
                    });
                }
                JointType::Revolute => {
                    // Revolute: respect limits
                    let (lower_rad, upper_rad) = joint
                        .limits
                        .as_ref()
                        .map(|l| (l.lower, l.upper))
                        .unwrap_or((-std::f32::consts::PI, std::f32::consts::PI));

                    let lower_display = angle_mode.from_radians(lower_rad);
                    let upper_display = angle_mode.from_radians(upper_rad);
                    let mut display_value = angle_mode.from_radians(current_value_rad);

                    ui.horizontal(|ui| {
                        let slider =
                            egui::Slider::new(&mut display_value, lower_display..=upper_display)
                                .suffix(angle_mode.suffix());
                        if ui.add(slider).changed() {
                            let new_rad = angle_mode.to_radians(display_value);
                            app_state
                                .lock()
                                .queue_action(AppAction::UpdateJointPosition {
                                    joint_id: joint.id,
                                    position: new_rad,
                                });
                        }
                        if ui.button("R").on_hover_text("Reset to 0").clicked() {
                            app_state
                                .lock()
                                .queue_action(AppAction::ResetJointPosition { joint_id: joint.id });
                        }
                    });
                }
                JointType::Prismatic => {
                    // Prismatic: linear motion with limits (in meters)
                    let (lower, upper) = joint
                        .limits
                        .as_ref()
                        .map(|l| (l.lower, l.upper))
                        .unwrap_or((-1.0, 1.0));

                    let mut value = current_value_rad; // For prismatic, this is meters

                    ui.horizontal(|ui| {
                        let slider = egui::Slider::new(&mut value, lower..=upper).suffix(" m");
                        if ui.add(slider).changed() {
                            app_state
                                .lock()
                                .queue_action(AppAction::UpdateJointPosition {
                                    joint_id: joint.id,
                                    position: value,
                                });
                        }
                        if ui.button("R").on_hover_text("Reset to 0").clicked() {
                            app_state
                                .lock()
                                .queue_action(AppAction::ResetJointPosition { joint_id: joint.id });
                        }
                    });
                }
                JointType::Floating | JointType::Planar => {
                    // These require multi-DOF controls - show as not implemented
                    ui.weak("(Multi-DOF control not implemented)");
                }
            }

            ui.add_space(4.0);
        });
    }
}
