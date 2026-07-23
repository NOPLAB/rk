//! Common UI helper functions for property components

use egui::{DragValue, RichText, Ui};

use crate::theme::palette::{AXIS_X, AXIS_Y, AXIS_Z};

/// Render a labeled XYZ vector3 row with drag values
/// Returns true if any value was changed
pub fn vector3_row(ui: &mut Ui, label: &str, values: &mut [f32; 3], speed: f32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        let mut changed = false;
        ui.label(RichText::new("X").color(AXIS_X));
        changed |= ui
            .add(DragValue::new(&mut values[0]).speed(speed))
            .changed();
        ui.label(RichText::new("Y").color(AXIS_Y));
        changed |= ui
            .add(DragValue::new(&mut values[1]).speed(speed))
            .changed();
        ui.label(RichText::new("Z").color(AXIS_Z));
        changed |= ui
            .add(DragValue::new(&mut values[2]).speed(speed))
            .changed();
        changed
    })
    .inner
}

/// Render a labeled XYZ vector3 row (read-only)
#[allow(dead_code)]
pub fn vector3_readonly_row(ui: &mut Ui, label: &str, values: &[f32; 3]) {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("X").color(AXIS_X));
        ui.weak(format!("{:.3}", values[0]));
        ui.label(RichText::new("Y").color(AXIS_Y));
        ui.weak(format!("{:.3}", values[1]));
        ui.label(RichText::new("Z").color(AXIS_Z));
        ui.weak(format!("{:.3}", values[2]));
    });
}

/// Render rotation row with degree suffix
/// Returns true if any value was changed
pub fn rotation_row(ui: &mut Ui, label: &str, rot_deg: &mut [f32; 3], speed: f32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        let mut changed = false;
        ui.label(RichText::new("X").color(AXIS_X));
        changed |= ui
            .add(DragValue::new(&mut rot_deg[0]).speed(speed).suffix("°"))
            .changed();
        ui.label(RichText::new("Y").color(AXIS_Y));
        changed |= ui
            .add(DragValue::new(&mut rot_deg[1]).speed(speed).suffix("°"))
            .changed();
        ui.label(RichText::new("Z").color(AXIS_Z));
        changed |= ui
            .add(DragValue::new(&mut rot_deg[2]).speed(speed).suffix("°"))
            .changed();
        changed
    })
    .inner
}

/// Render rotation row (read-only) with degree suffix
#[allow(dead_code)]
pub fn rotation_readonly_row(ui: &mut Ui, label: &str, rot_deg: &[f32; 3]) {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("X").color(AXIS_X));
        ui.weak(format!("{:.1}°", rot_deg[0]));
        ui.label(RichText::new("Y").color(AXIS_Y));
        ui.weak(format!("{:.1}°", rot_deg[1]));
        ui.label(RichText::new("Z").color(AXIS_Z));
        ui.weak(format!("{:.1}°", rot_deg[2]));
    });
}

/// Render a labeled drag value
/// Returns true if the value was changed
#[allow(dead_code)]
pub fn labeled_drag_value(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    speed: f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(DragValue::new(value).speed(speed).range(range))
            .changed()
    })
    .inner
}
