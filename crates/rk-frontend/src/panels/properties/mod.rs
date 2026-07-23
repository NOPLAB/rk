//! Properties panel with Unity-style component system

mod component;
mod components;
mod helpers;

pub use component::{ChildJointInfo, PropertyComponent, PropertyContext};

use components::{
    CollisionComponent, GeometryComponent, JointComponent, PhysicalComponent, TransformComponent,
    VisualComponent,
};

use crate::config::SharedConfig;
use crate::panels::Panel;
use crate::state::{AppAction, SharedAppState, SharedViewportState};

/// Properties panel for editing selected part
pub struct PropertiesPanel {
    transform: TransformComponent,
    physical: PhysicalComponent,
    visual: VisualComponent,
    geometry: GeometryComponent,
    collision: CollisionComponent,
    joint: JointComponent,
}

impl PropertiesPanel {
    pub fn new() -> Self {
        Self {
            transform: TransformComponent::new(),
            physical: PhysicalComponent::new(),
            visual: VisualComponent::new(),
            geometry: GeometryComponent::new(),
            collision: CollisionComponent::new(),
            joint: JointComponent::new(),
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
        _config: &SharedConfig,
    ) {
        let mut state = app_state.lock();

        let Some(selected_id) = state.selected_part else {
            ui.weak("No part selected");
            return;
        };

        // Find link info for this part
        let (link_id, link_world_transform, collisions, child_joints) = state
            .project
            .assembly
            .find_link_by_part(selected_id)
            .map(|link| {
                // Use the link's own world_transform for local coordinate calculations
                // This ensures local coordinates are relative to the link's coordinate system,
                // not the parent link's. This is important because joint origin changes
                // should not affect the displayed local coordinates.
                let link_transform = Some(link.world_transform);

                // Collect child joint info
                let children = state.project.assembly.get_children(link.id);
                let child_joints: Vec<ChildJointInfo> = children
                    .iter()
                    .filter_map(|(joint_id, child_link_id)| {
                        let joint = state.project.assembly.get_joint(*joint_id)?.clone();
                        let child_link = state.project.assembly.get_link(*child_link_id)?;
                        let child_part_name = child_link
                            .part_id
                            .and_then(|pid| state.project.get_part(pid))
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| child_link.name.clone());
                        Some(ChildJointInfo {
                            joint_id: *joint_id,
                            joint,
                            child_part_name,
                        })
                    })
                    .collect();

                (
                    Some(link.id),
                    link_transform,
                    link.collisions.clone(),
                    child_joints,
                )
            })
            .unwrap_or((None, None, Vec::new(), Vec::new()));

        // Get selected collision index if the link matches
        let selected_collision_index = state.selected_collision.and_then(|(sel_link_id, index)| {
            if Some(sel_link_id) == link_id {
                Some(index)
            } else {
                None
            }
        });

        let Some(part) = state.get_part_mut(selected_id) else {
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

        // Pending actions to queue after rendering
        let mut pending_actions: Vec<AppAction> = Vec::new();

        // Create context for components
        let mut ctx = PropertyContext {
            part,
            link_world_transform,
            link_id,
            collisions,
            selected_collision_index,
            child_joints,
            pending_actions: &mut pending_actions,
        };

        // Render each component with collapsible header
        // Track if transform was changed
        let transform_changed = render_component(ui, &mut self.transform, &mut ctx);
        render_component(ui, &mut self.physical, &mut ctx);
        render_component(ui, &mut self.visual, &mut ctx);
        render_component(ui, &mut self.geometry, &mut ctx);
        render_component(ui, &mut self.collision, &mut ctx);
        render_component(ui, &mut self.joint, &mut ctx);

        // If transform changed, update the renderer
        let new_transform = if transform_changed {
            Some(ctx.part.origin_transform)
        } else {
            None
        };

        // Queue any pending actions from components
        for action in pending_actions {
            state.queue_action(action);
        }

        drop(state);

        // Update renderer with new transform
        if let Some(transform) = new_transform {
            viewport_state
                .lock()
                .update_part_transform(selected_id, transform);
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
