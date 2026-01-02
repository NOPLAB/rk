//! Main application

use std::sync::Arc;

use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use parking_lot::Mutex;

use urdf_core::{
    generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh,
    import_urdf, load_stl_with_unit, ImportOptions, Joint, JointPoint, Link, Part, Pose, Project,
};

use crate::app_state::{create_shared_state, AppAction, PrimitiveType, SharedAppState};
use crate::panels::{Panel, PartListPanel, PropertiesPanel, ViewportPanel};
use crate::viewport_state::{SharedViewportState, ViewportState};

/// Panel types
enum PanelType {
    Viewport(ViewportPanel),
    PartList(PartListPanel),
    Properties(PropertiesPanel),
}

impl PanelType {
    fn name(&self) -> &str {
        match self {
            PanelType::Viewport(p) => p.name(),
            PanelType::PartList(p) => p.name(),
            PanelType::Properties(p) => p.name(),
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
                AppAction::ExportUrdf { path, robot_name } => {
                    let state = self.app_state.lock();
                    let options = urdf_core::ExportOptions {
                        output_dir: path,
                        robot_name,
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
                AppAction::ConnectParts { parent, child } => {
                    let mut state = self.app_state.lock();

                    // Helper to find or create a link for a part
                    let find_or_create_link = |state: &mut crate::app_state::AppState, part_id: uuid::Uuid| -> Option<uuid::Uuid> {
                        // Check if link already exists for this part
                        if let Some((link_id, _)) = state.project.assembly.links.iter().find(|(_, l)| l.part_id == Some(part_id)) {
                            return Some(*link_id);
                        }
                        // Create new link
                        if let Some(part) = state.parts.get(&part_id) {
                            let link = Link::from_part(part);
                            let link_id = state.project.assembly.add_link(link);
                            Some(link_id)
                        } else {
                            None
                        }
                    };

                    // Get or create links
                    let parent_link_id = find_or_create_link(&mut state, parent);
                    let child_link_id = find_or_create_link(&mut state, child);

                    if let (Some(parent_link_id), Some(child_link_id)) = (parent_link_id, child_link_id) {
                        // Disconnect child from existing parent first
                        if state.project.assembly.parent.contains_key(&child_link_id) {
                            if let Err(e) = state.project.assembly.disconnect(child_link_id) {
                                tracing::warn!("Failed to disconnect existing parent: {}", e);
                            }
                        }

                        // Get names first to avoid borrow issues
                        let child_name_for_jp = state.parts.get(&child).map(|p| p.name.clone()).unwrap_or_else(|| "child".to_string());
                        let parent_name_for_jp = state.parts.get(&parent).map(|p| p.name.clone()).unwrap_or_else(|| "parent".to_string());

                        // Create joint point on parent part (at center)
                        let parent_jp_id = if let Some(part) = state.parts.get_mut(&parent) {
                            let center = glam::Vec3::new(
                                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
                            );
                            let jp = JointPoint::new(format!("joint_to_{}", child_name_for_jp), center);
                            let jp_id = jp.id;
                            part.joint_points.push(jp);
                            Some(jp_id)
                        } else {
                            None
                        };

                        // Create joint point on child part (at center)
                        let child_jp_id = if let Some(part) = state.parts.get_mut(&child) {
                            let center = glam::Vec3::new(
                                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
                            );
                            let jp = JointPoint::new(format!("joint_from_{}", parent_name_for_jp), center);
                            let jp_id = jp.id;
                            part.joint_points.push(jp);
                            Some(jp_id)
                        } else {
                            None
                        };

                        // Get names for joint
                        let parent_name = state.project.assembly.links.get(&parent_link_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_default();
                        let child_name = state.project.assembly.links.get(&child_link_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_default();

                        // Create fixed joint
                        let mut joint = Joint::fixed(
                            format!("{}_to_{}", parent_name, child_name),
                            parent_link_id,
                            child_link_id,
                            Pose::default(),
                        );
                        joint.parent_joint_point = parent_jp_id;
                        joint.child_joint_point = child_jp_id;

                        match state.project.assembly.connect(parent_link_id, child_link_id, joint) {
                            Ok(joint_id) => {
                                tracing::info!("Connected {} to {} via joint {}", parent_name, child_name, joint_id);
                                state.modified = true;
                            }
                            Err(e) => {
                                tracing::error!("Failed to connect parts: {}", e);
                            }
                        }
                    }
                }
                AppAction::DisconnectPart { child } => {
                    let mut state = self.app_state.lock();

                    // Find the link for this part
                    let child_link_id = state.project.assembly.links.iter()
                        .find(|(_, l)| l.part_id == Some(child))
                        .map(|(id, _)| *id);

                    if let Some(link_id) = child_link_id {
                        match state.project.assembly.disconnect(link_id) {
                            Ok(joint) => {
                                tracing::info!("Disconnected part {}, removed joint {}", child, joint.name);
                                state.modified = true;
                            }
                            Err(e) => {
                                tracing::error!("Failed to disconnect part: {}", e);
                            }
                        }
                    }
                }
                AppAction::ConnectToBaseLink(part_id) => {
                    let mut state = self.app_state.lock();

                    // Get base_link id
                    let base_link_id = state.project.assembly.root_link;

                    if let Some(base_link_id) = base_link_id {
                        // Find or create link for this part
                        let child_link_id = state.project.assembly.links.iter()
                            .find(|(_, l)| l.part_id == Some(part_id))
                            .map(|(id, _)| *id);

                        let child_link_id = if let Some(id) = child_link_id {
                            // Disconnect from existing parent if any
                            if state.project.assembly.parent.contains_key(&id) {
                                let _ = state.project.assembly.disconnect(id);
                            }
                            id
                        } else {
                            // Create new link for this part
                            if let Some(part) = state.parts.get(&part_id) {
                                let link = Link::from_part(part);
                                state.project.assembly.add_link(link)
                            } else {
                                return;
                            }
                        };

                        // Get names for joint
                        let base_name = state.project.assembly.links.get(&base_link_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_else(|| "base_link".to_string());
                        let child_name = state.project.assembly.links.get(&child_link_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_default();

                        // Create fixed joint
                        let joint = Joint::fixed(
                            format!("{}_to_{}", base_name, child_name),
                            base_link_id,
                            child_link_id,
                            Pose::default(),
                        );

                        match state.project.assembly.connect(base_link_id, child_link_id, joint) {
                            Ok(joint_id) => {
                                tracing::info!("Connected {} to base_link via joint {}", child_name, joint_id);
                                state.modified = true;
                            }
                            Err(e) => {
                                tracing::error!("Failed to connect to base_link: {}", e);
                            }
                        }
                    }
                }
                AppAction::ImportUrdf(path) => {
                    let stl_unit = self.app_state.lock().stl_import_unit;
                    let options = ImportOptions {
                        base_dir: path.parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| std::path::PathBuf::from(".")),
                        stl_unit,
                        default_color: [0.7, 0.7, 0.7, 1.0],
                    };

                    match import_urdf(&path, &options) {
                        Ok(project) => {
                            tracing::info!(
                                "Imported URDF: {} ({} links, {} joints, {} parts)",
                                project.name,
                                project.assembly.links.len(),
                                project.assembly.joints.len(),
                                project.parts.len()
                            );

                            // Clear viewport
                            if let Some(ref viewport_state) = self.viewport_state {
                                viewport_state.lock().clear_parts();
                                viewport_state.lock().clear_overlays();
                            }

                            // Add parts to viewport and fit camera
                            if let Some(ref viewport_state) = self.viewport_state {
                                let mut total_center = glam::Vec3::ZERO;
                                let mut max_radius: f32 = 1.0;
                                let part_count = project.parts.len() as f32;

                                for part in &project.parts {
                                    viewport_state.lock().add_part(part);

                                    // Accumulate for camera fitting
                                    let center = glam::Vec3::new(
                                        (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                                        (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                                        (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
                                    );
                                    total_center += center;

                                    let extent = glam::Vec3::new(
                                        part.bbox_max[0] - part.bbox_min[0],
                                        part.bbox_max[1] - part.bbox_min[1],
                                        part.bbox_max[2] - part.bbox_min[2],
                                    );
                                    max_radius = max_radius.max(extent.length() / 2.0);
                                }

                                if part_count > 0.0 {
                                    total_center /= part_count;
                                    viewport_state.lock().renderer.camera.fit_all(total_center, max_radius * 2.0);
                                }
                            }

                            // Load into app state
                            self.app_state.lock().load_project(project, path);
                        }
                        Err(e) => {
                            tracing::error!("Failed to import URDF: {}", e);
                        }
                    }
                }
                AppAction::CreatePrimitive { primitive_type, name } => {
                    // Generate unique name
                    let existing_count = self.app_state.lock().parts.len();
                    let part_name = name.unwrap_or_else(|| {
                        format!("{}_{}", primitive_type.name(), existing_count + 1)
                    });

                    // Generate mesh based on primitive type (default size: 0.1m)
                    let (vertices, normals, indices) = match primitive_type {
                        PrimitiveType::Box => generate_box_mesh([0.1, 0.1, 0.1]),
                        PrimitiveType::Cylinder => generate_cylinder_mesh(0.05, 0.1),
                        PrimitiveType::Sphere => generate_sphere_mesh(0.05),
                    };

                    // Create part
                    let mut part = Part::new(&part_name);
                    part.vertices = vertices;
                    part.normals = normals;
                    part.indices = indices;
                    part.calculate_bounding_box();

                    // Set material name based on primitive type
                    part.material_name = Some(format!("{}_material", primitive_type.name().to_lowercase()));

                    tracing::info!(
                        "Created primitive: {} ({} vertices)",
                        part.name,
                        part.vertices.len()
                    );

                    // Add to viewport
                    if let Some(ref viewport_state) = self.viewport_state {
                        let mut vp = viewport_state.lock();
                        vp.add_part(&part);

                        // Fit camera if this is the first part
                        let center = part.center();
                        let radius = part.size().length() / 2.0;
                        vp.renderer.camera.fit_all(center, radius.max(0.5));
                    }

                    // Add to app state
                    self.app_state.lock().add_part(part);
                }
                AppAction::CreateEmpty { name } => {
                    // Generate unique name
                    let existing_count = self.app_state.lock().parts.len();
                    let part_name = name.unwrap_or_else(|| {
                        format!("Empty_{}", existing_count + 1)
                    });

                    // Create empty part (no geometry)
                    let part = Part::new(&part_name);

                    tracing::info!("Created empty part: {}", part.name);

                    // Add to app state (no viewport mesh for empty parts)
                    self.app_state.lock().add_part(part);
                }
                AppAction::AddJointPoint { .. } | AppAction::RemoveJointPoint { .. } => {
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
                    if ui.button("Import URDF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF files", &["urdf", "URDF", "xml"])
                            .pick_file()
                        {
                            self.app_state.lock().queue_action(AppAction::ImportUrdf(path));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export URDF...").clicked() {
                        let default_name = self.app_state.lock().project.name.clone();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("URDF", &["urdf"])
                            .set_file_name(format!("{}.urdf", default_name))
                            .save_file()
                        {
                            // Extract robot name from file name (without extension)
                            let robot_name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("robot")
                                .to_string();
                            // Use parent directory as output dir
                            let output_dir = path.parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| std::path::PathBuf::from("."));
                            self.app_state.lock().queue_action(AppAction::ExportUrdf {
                                path: output_dir,
                                robot_name,
                            });
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

    // Split right for properties
    let [_viewport, _right] = surface.split_right(
        NodeIndex::root(),
        0.75,
        vec![PanelType::Properties(PropertiesPanel::new())],
    );

    // Split left for parts list (now includes hierarchy via tree structure)
    let [_left, _viewport] = surface.split_left(
        NodeIndex::root(),
        0.2,
        vec![PanelType::PartList(PartListPanel::new())],
    );

    dock_state
}
