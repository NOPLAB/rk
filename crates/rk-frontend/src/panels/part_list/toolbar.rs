//! Toolbar and context menus for part list

use rk_core::StlUnit;

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
    // Import Parts submenu (native)
    #[cfg(not(target_arch = "wasm32"))]
    ui.menu_button("Import Parts", |ui| {
        if ui.button("STL...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STL files", &["stl", "STL"])
                .pick_file()
            {
                app_state.lock().queue_action(AppAction::ImportMesh(path));
            }
            ui.close();
        }
        if ui.button("OBJ...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OBJ files", &["obj", "OBJ"])
                .pick_file()
            {
                app_state.lock().queue_action(AppAction::ImportMesh(path));
            }
            ui.close();
        }
        if ui.button("DAE (COLLADA)...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("DAE files", &["dae", "DAE"])
                .pick_file()
            {
                app_state.lock().queue_action(AppAction::ImportMesh(path));
            }
            ui.close();
        }
    });

    // Import Parts submenu (WASM)
    #[cfg(target_arch = "wasm32")]
    ui.menu_button("Import Parts", |ui| {
        if ui.button("STL...").clicked() {
            let app_state = app_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("STL files", &["stl", "STL"])
                    .pick_file()
                    .await
                {
                    let filename = file.file_name();
                    let data = file.read().await;
                    app_state.lock().queue_action(AppAction::ImportMeshBytes {
                        name: filename,
                        data,
                    });
                }
            });
            ui.close();
        }
        if ui.button("OBJ...").clicked() {
            let app_state = app_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("OBJ files", &["obj", "OBJ"])
                    .pick_file()
                    .await
                {
                    let filename = file.file_name();
                    let data = file.read().await;
                    app_state.lock().queue_action(AppAction::ImportMeshBytes {
                        name: filename,
                        data,
                    });
                }
            });
            ui.close();
        }
        if ui.button("DAE (COLLADA)...").clicked() {
            let app_state = app_state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("DAE files", &["dae", "DAE"])
                    .pick_file()
                    .await
                {
                    let filename = file.file_name();
                    let data = file.read().await;
                    app_state.lock().queue_action(AppAction::ImportMeshBytes {
                        name: filename,
                        data,
                    });
                }
            });
            ui.close();
        }
    });

    ui.separator();

    // Create Primitives submenu
    ui.menu_button("Create Primitives", |ui| {
        if ui.button("Box").clicked() {
            app_state.lock().queue_action(AppAction::CreatePrimitive {
                primitive_type: PrimitiveType::Box,
                name: None,
            });
            ui.close();
        }
        if ui.button("Cylinder").clicked() {
            app_state.lock().queue_action(AppAction::CreatePrimitive {
                primitive_type: PrimitiveType::Cylinder,
                name: None,
            });
            ui.close();
        }
        if ui.button("Sphere").clicked() {
            app_state.lock().queue_action(AppAction::CreatePrimitive {
                primitive_type: PrimitiveType::Sphere,
                name: None,
            });
            ui.close();
        }
    });

    // Create Empty
    if ui.button("Create Empty...").clicked() {
        app_state
            .lock()
            .queue_action(AppAction::CreateEmpty { name: None });
        ui.close();
    }
}
