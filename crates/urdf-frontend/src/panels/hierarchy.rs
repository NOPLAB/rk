//! Hierarchy panel - tree view of assembly

use crate::app_state::SharedAppState;
use crate::panels::Panel;

/// Hierarchy panel showing assembly tree
pub struct HierarchyPanel;

impl HierarchyPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for HierarchyPanel {
    fn name(&self) -> &str {
        "Hierarchy"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        let state = app_state.lock();
        let assembly = &state.project.assembly;

        ui.heading("Assembly Tree");
        ui.separator();

        if assembly.links.is_empty() {
            ui.weak("No links in assembly.\nConnect parts in the Node Graph.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(root_id) = assembly.root_link {
                self.render_link_tree(ui, assembly, root_id, &state, 0);
            } else {
                ui.weak("No root link defined.");
            }
        });
    }
}

impl HierarchyPanel {
    fn render_link_tree(
        &self,
        ui: &mut egui::Ui,
        assembly: &urdf_core::Assembly,
        link_id: uuid::Uuid,
        state: &crate::app_state::AppState,
        depth: usize,
    ) {
        let Some(link) = assembly.links.get(&link_id) else {
            return;
        };

        let indent = depth as f32 * 16.0;

        ui.horizontal(|ui| {
            ui.add_space(indent);

            let is_selected = state.selected_part == Some(link.part_id);

            // Icon based on whether it has children
            let has_children = assembly.children.get(&link_id).is_some_and(|c| !c.is_empty());
            let icon = if has_children { "▼" } else { "•" };

            if ui.selectable_label(is_selected, format!("{} {}", icon, link.name)).clicked() {
                // Would need to queue action here
            }
        });

        // Render children
        if let Some(children) = assembly.children.get(&link_id) {
            for (joint_id, child_id) in children {
                // Show joint info
                if let Some(joint) = assembly.joints.get(joint_id) {
                    ui.horizontal(|ui| {
                        ui.add_space(indent + 8.0);
                        ui.weak(format!("↳ {} ({})", joint.name, joint.joint_type.display_name()));
                    });
                }

                self.render_link_tree(ui, assembly, *child_id, state, depth + 1);
            }
        }
    }
}
