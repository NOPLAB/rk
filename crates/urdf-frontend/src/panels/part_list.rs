//! Part list panel with hierarchical tree structure and drag-and-drop

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use urdf_core::StlUnit;

use crate::app_state::{AppAction, PrimitiveType, SharedAppState};
use crate::panels::Panel;

/// Part list panel with drag-and-drop hierarchy
pub struct PartListPanel {
    /// Currently dragged part ID
    dragging_part: Option<Uuid>,
    /// Current drop target (part_id)
    drop_target: Option<Uuid>,
    /// Drop target is base_link
    drop_target_base_link: bool,
    /// Editing project name
    editing_project_name: bool,
    /// Temporary buffer for editing project name
    project_name_buffer: String,
    /// Editing base_link name
    editing_base_link_name: bool,
    /// Temporary buffer for editing base_link name
    base_link_name_buffer: String,
}

impl PartListPanel {
    pub fn new() -> Self {
        Self {
            dragging_part: None,
            drop_target: None,
            drop_target_base_link: false,
            editing_project_name: false,
            project_name_buffer: String::new(),
            editing_base_link_name: false,
            base_link_name_buffer: String::new(),
        }
    }
}

impl Default for PartListPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions collected during tree rendering
enum TreeAction {
    Select(Uuid),
    Delete(Uuid),
    Disconnect(Uuid),
    Connect { parent: Uuid, child: Uuid },
    ConnectToBaseLink(Uuid),
}

impl Panel for PartListPanel {
    fn name(&self) -> &str {
        "Parts"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        // Import STL toolbar
        ui.horizontal(|ui| {
            if ui.button("Import STL...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("STL files", &["stl", "STL"])
                    .pick_file()
                {
                    app_state.lock().queue_action(AppAction::ImportStl(path));
                }
            }

            ui.separator();

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

        ui.separator();

        // Collect state data
        let state = app_state.lock();
        let selected_id = state.selected_part;
        let project_name = state.project.name.clone();
        let base_link_name = state.project.assembly.base_link()
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "base_link".to_string());

        // Build tree structure from Assembly
        let (root_parts, children_map, parts_with_parent, orphaned_parts) = self.build_tree_structure(&state);

        // Collect part names for display
        let part_names: HashMap<Uuid, String> = state.parts.iter()
            .map(|(id, p)| (*id, p.name.clone()))
            .collect();

        let is_empty = state.parts.is_empty();
        drop(state);

        // Reset drop targets each frame
        self.drop_target = None;
        self.drop_target_base_link = false;

        // Collect actions during rendering
        let mut actions: Vec<TreeAction> = Vec::new();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Render project root node
            self.render_project_root(ui, &project_name, app_state);

            ui.add_space(4.0);

            // Render base_link node (renameable, drop target)
            self.render_base_link(ui, &base_link_name, app_state);

            // Render connected parts (tree structure) under base_link
            // These are children of base_link in the assembly
            for root_id in &root_parts {
                self.render_part_tree(
                    ui,
                    *root_id,
                    &part_names,
                    &children_map,
                    &parts_with_parent,
                    selected_id,
                    2, // Start at depth 2 (under base_link)
                    &mut actions,
                );
            }

            // Render orphaned parts (not connected to base_link)
            if !orphaned_parts.is_empty() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("Unconnected").weak().italics());
                });

                for part_id in &orphaned_parts {
                    if let Some(name) = part_names.get(part_id) {
                        self.render_orphan_part(
                            ui,
                            *part_id,
                            name,
                            selected_id,
                            &mut actions,
                        );
                    }
                }
            }

            // Empty space area for context menu (right-click on empty space)
            let remaining = ui.available_size();
            if remaining.y > 0.0 {
                let (rect, response) = ui.allocate_exact_size(remaining, egui::Sense::click());

                // Only show context menu if clicked in this empty area
                response.context_menu(|ui| {
                    Self::show_tree_context_menu(ui, app_state);
                });

                // Visual feedback for drop target in empty area
                if self.dragging_part.is_some() && response.hovered() {
                    ui.painter().rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(1.0, egui::Color32::GRAY),
                    );
                }
            }
        });

        // Handle drop on release
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(dragged_id) = self.dragging_part {
                if self.drop_target_base_link {
                    // Dropped on base_link - connect to base_link
                    actions.push(TreeAction::ConnectToBaseLink(dragged_id));
                } else if let Some(target_id) = self.drop_target {
                    // Dropped on another part - connect
                    if dragged_id != target_id {
                        let state = app_state.lock();
                        let can_connect = self.can_connect(&state, target_id, dragged_id);
                        drop(state);

                        if can_connect {
                            actions.push(TreeAction::Connect {
                                parent: target_id,
                                child: dragged_id,
                            });
                        }
                    }
                } else {
                    // Dropped outside (no target) - disconnect if has parent
                    let state = app_state.lock();
                    let has_parent = state.project.assembly.links.iter()
                        .find(|(_, l)| l.part_id == Some(dragged_id))
                        .and_then(|(link_id, _)| state.project.assembly.parent.get(link_id))
                        .is_some();
                    drop(state);

                    if has_parent {
                        actions.push(TreeAction::Disconnect(dragged_id));
                    }
                }
            }
            // Clear drag state
            self.dragging_part = None;
            self.drop_target = None;
            self.drop_target_base_link = false;
        }

        // Process collected actions
        for action in actions {
            match action {
                TreeAction::Select(id) => {
                    app_state.lock().queue_action(AppAction::SelectPart(Some(id)));
                }
                TreeAction::Delete(id) => {
                    app_state.lock().queue_action(AppAction::SelectPart(Some(id)));
                    app_state.lock().queue_action(AppAction::DeleteSelectedPart);
                }
                TreeAction::Disconnect(id) => {
                    app_state.lock().queue_action(AppAction::DisconnectPart { child: id });
                }
                TreeAction::Connect { parent, child } => {
                    // ConnectParts handler will disconnect existing parent if needed
                    app_state.lock().queue_action(AppAction::ConnectParts { parent, child });
                }
                TreeAction::ConnectToBaseLink(part_id) => {
                    app_state.lock().queue_action(AppAction::ConnectToBaseLink(part_id));
                }
            }
        }

        if is_empty {
            ui.weak("No parts loaded.\nClick 'Import STL...' to add parts.");
        }
    }
}

impl PartListPanel {
    /// Build tree structure from Assembly state
    fn build_tree_structure(
        &self,
        state: &crate::app_state::AppState,
    ) -> (Vec<Uuid>, HashMap<Uuid, Vec<Uuid>>, HashSet<Uuid>, Vec<Uuid>) {
        let assembly = &state.project.assembly;

        // Map link_id -> part_id (only for links with parts)
        let link_to_part: HashMap<Uuid, Uuid> = assembly.links
            .iter()
            .filter_map(|(link_id, link)| link.part_id.map(|pid| (*link_id, pid)))
            .collect();

        // Map part_id -> link_id
        let part_to_link: HashMap<Uuid, Uuid> = assembly.links
            .iter()
            .filter_map(|(link_id, link)| link.part_id.map(|pid| (pid, *link_id)))
            .collect();

        // Build children map (part_id -> [child_part_ids])
        // For base_link (which has no part_id), we track its children separately
        let mut children_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (parent_link_id, children) in &assembly.children {
            if let Some(&parent_part_id) = link_to_part.get(parent_link_id) {
                let child_parts: Vec<Uuid> = children
                    .iter()
                    .filter_map(|(_, child_link_id)| link_to_part.get(child_link_id).copied())
                    .collect();
                if !child_parts.is_empty() {
                    children_map.insert(parent_part_id, child_parts);
                }
            }
        }

        // Identify parts with parents
        let parts_with_parent: HashSet<Uuid> = assembly.parent.keys()
            .filter_map(|link_id| link_to_part.get(link_id).copied())
            .collect();

        // Find parts that are direct children of base_link (root_link)
        let mut root_parts: Vec<Uuid> = Vec::new();
        if let Some(root_link_id) = assembly.root_link {
            // Get children of base_link
            if let Some(children) = assembly.children.get(&root_link_id) {
                for (_, child_link_id) in children {
                    if let Some(&part_id) = link_to_part.get(child_link_id) {
                        root_parts.push(part_id);
                    }
                }
            }
        }

        // Find orphaned parts (not connected to base_link hierarchy)
        let orphaned_parts: Vec<Uuid> = state.parts.keys()
            .filter(|part_id| {
                if let Some(&link_id) = part_to_link.get(part_id) {
                    // Part has a link - check if it's disconnected (no parent)
                    !assembly.parent.contains_key(&link_id)
                } else {
                    // Part not in assembly at all
                    true
                }
            })
            .copied()
            .collect();

        (root_parts, children_map, parts_with_parent, orphaned_parts)
    }

    /// Check if connecting parent to child would be valid
    fn can_connect(&self, state: &crate::app_state::AppState, parent_part: Uuid, child_part: Uuid) -> bool {
        if parent_part == child_part {
            return false;
        }

        let assembly = &state.project.assembly;

        // Find link IDs
        let parent_link = assembly.links.iter()
            .find(|(_, l)| l.part_id == Some(parent_part))
            .map(|(id, _)| *id);
        let child_link = assembly.links.iter()
            .find(|(_, l)| l.part_id == Some(child_part))
            .map(|(id, _)| *id);

        match (parent_link, child_link) {
            (Some(p), Some(c)) => {
                // Check if connecting would create a cycle
                // by checking if child is an ancestor of parent
                let mut current = Some(p);
                while let Some(id) = current {
                    if id == c {
                        return false;
                    }
                    current = assembly.parent.get(&id).map(|(_, parent_id)| *parent_id);
                }
                true
            }
            _ => true, // If either isn't in assembly yet, no cycle possible
        }
    }

    /// Render a draggable/droppable part item
    fn render_part_item(
        &mut self,
        ui: &mut egui::Ui,
        part_id: Uuid,
        label_text: &str,
        is_selected: bool,
        has_parent: bool,
        actions: &mut Vec<TreeAction>,
    ) {
        let is_being_dragged = self.dragging_part == Some(part_id);
        let is_drop_target = self.drop_target == Some(part_id);

        // Visual style based on drag state
        let text = if is_being_dragged {
            egui::RichText::new(label_text).italics().weak()
        } else if is_drop_target {
            egui::RichText::new(label_text).strong().color(egui::Color32::GREEN)
        } else {
            egui::RichText::new(label_text)
        };

        // Create interactive area with click and drag
        let response = ui.add(
            egui::Label::new(text)
                .selectable(false)
                .sense(egui::Sense::click_and_drag())
        );

        // Draw selection background
        if is_selected {
            ui.painter().rect_filled(
                response.rect,
                2.0,
                ui.visuals().selection.bg_fill,
            );
        }

        // Context menu
        response.context_menu(|ui| {
            if has_parent {
                if ui.button("Disconnect").clicked() {
                    actions.push(TreeAction::Disconnect(part_id));
                    ui.close_menu();
                }
            }
            if ui.button("Delete").clicked() {
                actions.push(TreeAction::Delete(part_id));
                ui.close_menu();
            }
        });

        // Handle drag start
        if response.drag_started() {
            self.dragging_part = Some(part_id);
        }

        // Handle as drop target when something else is being dragged
        if let Some(dragging) = self.dragging_part {
            if dragging != part_id && response.hovered() {
                self.drop_target = Some(part_id);
            }
        }

        // Selection on click
        if response.clicked() {
            actions.push(TreeAction::Select(part_id));
        }
    }

    /// Render a part in the tree with its children
    fn render_part_tree(
        &mut self,
        ui: &mut egui::Ui,
        part_id: Uuid,
        part_names: &HashMap<Uuid, String>,
        children_map: &HashMap<Uuid, Vec<Uuid>>,
        parts_with_parent: &HashSet<Uuid>,
        selected_id: Option<Uuid>,
        depth: usize,
        actions: &mut Vec<TreeAction>,
    ) {
        let Some(name) = part_names.get(&part_id) else { return };
        let children = children_map.get(&part_id);
        let has_children = children.is_some_and(|c| !c.is_empty());
        let has_parent = parts_with_parent.contains(&part_id);
        let is_selected = selected_id == Some(part_id);

        ui.push_id(part_id, |ui| {
            let indent = depth as f32 * 16.0;

            // Tree icon
            let icon = if has_children { "▼" } else { "●" };
            let label_text = format!("{} {}", icon, name);

            ui.horizontal(|ui| {
                ui.add_space(indent);
                self.render_part_item(ui, part_id, &label_text, is_selected, has_parent, actions);
            });

            // Render children
            if let Some(children) = children {
                for child_id in children {
                    self.render_part_tree(
                        ui,
                        *child_id,
                        part_names,
                        children_map,
                        parts_with_parent,
                        selected_id,
                        depth + 1,
                        actions,
                    );
                }
            }
        });
    }

    /// Render an orphaned (unconnected) part
    fn render_orphan_part(
        &mut self,
        ui: &mut egui::Ui,
        part_id: Uuid,
        name: &str,
        selected_id: Option<Uuid>,
        actions: &mut Vec<TreeAction>,
    ) {
        let is_selected = selected_id == Some(part_id);
        let label_text = format!("○ {}", name);

        ui.push_id(part_id, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(16.0); // Indent under project root
                self.render_part_item(ui, part_id, &label_text, is_selected, false, actions);
            });
        });
    }

    /// Render the project root node
    fn render_project_root(
        &mut self,
        ui: &mut egui::Ui,
        project_name: &str,
        app_state: &SharedAppState,
    ) {
        ui.push_id("project_root", |ui| {
            if self.editing_project_name {
                // Editing mode - show text input
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.project_name_buffer)
                            .desired_width(150.0)
                    );

                    // Auto-focus on first frame
                    if response.gained_focus() || ui.memory(|m| m.has_focus(response.id)) {
                        response.request_focus();
                    }

                    // Confirm on Enter or when focus is lost
                    if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.project_name_buffer.trim().is_empty() {
                            app_state.lock().project.name = self.project_name_buffer.trim().to_string();
                            app_state.lock().modified = true;
                        }
                        self.editing_project_name = false;
                    }

                    // Cancel on Escape
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.editing_project_name = false;
                    }
                });
            } else {
                // Display mode
                let label_text = format!("📦 {}", project_name);
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(label_text).strong())
                        .selectable(false)
                        .sense(egui::Sense::click())
                );

                // Context menu for renaming
                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        self.project_name_buffer = project_name.to_string();
                        self.editing_project_name = true;
                        ui.close_menu();
                    }
                });

                // Double-click to rename
                if response.double_clicked() {
                    self.project_name_buffer = project_name.to_string();
                    self.editing_project_name = true;
                }
            }
        });
    }

    /// Render the base_link node (renameable, drop target for parts)
    fn render_base_link(
        &mut self,
        ui: &mut egui::Ui,
        base_link_name: &str,
        app_state: &SharedAppState,
    ) {
        ui.push_id("base_link", |ui| {
            ui.horizontal(|ui| {
                ui.add_space(16.0);

                if self.editing_base_link_name {
                    // Editing mode - show text input
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.base_link_name_buffer)
                            .desired_width(120.0)
                    );

                    // Auto-focus
                    response.request_focus();

                    // Confirm on Enter or when focus is lost
                    if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.base_link_name_buffer.trim().is_empty() {
                            let mut state = app_state.lock();
                            if let Some(base_link) = state.project.assembly.base_link_mut() {
                                base_link.name = self.base_link_name_buffer.trim().to_string();
                            }
                            state.modified = true;
                        }
                        self.editing_base_link_name = false;
                    }

                    // Cancel on Escape
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.editing_base_link_name = false;
                    }
                } else {
                    // Display mode - can be a drop target
                    let is_drop_target = self.drop_target_base_link;

                    let text = if is_drop_target {
                        egui::RichText::new(format!("🔗 {}", base_link_name))
                            .strong()
                            .color(egui::Color32::GREEN)
                    } else {
                        egui::RichText::new(format!("🔗 {}", base_link_name))
                            .color(egui::Color32::LIGHT_BLUE)
                    };

                    let response = ui.add(
                        egui::Label::new(text)
                            .selectable(false)
                            .sense(egui::Sense::click())
                    );

                    // Context menu for renaming
                    response.context_menu(|ui| {
                        if ui.button("Rename").clicked() {
                            self.base_link_name_buffer = base_link_name.to_string();
                            self.editing_base_link_name = true;
                            ui.close_menu();
                        }
                    });

                    // Double-click to rename
                    if response.double_clicked() {
                        self.base_link_name_buffer = base_link_name.to_string();
                        self.editing_base_link_name = true;
                    }

                    // Handle as drop target when dragging
                    if self.dragging_part.is_some() && response.hovered() {
                        self.drop_target_base_link = true;
                    }
                }
            });
        });
    }

    /// Show context menu for creating new objects
    fn show_tree_context_menu(ui: &mut egui::Ui, app_state: &SharedAppState) {
        // Import STL
        if ui.button("Import STL...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STL files", &["stl", "STL"])
                .pick_file()
            {
                app_state.lock().queue_action(AppAction::ImportStl(path));
            }
            ui.close_menu();
        }

        ui.separator();

        // Create Primitives submenu
        ui.menu_button("Create Primitives", |ui| {
            if ui.button("Box").clicked() {
                app_state.lock().queue_action(AppAction::CreatePrimitive {
                    primitive_type: PrimitiveType::Box,
                    name: None,
                });
                ui.close_menu();
            }
            if ui.button("Cylinder").clicked() {
                app_state.lock().queue_action(AppAction::CreatePrimitive {
                    primitive_type: PrimitiveType::Cylinder,
                    name: None,
                });
                ui.close_menu();
            }
            if ui.button("Sphere").clicked() {
                app_state.lock().queue_action(AppAction::CreatePrimitive {
                    primitive_type: PrimitiveType::Sphere,
                    name: None,
                });
                ui.close_menu();
            }
        });

        // Create Empty
        if ui.button("Create Empty...").clicked() {
            app_state.lock().queue_action(AppAction::CreateEmpty { name: None });
            ui.close_menu();
        }
    }
}
