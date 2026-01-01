//! Part list panel

use crate::app_state::{AppAction, SharedAppState};
use crate::panels::Panel;

/// Part list panel
pub struct PartListPanel;

impl PartListPanel {
    pub fn new() -> Self {
        Self
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
        ui.horizontal(|ui| {
            if ui.button("Import STL...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("STL files", &["stl", "STL"])
                    .pick_file()
                {
                    app_state.lock().queue_action(AppAction::ImportStl(path));
                }
            }
        });

        ui.separator();

        // List of parts
        let state = app_state.lock();
        let selected_id = state.selected_part;
        let parts: Vec<_> = state.parts.values().map(|p| (p.id, p.name.clone())).collect();
        let is_empty = parts.is_empty();
        drop(state);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, name) in parts {
                let is_selected = selected_id == Some(id);
                let response = ui.selectable_label(is_selected, &name);

                if response.clicked() {
                    app_state.lock().queue_action(AppAction::SelectPart(Some(id)));
                }

                response.context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        app_state.lock().queue_action(AppAction::SelectPart(Some(id)));
                        app_state.lock().queue_action(AppAction::DeleteSelectedPart);
                        ui.close_menu();
                    }
                });
            }
        });

        if is_empty {
            ui.weak("No parts loaded.\nClick 'Import STL...' to add parts.");
        }
    }
}
