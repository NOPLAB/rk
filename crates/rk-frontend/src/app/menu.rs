//! Menu bar rendering

use crate::state::{AppAction, SharedAppState};

/// Render the menu bar and return any triggered action
pub fn render_menu_bar(ctx: &egui::Context, app_state: &SharedAppState) -> Option<MenuAction> {
    let mut menu_action = None;

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app_state.lock().queue_action(AppAction::NewProject);
                    ui.close();
                }
                {
                    if ui.button("Open Project...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RK Project", &["rk"])
                            .pick_file()
                        {
                            app_state.lock().queue_action(AppAction::LoadProject(path));
                        }
                        ui.close();
                    }
                    if ui.button("Save Project").clicked() {
                        app_state.lock().queue_action(AppAction::SaveProject(None));
                        ui.close();
                    }
                    if ui.button("Save Project As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RK Project", &["rk"])
                            .save_file()
                        {
                            app_state
                                .lock()
                                .queue_action(AppAction::SaveProject(Some(path)));
                        }
                        ui.close();
                    }
                    ui.separator();
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
                    if ui.button("Import URDF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF", &["urdf", "xacro", "xml"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            app_state.lock().queue_action(AppAction::ImportUrdf(path));
                        }
                        ui.close();
                    }
                    if ui.button("Export URDF...").clicked() {
                        let default_name = app_state.lock().project.name.clone();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF", &["urdf"])
                            .set_file_name(format!("{}.urdf", default_name))
                            .save_file()
                        {
                            // Extract robot name from file name (without extension)
                            let robot_name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("robot")
                                .to_string();
                            // Use parent directory as output dir
                            let output_dir = path
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| std::path::PathBuf::from("."));
                            app_state.lock().queue_action(AppAction::ExportUrdf {
                                path: output_dir,
                                robot_name,
                            });
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });

            ui.menu_button("Edit", |ui| {
                let can_undo = app_state.lock().history.can_undo();
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo  Ctrl+Z"))
                    .clicked()
                {
                    app_state.lock().queue_action(AppAction::Undo);
                    ui.close();
                }

                let can_redo = app_state.lock().history.can_redo();
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo  Ctrl+Y"))
                    .clicked()
                {
                    app_state.lock().queue_action(AppAction::Redo);
                    ui.close();
                }

                ui.separator();

                if ui.button("Delete Selected").clicked() {
                    app_state.lock().queue_action(AppAction::DeleteSelectedPart);
                    ui.close();
                }
                ui.separator();
                if ui.button("Preferences...").clicked() {
                    menu_action = Some(MenuAction::OpenPreferences);
                    ui.close();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Reset Layout").clicked() {
                    menu_action = Some(MenuAction::ResetLayout);
                    ui.close();
                }
            });
        });
    });

    menu_action
}

/// Actions triggered by the menu
pub enum MenuAction {
    ResetLayout,
    OpenPreferences,
}
