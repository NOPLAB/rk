//! Menu bar rendering

use rk_engine::Command;

use crate::state::{AppAction, SharedAppState};

/// Render the menu bar and return any triggered action
pub fn render_menu_bar(ctx: &egui::Context, app_state: &SharedAppState) -> Option<MenuAction> {
    let mut menu_action = None;

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app_state
                        .lock()
                        .queue_action(AppAction::Cmd(Command::NewDocument));
                    ui.close();
                }
                {
                    if ui.button("Open Project...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RK Project", &["rk"])
                            .pick_file()
                        {
                            app_state
                                .lock()
                                .queue_action(AppAction::Cmd(Command::LoadDocument { path }));
                        }
                        ui.close();
                    }
                    if ui.button("Save Project").clicked() {
                        app_state
                            .lock()
                            .queue_action(AppAction::Cmd(Command::SaveDocument { path: None }));
                        ui.close();
                    }
                    if ui.button("Save Project As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RK Project", &["rk"])
                            .save_file()
                        {
                            app_state
                                .lock()
                                .queue_action(AppAction::Cmd(Command::SaveDocument {
                                    path: Some(path),
                                }));
                        }
                        ui.close();
                    }
                    ui.separator();
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
                                    state.queue_action(AppAction::Cmd(Command::ImportMesh {
                                        path,
                                        unit,
                                    }));
                                }
                                ui.close();
                            }
                        }
                    });
                    if ui.button("Import URDF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF", &["urdf", "xacro", "xml"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            let mut state = app_state.lock();
                            let stl_unit = state.stl_import_unit;
                            state.queue_action(AppAction::Cmd(Command::ImportUrdf {
                                path,
                                stl_unit,
                            }));
                        }
                        ui.close();
                    }
                    if ui.button("Export URDF...").clicked() {
                        let engine = app_state.lock().engine.clone();
                        let default_name = engine.lock().project().name.clone();
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
                            app_state
                                .lock()
                                .queue_action(AppAction::Cmd(Command::ExportUrdf {
                                    path: output_dir,
                                    robot_name,
                                }));
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
                let engine = app_state.lock().engine.clone();
                let (can_undo, can_redo) = {
                    let eng = engine.lock();
                    (eng.can_undo(), eng.can_redo())
                };

                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo  Ctrl+Z"))
                    .clicked()
                {
                    app_state
                        .lock()
                        .queue_action(AppAction::Cmd(Command::Undo));
                    ui.close();
                }

                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo  Ctrl+Y"))
                    .clicked()
                {
                    app_state
                        .lock()
                        .queue_action(AppAction::Cmd(Command::Redo));
                    ui.close();
                }

                ui.separator();

                if ui.button("Delete Selected").clicked() {
                    let mut state = app_state.lock();
                    if let Some(part_id) = state.selected_part {
                        state.queue_action(AppAction::Cmd(Command::DeletePart { part_id }));
                    }
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
