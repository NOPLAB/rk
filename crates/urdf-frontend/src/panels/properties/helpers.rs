//! Common UI helper functions for property components

use egui::{DragValue, Ui};

/// Render a labeled XYZ vector3 row with drag values
/// Returns true if any value was changed
pub fn vector3_row(ui: &mut Ui, label: &str, values: &mut [f32; 3], speed: f32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        let mut changed = false;
        ui.label("X");
        changed |= ui
            .add(DragValue::new(&mut values[0]).speed(speed))
            .changed();
        ui.label("Y");
        changed |= ui
            .add(DragValue::new(&mut values[1]).speed(speed))
            .changed();
        ui.label("Z");
        changed |= ui
            .add(DragValue::new(&mut values[2]).speed(speed))
            .changed();
        changed
    })
    .inner
}

/// Render a labeled XYZ vector3 row (read-only)
pub fn vector3_readonly_row(ui: &mut Ui, label: &str, values: &[f32; 3]) {
    ui.horizontal(|ui| {
        ui.label(label);
    });
    ui.horizontal(|ui| {
        ui.label("X");
        ui.weak(format!("{:.3}", values[0]));
        ui.label("Y");
        ui.weak(format!("{:.3}", values[1]));
        ui.label("Z");
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
        ui.label("X");
        changed |= ui
            .add(DragValue::new(&mut rot_deg[0]).speed(speed).suffix("°"))
            .changed();
        ui.label("Y");
        changed |= ui
            .add(DragValue::new(&mut rot_deg[1]).speed(speed).suffix("°"))
            .changed();
        ui.label("Z");
        changed |= ui
            .add(DragValue::new(&mut rot_deg[2]).speed(speed).suffix("°"))
            .changed();
        changed
    })
    .inner
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
