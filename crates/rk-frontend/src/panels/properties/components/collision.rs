//! Collision component - collision element editing

use egui::{DragValue, Ui};

use rk_core::{GeometryType, Pose};

use crate::panels::properties::helpers::{rotation_row, vector3_row};
use crate::panels::properties::{PropertyComponent, PropertyContext};
use crate::state::AppAction;

/// Collision component for editing collision elements
pub struct CollisionComponent {
    /// Whether the component is open
    is_open: bool,
}

impl CollisionComponent {
    pub fn new() -> Self {
        Self { is_open: true }
    }
}

impl Default for CollisionComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for CollisionComponent {
    fn name(&self) -> &str {
        "Collisions"
    }

    fn default_open(&self) -> bool {
        self.is_open
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        let Some(link_id) = ctx.link_id else {
            ui.weak("No link associated with this part");
            return false;
        };

        let mut changed = false;

        // Add collision button
        ui.horizontal(|ui| {
            ui.label(format!("{} collision(s)", ctx.collisions.len()));
            if ui.button("+ Add").clicked() {
                // Add a default box collision
                ctx.pending_actions.push(AppAction::AddCollision {
                    link_id,
                    geometry: GeometryType::Box {
                        size: [0.1, 0.1, 0.1],
                    },
                });
                changed = true;
            }
        });

        ui.add_space(4.0);

        // List all collisions
        let mut remove_index = None;
        for (index, collision) in ctx.collisions.iter().enumerate() {
            let is_selected = ctx.selected_collision_index == Some(index);
            let header_text = collision
                .name
                .clone()
                .unwrap_or_else(|| format!("Collision {}", index));

            // Selectable header
            let response = ui.selectable_label(is_selected, &header_text);
            if response.clicked() {
                if is_selected {
                    // Deselect
                    ctx.pending_actions.push(AppAction::SelectCollision(None));
                } else {
                    // Select
                    ctx.pending_actions
                        .push(AppAction::SelectCollision(Some((link_id, index))));
                }
                changed = true;
            }

            // Show details when selected
            if is_selected {
                ui.indent(format!("collision_{}", index), |ui| {
                    // Origin position
                    let mut pos = collision.origin.xyz;
                    if vector3_row(ui, "Position", &mut pos, 0.01) {
                        let origin = Pose::new(pos, collision.origin.rpy);
                        ctx.pending_actions.push(AppAction::UpdateCollisionOrigin {
                            link_id,
                            index,
                            origin,
                        });
                        changed = true;
                    }

                    // Origin rotation
                    let mut rot_deg = [
                        collision.origin.rpy[0].to_degrees(),
                        collision.origin.rpy[1].to_degrees(),
                        collision.origin.rpy[2].to_degrees(),
                    ];
                    if rotation_row(ui, "Rotation", &mut rot_deg, 1.0) {
                        let rpy = [
                            rot_deg[0].to_radians(),
                            rot_deg[1].to_radians(),
                            rot_deg[2].to_radians(),
                        ];
                        let origin = Pose::new(collision.origin.xyz, rpy);
                        ctx.pending_actions.push(AppAction::UpdateCollisionOrigin {
                            link_id,
                            index,
                            origin,
                        });
                        changed = true;
                    }

                    // Geometry type selector and parameters
                    ui.add_space(4.0);
                    if let Some(new_geometry) = render_geometry_editor(ui, &collision.geometry) {
                        ctx.pending_actions
                            .push(AppAction::UpdateCollisionGeometry {
                                link_id,
                                index,
                                geometry: new_geometry,
                            });
                        changed = true;
                    }

                    // Remove button
                    ui.add_space(4.0);
                    if ui.button("Remove").clicked() {
                        remove_index = Some(index);
                    }
                });
            }
        }

        // Handle removal
        if let Some(index) = remove_index {
            ctx.pending_actions
                .push(AppAction::RemoveCollision { link_id, index });
            changed = true;
        }

        changed
    }
}

/// Render geometry editor and return new geometry if changed
fn render_geometry_editor(ui: &mut Ui, geometry: &GeometryType) -> Option<GeometryType> {
    let mut changed = false;
    let mut new_geometry = geometry.clone();

    // Geometry type selector
    let current_type = match geometry {
        GeometryType::Box { .. } => "Box",
        GeometryType::Cylinder { .. } => "Cylinder",
        GeometryType::Sphere { .. } => "Sphere",
        GeometryType::Capsule { .. } => "Capsule",
        GeometryType::Mesh { .. } => "Mesh",
    };

    ui.horizontal(|ui| {
        ui.label("Geometry:");
        egui::ComboBox::from_id_salt("geometry_type")
            .selected_text(current_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(geometry, GeometryType::Box { .. }), "Box")
                    .clicked()
                {
                    new_geometry = GeometryType::Box {
                        size: [0.1, 0.1, 0.1],
                    };
                    changed = true;
                }
                if ui
                    .selectable_label(
                        matches!(geometry, GeometryType::Cylinder { .. }),
                        "Cylinder",
                    )
                    .clicked()
                {
                    new_geometry = GeometryType::Cylinder {
                        radius: 0.05,
                        length: 0.1,
                    };
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(geometry, GeometryType::Sphere { .. }), "Sphere")
                    .clicked()
                {
                    new_geometry = GeometryType::Sphere { radius: 0.05 };
                    changed = true;
                }
                if ui
                    .selectable_label(matches!(geometry, GeometryType::Capsule { .. }), "Capsule")
                    .clicked()
                {
                    new_geometry = GeometryType::Capsule {
                        radius: 0.05,
                        length: 0.1,
                    };
                    changed = true;
                }
            });
    });

    // Geometry-specific parameters
    match &mut new_geometry {
        GeometryType::Box { size } => {
            ui.horizontal(|ui| {
                ui.label("Size:");
            });
            ui.horizontal(|ui| {
                ui.label("X");
                if ui.add(DragValue::new(&mut size[0]).speed(0.01)).changed() {
                    changed = true;
                }
                ui.label("Y");
                if ui.add(DragValue::new(&mut size[1]).speed(0.01)).changed() {
                    changed = true;
                }
                ui.label("Z");
                if ui.add(DragValue::new(&mut size[2]).speed(0.01)).changed() {
                    changed = true;
                }
            });
        }
        GeometryType::Cylinder { radius, length } => {
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(DragValue::new(radius).speed(0.01)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Length:");
                if ui.add(DragValue::new(length).speed(0.01)).changed() {
                    changed = true;
                }
            });
        }
        GeometryType::Sphere { radius } => {
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(DragValue::new(radius).speed(0.01)).changed() {
                    changed = true;
                }
            });
        }
        GeometryType::Capsule { radius, length } => {
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(DragValue::new(radius).speed(0.01)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Length:");
                if ui.add(DragValue::new(length).speed(0.01)).changed() {
                    changed = true;
                }
            });
        }
        GeometryType::Mesh { path, .. } => {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.weak(path.as_deref().unwrap_or("(none)"));
            });
            ui.weak("Mesh geometry cannot be edited");
        }
    }

    if changed { Some(new_geometry) } else { None }
}
