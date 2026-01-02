//! Main application

use std::sync::Arc;

use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use parking_lot::Mutex;

use urdf_core::{load_stl_with_unit, Project};

use crate::app_state::{create_shared_state, AppAction, SharedAppState};
use crate::panels::{
    GraphPanel, HierarchyPanel, Panel, PartListPanel, PropertiesPanel, ViewportPanel,
};
use crate::viewport_state::{SharedViewportState, ViewportState};

/// Panel types
enum PanelType {
    Viewport(ViewportPanel),
    PartList(PartListPanel),
    Properties(PropertiesPanel),
    Hierarchy(HierarchyPanel),
    Graph(GraphPanel),
}

impl PanelType {
    fn name(&self) -> &str {
        match self {
            PanelType::Viewport(p) => p.name(),
            PanelType::PartList(p) => p.name(),
            PanelType::Properties(p) => p.name(),
            PanelType::Hierarchy(p) => p.name(),
            PanelType::Graph(p) => p.name(),
        }
    }
}

/// Tab viewer for dock area
struct UrdfTabViewer<'a> {
    app_state: &'a SharedAppState,
    render_state: Option<&'a egui_wgpu::RenderState>,
    viewport_state: &'a Option<SharedViewportState>,
}

impl TabViewer for UrdfTabViewer<'_> {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelType::Viewport(panel) => {
                if let (Some(render_state), Some(viewport_state)) =
                    (self.render_state, self.viewport_state)
                {
                    panel.ui_with_render_context(ui, self.app_state, render_state, viewport_state);
                } else {
                    panel.ui(ui, self.app_state);
                }
            }
            PanelType::PartList(panel) => panel.ui(ui, self.app_state),
            PanelType::Properties(panel) => panel.ui(ui, self.app_state),
            PanelType::Hierarchy(panel) => panel.ui(ui, self.app_state),
            PanelType::Graph(panel) => panel.ui(ui, self.app_state),
        }
    }
}

/// Main application
pub struct UrdfEditorApp {
    dock_state: DockState<PanelType>,
    app_state: SharedAppState,
    viewport_state: Option<SharedViewportState>,
}

impl UrdfEditorApp {
    /// Create a new app
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create viewport state if WGPU is available
        let viewport_state = cc.wgpu_render_state.as_ref().map(|render_state| {
            let device = render_state.device.clone();
            let queue = render_state.queue.clone();
            let format = render_state.target_format;

            Arc::new(Mutex::new(ViewportState::new(device, queue, format)))
        });

        // Create dock layout
        let dock_state = create_dock_layout();

        Self {
            dock_state,
            app_state: create_shared_state(),
            viewport_state,
        }
    }

    /// Process pending actions
    fn process_actions(&mut self) {
        let actions = self.app_state.lock().take_pending_actions();

        for action in actions {
            match action {
                AppAction::ImportStl(path) => {
                    let unit = self.app_state.lock().stl_import_unit;
                    match load_stl_with_unit(&path, unit) {
                        Ok(part) => {
                            tracing::info!("Loaded STL: {} ({} vertices, unit={:?})", part.name, part.vertices.len(), unit);

                            // Calculate bounding sphere for camera fit
                            let center = glam::Vec3::new(
                                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
                            );
                            let extent = glam::Vec3::new(
                                part.bbox_max[0] - part.bbox_min[0],
                                part.bbox_max[1] - part.bbox_min[1],
                                part.bbox_max[2] - part.bbox_min[2],
                            );
                            let radius = extent.length() / 2.0;

                            // Add to viewport
                            if let Some(ref viewport_state) = self.viewport_state {
                                tracing::info!("Adding part to viewport...");
                                let mut vp = viewport_state.lock();
                                vp.add_part(&part);
                                // Auto-fit camera to new part
                                vp.renderer.camera.fit_all(center, radius);
                                tracing::info!("Part added, camera fitted to center={:?}, radius={}", center, radius);
                            } else {
                                tracing::warn!("viewport_state is None - cannot add part to renderer");
                            }

                            // Add to app state
                            self.app_state.lock().add_part(part);
                        }
                        Err(e) => {
                            tracing::error!("Failed to load STL: {}", e);
                        }
                    }
                }
                AppAction::SelectPart(part_id) => {
                    self.app_state.lock().select_part(part_id);

                    // Update viewport selection (mesh highlighting)
                    if let Some(ref viewport_state) = self.viewport_state {
                        viewport_state.lock().set_selected_part(part_id);
                    }
                    // Overlays are updated in update_overlays() called after process_actions
                }
                AppAction::DeleteSelectedPart => {
                    let selected = self.app_state.lock().selected_part;
                    if let Some(id) = selected {
                        self.app_state.lock().remove_part(id);

                        if let Some(ref viewport_state) = self.viewport_state {
                            viewport_state.lock().remove_part(id);
                            viewport_state.lock().clear_overlays();
                        }
                    }
                }
                AppAction::UpdatePartTransform { part_id, transform } => {
                    if let Some(part) = self.app_state.lock().get_part_mut(part_id) {
                        part.origin_transform = transform;
                    }
                    if let Some(ref viewport_state) = self.viewport_state {
                        viewport_state.lock().update_part_transform(part_id, transform);
                    }
                }
                AppAction::NewProject => {
                    self.app_state.lock().new_project();
                    if let Some(ref viewport_state) = self.viewport_state {
                        viewport_state.lock().clear_parts();
                        viewport_state.lock().clear_overlays();
                    }
                }
                AppAction::SaveProject(path) => {
                    let mut state = self.app_state.lock();
                    let save_path = path.or(state.project_path.clone());

                    if let Some(ref path) = save_path {
                        // Sync parts to project
                        state.project.parts = state.parts.values().cloned().collect();

                        match state.project.save(path) {
                            Ok(()) => {
                                tracing::info!("Saved project to {:?}", path);
                                state.project_path = Some(path.clone());
                                state.modified = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to save project: {}", e);
                            }
                        }
                    }
                }
                AppAction::LoadProject(path) => {
                    match Project::load(&path) {
                        Ok(project) => {
                            tracing::info!("Loaded project: {}", project.name);

                            // Clear viewport
                            if let Some(ref viewport_state) = self.viewport_state {
                                viewport_state.lock().clear_parts();
                                viewport_state.lock().clear_overlays();
                            }

                            // Load parts into viewport
                            if let Some(ref viewport_state) = self.viewport_state {
                                for part in &project.parts {
                                    viewport_state.lock().add_part(part);
                                }
                            }

                            // Load into app state
                            self.app_state.lock().load_project(project, path);
                        }
                        Err(e) => {
                            tracing::error!("Failed to load project: {}", e);
                        }
                    }
                }
                AppAction::ExportUrdf(path) => {
                    let state = self.app_state.lock();
                    let options = urdf_core::ExportOptions {
                        output_dir: path,
                        robot_name: state.project.name.clone(),
                        mesh_prefix: "meshes".to_string(),
                        use_package_uri: false,
                    };

                    match urdf_core::export_urdf(&state.project.assembly, &state.parts, &options) {
                        Ok(_urdf) => {
                            tracing::info!("Exported URDF to {:?}", options.output_dir);
                        }
                        Err(e) => {
                            tracing::error!("Failed to export URDF: {}", e);
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unhandled action: {:?}", action);
                }
            }
        }
    }

    /// Update overlays based on current selection
    fn update_overlays(&mut self) {
        let Some(ref viewport_state) = self.viewport_state else {
            return;
        };

        let state = self.app_state.lock();
        if let Some(part_id) = state.selected_part {
            if let Some(part) = state.get_part(part_id) {
                let selected_point = state.selected_joint_point.map(|(_, idx)| idx);
                let part_clone = part.clone();
                drop(state);

                let mut vp = viewport_state.lock();
                vp.update_axes_for_part(&part_clone);
                vp.update_markers_for_part(&part_clone, selected_point);

                // Show gizmo based on selection
                if let Some(point_idx) = selected_point {
                    // Show gizmo at joint point position
                    vp.show_gizmo_for_joint_point(&part_clone, point_idx);
                } else {
                    // Show gizmo at part center
                    vp.show_gizmo_for_part(&part_clone);
                }
            }
        } else {
            // No selection - clear overlays
            viewport_state.lock().clear_overlays();
        }
    }
}

impl eframe::App for UrdfEditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process pending actions
        self.process_actions();

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.app_state.lock().queue_action(AppAction::NewProject);
                        ui.close_menu();
                    }
                    if ui.button("Open Project...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF Project", &["ron"])
                            .pick_file()
                        {
                            self.app_state.lock().queue_action(AppAction::LoadProject(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        self.app_state.lock().queue_action(AppAction::SaveProject(None));
                        ui.close_menu();
                    }
                    if ui.button("Save Project As...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF Project", &["ron"])
                            .save_file()
                        {
                            self.app_state.lock().queue_action(AppAction::SaveProject(Some(path)));
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Import STL...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("STL files", &["stl", "STL"])
                            .pick_file()
                        {
                            self.app_state.lock().queue_action(AppAction::ImportStl(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export URDF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.app_state.lock().queue_action(AppAction::ExportUrdf(path));
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Delete Selected").clicked() {
                        self.app_state.lock().queue_action(AppAction::DeleteSelectedPart);
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Reset Layout").clicked() {
                        self.dock_state = create_dock_layout();
                        ui.close_menu();
                    }
                });
            });
        });

        // Dock area
        let render_state = frame.wgpu_render_state();

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(
                ctx,
                &mut UrdfTabViewer {
                    app_state: &self.app_state,
                    render_state,
                    viewport_state: &self.viewport_state,
                },
            );

        // Update overlays when selection changes
        self.update_overlays();
    }
}

/// Create the default dock layout
fn create_dock_layout() -> DockState<PanelType> {
    let mut dock_state = DockState::new(vec![PanelType::Viewport(ViewportPanel::new())]);

    // Get the main surface
    let surface = dock_state.main_surface_mut();

    // Split right for properties (now includes joint points)
    let [_viewport, _right] = surface.split_right(
        NodeIndex::root(),
        0.75,
        vec![PanelType::Properties(PropertiesPanel::new())],
    );

    // Split left for parts list
    let [left, _viewport] = surface.split_left(
        NodeIndex::root(),
        0.2,
        vec![PanelType::PartList(PartListPanel::new())],
    );

    // Add hierarchy below parts
    surface.split_below(
        left,
        0.6,
        vec![PanelType::Hierarchy(HierarchyPanel::new())],
    );

    // Split bottom for graph
    surface.split_below(
        NodeIndex::root(),
        0.7,
        vec![PanelType::Graph(GraphPanel::new())],
    );

    dock_state
}
