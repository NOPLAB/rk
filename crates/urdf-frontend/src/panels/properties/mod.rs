//! Properties panel with Unity-style component system

mod component;
mod components;
mod helpers;

pub use component::{JointPointAction, PropertyComponent, PropertyContext};

use components::{
    GeometryComponent, JointPointsComponent, PhysicalComponent, TransformComponent, VisualComponent,
};

use crate::panels::Panel;
use crate::state::{SharedAppState, SharedViewportState};

/// Properties panel for editing selected part
pub struct PropertiesPanel {
    transform: TransformComponent,
    physical: PhysicalComponent,
    visual: VisualComponent,
    geometry: GeometryComponent,
    joint_points: JointPointsComponent,
}

impl PropertiesPanel {
    pub fn new() -> Self {
        Self {
            transform: TransformComponent::new(),
            physical: PhysicalComponent::new(),
            visual: VisualComponent::new(),
            geometry: GeometryComponent::new(),
            joint_points: JointPointsComponent::new(),
        }
    }
}

impl Default for PropertiesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for PropertiesPanel {
    fn name(&self) -> &str {
        "Properties"
    }

    fn needs_render_context(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, _app_state: &SharedAppState) {
        // Fallback when no render context (shouldn't happen normally)
        ui.weak("Properties panel requires render context");
    }

    fn ui_with_render_context(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &SharedAppState,
        _render_state: &egui_wgpu::RenderState,
        viewport_state: &SharedViewportState,
    ) {
        let mut state = app_state.lock();

        let Some(selected_id) = state.selected_part else {
            ui.weak("No part selected");
            return;
        };

        // Extract selection state before mutable borrow
        let selected_point = state.selected_joint_point.map(|(_, idx)| idx);

        // Find parent's world transform if this part is in assembly
        let parent_world_transform = state
            .project
            .assembly
            .find_link_by_part(selected_id)
            .and_then(|link| {
                state
                    .project
                    .assembly
                    .get_parent_link(link.id)
                    .map(|parent| parent.world_transform)
            });

        // Get joint points for this part from assembly
        let joint_points: Vec<_> = state
            .project
            .assembly
            .get_joint_points_for_part(selected_id)
            .into_iter()
            .cloned()
            .collect();

        let Some(part) = state.parts.get_mut(&selected_id) else {
            ui.weak("Selected part not found");
            return;
        };

        ui.heading("Part Properties");
        ui.separator();

        // Name (always shown, not a component)
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut part.name);
        });

        ui.separator();

        // Create context for components
        let mut ctx = PropertyContext {
            part,
            part_id: selected_id,
            selected_joint_point: selected_point,
            parent_world_transform,
            joint_points,
            joint_point_actions: Vec::new(),
        };

        // Render each component with collapsible header
        // Track if transform was changed
        let transform_changed = render_component(ui, &mut self.transform, &mut ctx);
        render_component(ui, &mut self.physical, &mut ctx);
        render_component(ui, &mut self.visual, &mut ctx);
        render_component(ui, &mut self.geometry, &mut ctx);
        render_component(ui, &mut self.joint_points, &mut ctx);

        // If transform changed, update the renderer
        let new_transform = if transform_changed {
            Some(ctx.part.origin_transform)
        } else {
            None
        };

        let is_empty = ctx.joint_points.is_empty();
        let jp_actions = std::mem::take(&mut ctx.joint_point_actions);

        // Handle joint point selection (needs state access after component rendering)
        let pending_select = self.joint_points.take_pending_select();

        // Apply joint point actions
        for action in jp_actions {
            match action {
                JointPointAction::Add(jp) => {
                    state.project.assembly.add_joint_point(jp);
                    state.modified = true;
                }
                JointPointAction::Remove(id) => {
                    state.project.assembly.remove_joint_point(id);
                    state.modified = true;
                }
                JointPointAction::Update(jp) => {
                    if let Some(existing) = state.project.assembly.get_joint_point_mut(jp.id) {
                        *existing = jp;
                        state.modified = true;
                    }
                }
            }
        }

        drop(state);

        // Update renderer with new transform
        if let Some(transform) = new_transform {
            viewport_state
                .lock()
                .update_part_transform(selected_id, transform);
        }

        if let Some(idx) = pending_select {
            app_state.lock().select_joint_point(selected_id, idx);
        }

        if is_empty {
            // Already shown in component
        }
    }
}

/// Render a component with collapsible header
/// Returns true if the component reported a change
fn render_component(
    ui: &mut egui::Ui,
    component: &mut dyn PropertyComponent,
    ctx: &mut PropertyContext,
) -> bool {
    let changed = if component.is_collapsible() {
        let response = egui::CollapsingHeader::new(component.name())
            .default_open(component.default_open())
            .show(ui, |ui| component.ui(ui, ctx));
        response.body_returned.unwrap_or(false)
    } else {
        ui.heading(component.name());
        component.ui(ui, ctx)
    };
    ui.separator();
    changed
}
