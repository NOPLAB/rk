//! Properties panel with Unity-style component system
//!
//! Components edit a per-frame clone of the selected part; the panel
//! diffs it afterwards and emits engine commands. Continuous edits
//! (DragValue drags) run under one interaction session so a whole edit
//! burst is a single undo step.

mod component;
mod components;
mod helpers;

pub use component::{ChildJointInfo, PropertyComponent, PropertyContext};

use components::{
    CollisionComponent, GeometryComponent, JointComponent, PhysicalComponent, TransformComponent,
    VisualComponent,
};
use rk_engine::{Command, InteractionId};
use uuid::Uuid;

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
    /// Active edit session (coalesces continuous edits into one undo step)
    edit_session: Option<InteractionId>,
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
            edit_session: None,
        }
    }
}

impl Default for PropertiesPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Commands produced by continuous widgets (DragValues); they coalesce
/// under the panel's interaction session
fn is_continuous_edit(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SetJointOrigin { .. }
            | Command::SetJointAxis { .. }
            | Command::SetJointLimits { .. }
            | Command::SetCollisionOrigin { .. }
            | Command::SetCollisionGeometry { .. }
    )
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
        _viewport_state: &SharedViewportState,
        _config: &SharedConfig,
    ) {
        let (engine, selected, selected_collision) = {
            let state = app_state.lock();
            (
                state.engine.clone(),
                state.selected_part,
                state.selected_collision,
            )
        };

        let Some(selected_id) = selected else {
            ui.weak("No part selected");
            self.edit_session = None;
            return;
        };

        // Pull an editable clone of the part plus link/joint context
        let (part, link_id, link_world_transform, collisions, child_joints) = {
            let eng = engine.lock();
            let Some(part) = eng.part(selected_id).cloned() else {
                drop(eng);
                ui.weak("Selected part not found");
                self.edit_session = None;
                return;
            };

            let assembly = eng.assembly();
            let (link_id, link_world_transform, collisions, child_joints) = assembly
                .find_link_by_part(selected_id)
                .map(|link| {
                    // Use the link's own world_transform so local coordinates
                    // are relative to this link, not its parent
                    let children = assembly.get_children(link.id);
                    let child_joints: Vec<ChildJointInfo> = children
                        .iter()
                        .filter_map(|(joint_id, child_link_id)| {
                            let joint = assembly.get_joint(*joint_id)?.clone();
                            let child_link = assembly.get_link(*child_link_id)?;
                            let child_part_name = child_link
                                .part_id
                                .and_then(|pid| eng.part(pid))
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
                        Some(link.world_transform),
                        link.collisions.clone(),
                        child_joints,
                    )
                })
                .unwrap_or((None, None, Vec::new(), Vec::new()));

            (
                part,
                link_id,
                link_world_transform,
                collisions,
                child_joints,
            )
        };
        let mut part = part;

        // Get selected collision index if the link matches
        let selected_collision_index = selected_collision.and_then(|(sel_link_id, index)| {
            if Some(sel_link_id) == link_id {
                Some(index)
            } else {
                None
            }
        });

        // Snapshot editable scalar fields for the post-render diff
        let before_name = part.name.clone();
        let before_transform = part.origin_transform;
        let before_mass = part.mass;
        let before_inertia = part.inertia;
        let before_color = part.color;
        let before_material = part.material_name.clone();

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
            part: &mut part,
            link_world_transform,
            link_id,
            collisions,
            selected_collision_index,
            child_joints,
            pending_actions: &mut pending_actions,
        };

        // Render each component with collapsible header
        render_component(ui, &mut self.transform, &mut ctx);
        render_component(ui, &mut self.physical, &mut ctx);
        render_component(ui, &mut self.visual, &mut ctx);
        render_component(ui, &mut self.geometry, &mut ctx);
        render_component(ui, &mut self.collision, &mut ctx);
        render_component(ui, &mut self.joint, &mut ctx);

        // Diff the edited clone against the engine's version
        let mut edits: Vec<Command> = Vec::new();
        if part.name != before_name {
            edits.push(Command::RenamePart {
                part_id: selected_id,
                name: part.name.clone(),
            });
        }
        if part.origin_transform != before_transform {
            edits.push(Command::SetPartTransform {
                part_id: selected_id,
                transform: part.origin_transform,
            });
        }
        if part.mass != before_mass {
            edits.push(Command::SetPartMass {
                part_id: selected_id,
                mass: part.mass,
            });
        }
        if part.inertia != before_inertia {
            edits.push(Command::SetPartInertia {
                part_id: selected_id,
                inertia: part.inertia,
            });
        }
        if part.color != before_color {
            edits.push(Command::SetPartColor {
                part_id: selected_id,
                color: part.color,
            });
        }
        if part.material_name != before_material {
            edits.push(Command::SetPartMaterial {
                part_id: selected_id,
                material_name: part.material_name.clone(),
            });
        }

        // Continuous component edits (joint/collision DragValues) join the
        // same session; everything else passes through untouched
        let mut passthrough: Vec<AppAction> = Vec::new();
        for action in pending_actions {
            match action {
                AppAction::Cmd(cmd) if is_continuous_edit(&cmd) => edits.push(cmd),
                other => passthrough.push(other),
            }
        }

        let mut state = app_state.lock();
        if edits.is_empty() {
            // A quiet frame ends the current edit burst
            if let Some(session) = self.edit_session.take() {
                state.queue_action(AppAction::EndInteraction {
                    session,
                    cancel: false,
                });
            }
        } else {
            let session = *self.edit_session.get_or_insert_with(Uuid::new_v4);
            for cmd in edits {
                state.queue_action(AppAction::Interactive { session, cmd });
            }
        }
        for action in passthrough {
            state.queue_action(action);
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
