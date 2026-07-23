//! Toolbar and context menus for part list

use rk_core::StlUnit;
use rk_engine::Command;

use crate::state::{AppAction, PrimitiveType, SharedAppState};

/// Render the global unit selector
pub fn render_unit_selector(ui: &mut egui::Ui, app_state: &SharedAppState) {
    ui.horizontal(|ui| {
        ui.label("Unit:");
        let mut state = app_state.lock();
        let current_unit = state.stl_import_unit;
        egui::ComboBox::from_id_salt("stl_unit")
            .selected_text(current_unit.name())
            .show_ui(ui, |ui| {
                for unit in StlUnit::ALL {
                    ui.selectable_value(&mut state.stl_import_unit, *unit, unit.name());
                }
            });
    });
}

/// Show context menu for creating new objects
pub fn show_tree_context_menu(ui: &mut egui::Ui, app_state: &SharedAppState) {
    ui.menu_button("Import Parts", |ui| {
        for (label, filter_name, exts) in [
            ("STL...", "STL files", &["stl", "STL"][..]),
            ("OBJ...", "OBJ files", &["obj", "OBJ"][..]),
            ("DAE (COLLADA)...", "DAE files", &["dae", "DAE"][..]),
        ] {
            if ui.button(label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(filter_name, exts)
                    .pick_file()
                {
                    let mut state = app_state.lock();
                    let unit = state.stl_import_unit;
                    state.queue_action(AppAction::Cmd(Command::ImportMesh { path, unit }));
                }
                ui.close();
            }
        }
    });

    ui.separator();

    // Create Primitives submenu
    ui.menu_button("Create Primitives", |ui| {
        for primitive_type in [
            PrimitiveType::Box,
            PrimitiveType::Cylinder,
            PrimitiveType::Sphere,
        ] {
            if ui.button(primitive_type.name()).clicked() {
                app_state
                    .lock()
                    .queue_action(AppAction::Cmd(Command::CreatePrimitive {
                        id: None,
                        primitive: primitive_type.to_spec(),
                        name: None,
                    }));
                ui.close();
            }
        }
    });

    // Create Empty
    if ui.button("Create Empty...").clicked() {
        app_state
            .lock()
            .queue_action(AppAction::Cmd(Command::CreateEmptyPart {
                id: None,
                name: None,
            }));
        ui.close();
    }
}
