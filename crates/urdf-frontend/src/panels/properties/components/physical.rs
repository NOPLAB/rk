//! Physical component - mass and inertia editing

use egui::{DragValue, Ui};

use crate::panels::properties::{PropertyComponent, PropertyContext};

/// Physical properties component (mass, inertia)
pub struct PhysicalComponent;

impl PhysicalComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PhysicalComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for PhysicalComponent {
    fn name(&self) -> &str {
        "Physical"
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        let mut changed = false;
        let part = &mut ctx.part;

        // Mass
        ui.horizontal(|ui| {
            ui.label("Mass (kg):");
            changed |= ui
                .add(
                    DragValue::new(&mut part.mass)
                        .speed(0.01)
                        .range(0.001..=1000.0),
                )
                .changed();
        });

        // Inertia (collapsible subsection)
        ui.collapsing("Inertia", |ui| {
            ui.horizontal(|ui| {
                ui.label("Ixx:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.ixx).speed(0.0001))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Ixy:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.ixy).speed(0.0001))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Ixz:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.ixz).speed(0.0001))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Iyy:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.iyy).speed(0.0001))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Iyz:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.iyz).speed(0.0001))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Izz:");
                changed |= ui
                    .add(DragValue::new(&mut part.inertia.izz).speed(0.0001))
                    .changed();
            });

            if ui.button("Auto-calculate from mesh").clicked() {
                part.inertia = urdf_core::InertiaMatrix::from_bounding_box(
                    part.mass,
                    part.bbox_min,
                    part.bbox_max,
                );
                changed = true;
            }
        });

        changed
    }
}
