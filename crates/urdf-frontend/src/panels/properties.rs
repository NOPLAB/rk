//! Properties panel

use crate::app_state::SharedAppState;
use crate::panels::Panel;

/// Properties panel for editing selected part
pub struct PropertiesPanel;

impl PropertiesPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PropertiesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for PropertiesPanel {
    fn name(&self) -> &str {
        "Properties"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        let mut state = app_state.lock();

        let Some(selected_id) = state.selected_part else {
            ui.weak("No part selected");
            return;
        };

        let Some(part) = state.parts.get_mut(&selected_id) else {
            ui.weak("Selected part not found");
            return;
        };

        ui.heading("Part Properties");
        ui.separator();

        // Name
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut part.name);
        });

        ui.separator();

        // Physical properties
        ui.heading("Physical");

        ui.horizontal(|ui| {
            ui.label("Mass (kg):");
            ui.add(egui::DragValue::new(&mut part.mass).speed(0.01).range(0.001..=1000.0));
        });

        ui.collapsing("Inertia", |ui| {
            ui.horizontal(|ui| {
                ui.label("Ixx:");
                ui.add(egui::DragValue::new(&mut part.inertia.ixx).speed(0.0001));
            });
            ui.horizontal(|ui| {
                ui.label("Ixy:");
                ui.add(egui::DragValue::new(&mut part.inertia.ixy).speed(0.0001));
            });
            ui.horizontal(|ui| {
                ui.label("Ixz:");
                ui.add(egui::DragValue::new(&mut part.inertia.ixz).speed(0.0001));
            });
            ui.horizontal(|ui| {
                ui.label("Iyy:");
                ui.add(egui::DragValue::new(&mut part.inertia.iyy).speed(0.0001));
            });
            ui.horizontal(|ui| {
                ui.label("Iyz:");
                ui.add(egui::DragValue::new(&mut part.inertia.iyz).speed(0.0001));
            });
            ui.horizontal(|ui| {
                ui.label("Izz:");
                ui.add(egui::DragValue::new(&mut part.inertia.izz).speed(0.0001));
            });

            if ui.button("Auto-calculate from mesh").clicked() {
                part.inertia = urdf_core::InertiaMatrix::from_bounding_box(
                    part.mass,
                    part.bbox_min,
                    part.bbox_max,
                );
            }
        });

        ui.separator();

        // Visual properties
        ui.heading("Visual");

        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut color = egui::Color32::from_rgba_unmultiplied(
                (part.color[0] * 255.0) as u8,
                (part.color[1] * 255.0) as u8,
                (part.color[2] * 255.0) as u8,
                (part.color[3] * 255.0) as u8,
            );
            if ui.color_edit_button_srgba(&mut color).changed() {
                part.color = [
                    color.r() as f32 / 255.0,
                    color.g() as f32 / 255.0,
                    color.b() as f32 / 255.0,
                    color.a() as f32 / 255.0,
                ];
            }
        });

        ui.horizontal(|ui| {
            ui.label("Material:");
            let mut material_name = part.material_name.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut material_name).changed() {
                part.material_name = if material_name.is_empty() {
                    None
                } else {
                    Some(material_name)
                };
            }
        });

        ui.separator();

        // Geometry info
        ui.heading("Geometry");
        ui.label(format!("Vertices: {}", part.vertices.len()));
        ui.label(format!("Triangles: {}", part.indices.len() / 3));
        ui.label(format!(
            "Bounding Box: [{:.3}, {:.3}, {:.3}] to [{:.3}, {:.3}, {:.3}]",
            part.bbox_min[0], part.bbox_min[1], part.bbox_min[2],
            part.bbox_max[0], part.bbox_max[1], part.bbox_max[2]
        ));

        let size = part.size();
        ui.label(format!("Size: {:.3} x {:.3} x {:.3}", size.x, size.y, size.z));

        if let Some(ref path) = part.stl_path {
            ui.label(format!("STL: {}", path));
        }
    }
}
