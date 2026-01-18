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
                #[cfg(not(target_arch = "wasm32"))]
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
                #[cfg(target_arch = "wasm32")]
                {
                    if ui.button("Open Project...").clicked() {
                        let app_state = app_state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("RK Project", &["rk"])
                                .pick_file()
                                .await
                            {
                                let name = file.file_name();
                                let data = file.read().await;
                                app_state
                                    .lock()
                                    .queue_action(AppAction::LoadProjectBytes { name, data });
                            }
                        });
                        ui.close();
                    }
                    if ui.button("Save Project...").clicked() {
                        let app_state = app_state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            // Serialize project to bytes
                            let data = {
                                let state = app_state.lock();
                                // No sync needed - parts are stored directly in project
                                match state.project.to_bytes() {
                                    Ok(data) => data,
                                    Err(e) => {
                                        tracing::error!("Failed to serialize project: {}", e);
                                        return;
                                    }
                                }
                            };
                            let project_name = app_state.lock().project.name.clone();
                            let filename = format!("{}.rk", project_name);

                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("RK Project", &["rk"])
                                .set_file_name(&filename)
                                .save_file()
                                .await
                            {
                                if let Err(e) = file.write(&data).await {
                                    tracing::error!("Failed to save project: {:?}", e);
                                } else {
                                    tracing::info!("Project saved successfully");
                                }
                            }
                        });
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Import Parts", |ui| {
                        if ui.button("STL...").clicked() {
                            let app_state = app_state.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Some(file) = rfd::AsyncFileDialog::new()
                                    .add_filter("STL files", &["stl", "STL"])
                                    .pick_file()
                                    .await
                                {
                                    let name = file.file_name();
                                    // Remove .stl extension for part name
                                    let part_name = name
                                        .strip_suffix(".stl")
                                        .or_else(|| name.strip_suffix(".STL"))
                                        .unwrap_or(&name)
                                        .to_string();
                                    let data = file.read().await;
                                    app_state.lock().queue_action(AppAction::ImportMeshBytes {
                                        name: part_name,
                                        data,
                                    });
                                }
                            });
                            ui.close();
                        }
                    });
                    if ui.button("Export URDF...").clicked() {
                        let app_state = app_state.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            // Generate URDF string
                            let (urdf_content, robot_name) = {
                                let state = app_state.lock();
                                let robot_name = state.project.name.clone();
                                match rk_core::export_urdf_to_string(
                                    &state.project.assembly,
                                    state.project.parts(),
                                    &robot_name,
                                ) {
                                    Ok(urdf) => (urdf, robot_name),
                                    Err(e) => {
                                        tracing::error!("Failed to generate URDF: {}", e);
                                        return;
                                    }
                                }
                            };
                            let filename = format!("{}.urdf", robot_name);

                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("URDF", &["urdf"])
                                .set_file_name(&filename)
                                .save_file()
                                .await
                            {
                                if let Err(e) = file.write(urdf_content.as_bytes()).await {
                                    tracing::error!("Failed to export URDF: {:?}", e);
                                } else {
                                    tracing::info!("URDF exported successfully");
                                }
                            }
                        });
                        ui.close();
                    }
                }
            });

            ui.menu_button("Edit", |ui| {
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
