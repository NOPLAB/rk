//! Menu bar rendering

use crate::state::{AppAction, SharedAppState};

/// Download bytes as a file in the browser (WASM only)
#[cfg(target_arch = "wasm32")]
fn download_bytes(data: &[u8], filename: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    // Create Uint8Array from bytes
    let uint8_array = js_sys::Uint8Array::from(data);
    let array = js_sys::Array::new();
    array.push(&uint8_array);

    // Create Blob
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &options)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Create object URL
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    // Create anchor element and trigger download
    let anchor = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor: {:?}", e))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "Failed to cast to anchor")?;

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    // Revoke URL to free memory
    let _ = web_sys::Url::revoke_object_url(&url);

    Ok(())
}

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
                    if ui.button("Save Project").clicked() {
                        // Serialize project to bytes
                        let data = {
                            let state = app_state.lock();
                            match state.project.to_bytes() {
                                Ok(data) => data,
                                Err(e) => {
                                    tracing::error!("Failed to serialize project: {}", e);
                                    ui.close();
                                    return;
                                }
                            }
                        };
                        let project_name = app_state.lock().project.name.clone();
                        let filename = format!("{}.rk", project_name);

                        // Download file directly without dialog
                        if let Err(e) = download_bytes(&data, &filename) {
                            tracing::error!("Failed to download project: {}", e);
                        } else {
                            tracing::info!("Project download started");
                        }
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
