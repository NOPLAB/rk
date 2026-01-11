//! Part list panel with hierarchical tree structure and drag-and-drop

mod toolbar;
mod tree;

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::panels::Panel;
use crate::state::{AppAction, SharedAppState};

use toolbar::{render_import_toolbar, show_tree_context_menu};
use tree::{build_tree_structure, can_connect, TreeAction};

/// Part list panel with drag-and-drop hierarchy
pub struct PartListPanel {
    /// Currently dragged part ID
    dragging_part: Option<Uuid>,
    /// Current drop target (part_id)
    drop_target: Option<Uuid>,
    /// Editing project name
    editing_project_name: bool,
    /// Temporary buffer for editing project name
    project_name_buffer: String,
}

impl PartListPanel {
    pub fn new() -> Self {
        Self {
            dragging_part: None,
            drop_target: None,
            editing_project_name: false,
            project_name_buffer: String::new(),
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
            egui::RichText::new(label_text)
                .strong()
                .color(egui::Color32::GREEN)
        } else {
            egui::RichText::new(label_text)
        };

        // Create interactive area with click and drag
        let response = ui.add(
            egui::Label::new(text)
                .selectable(false)
                .sense(egui::Sense::click_and_drag()),
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
            if has_parent && ui.button("Disconnect").clicked() {
                actions.push(TreeAction::Disconnect(part_id));
                ui.close_menu();
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
        let Some(name) = part_names.get(&part_id) else {
            return;
        };
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
                            .desired_width(150.0),
                    );

                    // Auto-focus on first frame
                    if response.gained_focus() || ui.memory(|m| m.has_focus(response.id)) {
                        response.request_focus();
                    }

                    // Confirm on Enter or when focus is lost
                    if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.project_name_buffer.trim().is_empty() {
                            app_state.lock().project.name =
                                self.project_name_buffer.trim().to_string();
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
                        .sense(egui::Sense::click()),
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

}

impl Default for PartListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for PartListPanel {
    fn name(&self) -> &str {
        "Parts"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        // Import STL toolbar
        render_import_toolbar(ui, app_state);

        ui.separator();

        // Collect state data
        let state = app_state.lock();
        let selected_id = state.selected_part;
        let project_name = state.project.name.clone();

        // Build tree structure from Assembly
        let (root_link_part, root_parts, children_map, parts_with_parent, orphaned_parts) =
            build_tree_structure(&state);

        // Collect part names for display
        let part_names: HashMap<Uuid, String> = state
            .parts
            .iter()
            .map(|(id, p)| (*id, p.name.clone()))
            .collect();

        let is_empty = state.parts.is_empty();
        drop(state);

        // Reset drop targets each frame
        self.drop_target = None;

        // Collect actions during rendering
        let mut actions: Vec<TreeAction> = Vec::new();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Render project root node
            self.render_project_root(ui, &project_name, app_state);

            ui.add_space(4.0);

            // Render root link's own part (if it has one, e.g., when importing URDF)
            // This will also render all its children via children_map
            if let Some(root_part_id) = root_link_part {
                self.render_part_tree(
                    ui,
                    root_part_id,
                    &part_names,
                    &children_map,
                    &parts_with_parent,
                    selected_id,
                    1,
                    &mut actions,
                );
            } else {
                // Only render root_parts separately when root_link has no part
                // (otherwise they're already rendered as children of root_link_part)
                for root_id in &root_parts {
                    self.render_part_tree(
                        ui,
                        *root_id,
                        &part_names,
                        &children_map,
                        &parts_with_parent,
                        selected_id,
                        1,
                        &mut actions,
                    );
                }
            }

            // Render orphaned parts (not connected to any hierarchy)
            if !orphaned_parts.is_empty() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("Unconnected").weak().italics());
                });

                for part_id in &orphaned_parts {
                    if let Some(name) = part_names.get(part_id) {
                        self.render_orphan_part(ui, *part_id, name, selected_id, &mut actions);
                    }
                }
            }

            // Empty space area for context menu (right-click on empty space)
            let remaining = ui.available_size();
            if remaining.y > 0.0 {
                let (rect, response) = ui.allocate_exact_size(remaining, egui::Sense::click());

                // Only show context menu if clicked in this empty area
                response.context_menu(|ui| {
                    show_tree_context_menu(ui, app_state);
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
                if let Some(target_id) = self.drop_target {
                    // Dropped on another part - connect
                    if dragged_id != target_id {
                        let state = app_state.lock();
                        let can_conn = can_connect(&state, target_id, dragged_id);
                        drop(state);

                        if can_conn {
                            actions.push(TreeAction::Connect {
                                parent: target_id,
                                child: dragged_id,
                            });
                        }
                    }
                } else {
                    // Dropped outside (no target) - disconnect if has parent
                    let state = app_state.lock();
                    let has_parent = state
                        .project
                        .assembly
                        .links
                        .iter()
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
        }

        // Process collected actions
        for action in actions {
            match action {
                TreeAction::Select(id) => {
                    app_state
                        .lock()
                        .queue_action(AppAction::SelectPart(Some(id)));
                }
                TreeAction::Delete(id) => {
                    app_state
                        .lock()
                        .queue_action(AppAction::SelectPart(Some(id)));
                    app_state.lock().queue_action(AppAction::DeleteSelectedPart);
                }
                TreeAction::Disconnect(id) => {
                    app_state
                        .lock()
                        .queue_action(AppAction::DisconnectPart { child: id });
                }
                TreeAction::Connect { parent, child } => {
                    // ConnectParts handler will disconnect existing parent if needed
                    app_state
                        .lock()
                        .queue_action(AppAction::ConnectParts { parent, child });
                }
            }
        }

        if is_empty {
            ui.weak("No parts loaded.\nClick 'Import STL...' to add parts.");
        }
    }
}
