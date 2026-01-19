//! 3D Viewport panel

mod overlays;
mod plane_picking;
mod sketch_input;
mod sketch_rendering;

use glam::{Mat4, Vec3};
use rk_renderer::{GizmoAxis, GizmoMode, GizmoSpace, SketchRenderData, plane_ids};

use crate::config::SharedConfig;
use crate::panels::Panel;
use crate::state::{
    AppAction, GizmoTransform, PickablePartData, ReferencePlane, SharedAppState,
    SharedViewportState, SketchAction, pick_object,
};

use overlays::{
    render_axes_indicator, render_camera_settings, render_dimension_dialog, render_extrude_dialog,
    render_gizmo_toggle, render_plane_selection_hint, render_sketch_toolbar,
};
use plane_picking::pick_reference_plane;
use sketch_input::handle_sketch_mode_input;
use sketch_rendering::sketch_to_render_data;

/// Convert ReferencePlane to plane_ids for the renderer.
fn reference_plane_to_id(plane: Option<ReferencePlane>) -> u32 {
    match plane {
        None => plane_ids::NONE,
        Some(ReferencePlane::XY) => plane_ids::XY,
        Some(ReferencePlane::XZ) => plane_ids::XZ,
        Some(ReferencePlane::YZ) => plane_ids::YZ,
    }
}

/// 3D viewport panel
pub struct ViewportPanel {
    last_size: egui::Vec2,
    hovered_axis: GizmoAxis,
    show_camera_settings: bool,
    /// Gizmo toolbar collapsed state
    gizmo_collapsed: bool,
    /// Camera toolbar collapsed state
    camera_collapsed: bool,
}

impl ViewportPanel {
    pub fn new() -> Self {
        Self {
            last_size: egui::Vec2::ZERO,
            hovered_axis: GizmoAxis::None,
            show_camera_settings: false,
            gizmo_collapsed: false,
            camera_collapsed: false,
        }
    }
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for ViewportPanel {
    fn name(&self) -> &str {
        "3D Viewport"
    }

    fn needs_render_context(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, _app_state: &SharedAppState) {
        // Fallback when no render context
        let available_size = ui.available_size();
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        painter.rect_filled(response.rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            "3D Viewport\n(WebGPU not available)",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );

        self.last_size = available_size;
    }

    fn ui_with_render_context(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &SharedAppState,
        render_state: &egui_wgpu::RenderState,
        viewport_state: &SharedViewportState,
        config: &SharedConfig,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("View:");
            if ui.button("Top").clicked() {
                viewport_state.lock().renderer.camera_mut().set_top_view();
            }
            if ui.button("Front").clicked() {
                viewport_state.lock().renderer.camera_mut().set_front_view();
            }
            if ui.button("Side").clicked() {
                viewport_state.lock().renderer.camera_mut().set_side_view();
            }
            if ui.button("Fit All").clicked() {
                viewport_state
                    .lock()
                    .renderer
                    .camera_mut()
                    .fit_all(Vec3::ZERO, 2.0);
            }

            ui.separator();

            let mut state = viewport_state.lock();
            let mut show_grid = state.renderer.show_grid();
            let mut show_axes = state.renderer.show_axes();
            let mut show_markers = state.renderer.show_markers();
            if ui.checkbox(&mut show_grid, "Grid").changed() {
                state.renderer.set_show_grid(show_grid);
            }
            if ui.checkbox(&mut show_axes, "Axes").changed() {
                state.renderer.set_show_axes(show_axes);
            }
            if ui.checkbox(&mut show_markers, "Markers").changed() {
                state.renderer.set_show_markers(show_markers);
            }
        });

        // Main viewport area
        let available_size = ui.available_size();
        let width = available_size.x as u32;
        let height = available_size.y as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Ensure texture and render
        let texture_id = {
            let mut vp_state = viewport_state.lock();
            let mut egui_renderer = render_state.renderer.write();
            let tex_id = vp_state.ensure_texture(width, height, &mut egui_renderer);

            // Check if we're in plane selection mode
            let is_plane_selection_mode = {
                let app = app_state.lock();
                app.cad.editor_mode.is_plane_selection()
            };

            // Enable/disable plane selector based on editor mode
            vp_state
                .renderer
                .set_plane_selector_visible(is_plane_selection_mode);

            // Update sketch data for rendering
            let sketch_render_data: Vec<SketchRenderData> = {
                let app = app_state.lock();
                let cad = &app.cad;

                // Get selected entities and in-progress state if in sketch mode
                let (selected_entities, in_progress, active_sketch_id) =
                    if let Some(sketch_state) = cad.editor_mode.sketch() {
                        (
                            sketch_state.selected_entities.clone(),
                            sketch_state.in_progress.clone(),
                            Some(sketch_state.active_sketch),
                        )
                    } else {
                        (Vec::new(), None, None)
                    };

                // Convert all sketches to render data
                cad.data
                    .history
                    .sketches()
                    .values()
                    .map(|sketch| {
                        let is_active = active_sketch_id == Some(sketch.id);
                        let in_prog_ref = if is_active {
                            in_progress.as_ref()
                        } else {
                            None
                        };
                        sketch_to_render_data(sketch, &selected_entities, is_active, in_prog_ref)
                    })
                    .collect()
            };

            // Set sketch data and prepare GPU resources
            let device = vp_state.device.clone();
            vp_state.renderer.set_sketch_data(sketch_render_data);
            vp_state.renderer.prepare_sketches(&device);

            // Update preview mesh for extrude preview
            {
                let app = app_state.lock();
                if let Some(sketch_state) = app.cad.editor_mode.sketch() {
                    if sketch_state.extrude_dialog.open {
                        if let Some(ref preview_mesh) = sketch_state.extrude_dialog.preview_mesh {
                            // The preview mesh is already in world coordinates
                            // (kernel.extrude transforms profile using plane origin/axes)
                            // so we use identity transform here
                            vp_state.renderer.set_preview_mesh(
                                &device,
                                &preview_mesh.vertices,
                                &preview_mesh.normals,
                                &preview_mesh.indices,
                                Mat4::IDENTITY,
                            );
                        } else {
                            vp_state.renderer.clear_preview_mesh();
                        }
                    } else {
                        vp_state.renderer.clear_preview_mesh();
                    }
                } else {
                    vp_state.renderer.clear_preview_mesh();
                }
            }

            vp_state.render();
            tex_id
        };

        // Display the rendered texture
        let response = ui.add(
            egui::Image::new(egui::load::SizedTexture::new(
                texture_id,
                [available_size.x, available_size.y],
            ))
            .sense(egui::Sense::click_and_drag()),
        );

        // Get mouse position relative to viewport
        let mouse_pos = response.hover_pos().or(response.interact_pointer_pos());
        let local_mouse = mouse_pos.map(|p| p - response.rect.min);

        // Handle camera input
        let mut vp_state = viewport_state.lock();

        // Check if in plane selection mode
        let is_plane_selection = {
            let app = app_state.lock();
            app.cad.editor_mode.is_plane_selection()
        };

        // Handle plane selection mode
        if is_plane_selection && let Some(pos) = local_mouse {
            // Pick plane under cursor
            let camera = vp_state.renderer.camera();
            let hovered_plane =
                pick_reference_plane(camera, pos.x, pos.y, available_size.x, available_size.y);

            // Update highlight
            let plane_id = reference_plane_to_id(hovered_plane);
            let queue = vp_state.queue.clone();
            vp_state
                .renderer
                .set_plane_selector_highlighted(&queue, plane_id);

            // Handle click to select plane
            if response.clicked_by(egui::PointerButton::Primary)
                && let Some(plane) = hovered_plane
            {
                drop(vp_state);
                app_state.lock().queue_action(AppAction::SketchAction(
                    SketchAction::SelectPlaneAndCreateSketch { plane },
                ));
                vp_state = viewport_state.lock();
            }
        }

        // Check if in sketch mode
        let is_sketch_mode = {
            let app = app_state.lock();
            app.cad.editor_mode.is_sketch()
        };

        // Handle sketch mode input
        if is_sketch_mode {
            let sketch_input_result = handle_sketch_mode_input(
                &response,
                ui,
                local_mouse,
                available_size,
                app_state,
                &vp_state,
            );

            // If sketch mode consumed the click, skip normal interaction
            if sketch_input_result {
                // Continue to camera controls at the end
            }
        }

        // Gizmo interaction (left mouse button) - skip if in sketch mode
        let mut gizmo_delta: Option<GizmoTransform> = None;

        if !is_sketch_mode && let Some(pos) = local_mouse {
            // Check for gizmo hover
            if !vp_state.is_dragging_gizmo() {
                let hit_axis =
                    vp_state.gizmo_hit_test(pos.x, pos.y, available_size.x, available_size.y);
                if hit_axis != self.hovered_axis {
                    self.hovered_axis = hit_axis;
                    let queue = vp_state.queue.clone();
                    vp_state.renderer.set_gizmo_highlight(&queue, hit_axis);
                }
            }

            // Start drag on left click
            if response.drag_started_by(egui::PointerButton::Primary)
                && self.hovered_axis != GizmoAxis::None
            {
                vp_state.start_gizmo_drag(
                    self.hovered_axis,
                    pos.x,
                    pos.y,
                    available_size.x,
                    available_size.y,
                );
            }

            // Update drag
            if vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Primary) {
                gizmo_delta =
                    vp_state.update_gizmo_drag(pos.x, pos.y, available_size.x, available_size.y);
            }

            // End drag
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                vp_state.end_gizmo_drag();
            }

            // Object picking on click (only if not interacting with gizmo)
            if response.clicked_by(egui::PointerButton::Primary)
                && self.hovered_axis == GizmoAxis::None
            {
                // Gather pickable part data from app_state
                let pickable_parts: Vec<PickablePartData> = {
                    let app = app_state.lock();
                    app.project
                        .parts()
                        .values()
                        .map(|part| PickablePartData {
                            id: part.id,
                            vertices: part.vertices.clone(),
                            indices: part.indices.clone(),
                            transform: part.origin_transform,
                            bbox_min: part.bbox_min,
                            bbox_max: part.bbox_max,
                        })
                        .collect()
                };

                // Perform picking
                let camera = vp_state.renderer.camera();
                let hit = pick_object(
                    camera,
                    pos.x,
                    pos.y,
                    available_size.x,
                    available_size.y,
                    &pickable_parts,
                );

                // Queue selection action
                let selected_id = hit.map(|(id, _)| id);
                app_state
                    .lock()
                    .queue_action(AppAction::SelectPart(selected_id));
            }
        }

        // Apply gizmo transform to collision element
        if let Some(transform) = gizmo_delta
            && let Some((link_id, collision_index)) = vp_state.gizmo.editing_collision
        {
            let link_world_transform = vp_state.gizmo.link_world_transform;
            drop(vp_state);

            // Calculate the delta in link-local space
            let link_world_inv = link_world_transform.inverse();

            let mut app = app_state.lock();

            match transform {
                GizmoTransform::Translation(delta) => {
                    // Transform world delta to link-local delta
                    let local_delta = link_world_inv.transform_vector3(delta);

                    if let Some(link) = app.project.assembly.get_link_mut(link_id)
                        && let Some(collision) = link.collisions.get_mut(collision_index)
                    {
                        collision.origin.xyz[0] += local_delta.x;
                        collision.origin.xyz[1] += local_delta.y;
                        collision.origin.xyz[2] += local_delta.z;
                    }
                }
                GizmoTransform::Rotation(rotation) => {
                    // For rotation, we need to update the RPY angles
                    if let Some(link) = app.project.assembly.get_link_mut(link_id)
                        && let Some(collision) = link.collisions.get_mut(collision_index)
                    {
                        // Current rotation as quaternion
                        let current_quat = collision.origin.to_quat();
                        // Apply the rotation delta
                        let new_quat = rotation * current_quat;
                        // Convert back to euler angles (XYZ order)
                        let (x, y, z) = new_quat.to_euler(glam::EulerRot::XYZ);
                        collision.origin.rpy = [x, y, z];
                    }
                }
                GizmoTransform::Scale(_) => {
                    // Collision origins don't support scaling - ignore
                }
            }

            app.modified = true;
            drop(app);

            // Re-lock viewport state for rest of handling
            vp_state = viewport_state.lock();
        }
        // Apply gizmo transform to part
        else if let Some(transform) = gizmo_delta
            && let Some(part_id) = vp_state.gizmo.part_id
        {
            let queue = vp_state.queue.clone();
            drop(vp_state);

            let mut app = app_state.lock();

            match transform {
                GizmoTransform::Translation(delta) => {
                    // Moving the whole part - update part transform
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (scale, rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_translation = translation + delta;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            scale,
                            rotation,
                            new_translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
                GizmoTransform::Rotation(rotation) => {
                    // Rotating the whole part
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (scale, old_rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_rotation = rotation * old_rotation;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            scale,
                            new_rotation,
                            translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
                GizmoTransform::Scale(scale_delta) => {
                    // Scaling the whole part
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (old_scale, rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_scale = old_scale * scale_delta;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            new_scale,
                            rotation,
                            translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
            }

            // Re-lock viewport state for rest of handling
            vp_state = viewport_state.lock();
        }

        // Get camera sensitivity from config
        let (orbit_sens, pan_sens, zoom_sens) = {
            let cfg = config.read();
            let cam = &cfg.config().renderer.camera;
            (
                cam.orbit_sensitivity,
                cam.pan_sensitivity,
                cam.zoom_sensitivity,
            )
        };

        // Middle mouse button for orbit/pan (only if not dragging gizmo)
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            if ui.input(|i| i.modifiers.shift) {
                // Pan
                vp_state
                    .renderer
                    .camera_mut()
                    .pan_with_sensitivity(delta.x, delta.y, pan_sens);
            } else {
                // Orbit
                vp_state
                    .renderer
                    .camera_mut()
                    .orbit(-delta.x * orbit_sens, delta.y * orbit_sens);
            }
        }

        // Right mouse button for orbit as well
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            vp_state
                .renderer
                .camera_mut()
                .orbit(-delta.x * orbit_sens, delta.y * orbit_sens);
        }

        // Zoom with scroll
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                vp_state
                    .renderer
                    .camera_mut()
                    .zoom(scroll_delta * zoom_sens);
            }
        }

        // Gizmo mode keyboard shortcuts
        if response.hovered() {
            ui.input(|i| {
                if i.key_pressed(egui::Key::T) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Translate);
                }
                if i.key_pressed(egui::Key::R) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Rotate);
                }
                if i.key_pressed(egui::Key::S) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Scale);
                }
                // Toggle coordinate space (G key)
                if i.key_pressed(egui::Key::G) {
                    let current_space = vp_state.renderer.gizmo_space();
                    let next_space = match current_space {
                        GizmoSpace::Global => GizmoSpace::Local,
                        GizmoSpace::Local => GizmoSpace::Global,
                    };
                    let queue = vp_state.queue.clone();
                    vp_state.renderer.set_gizmo_space(&queue, next_space);
                }
            });
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Reset View").clicked() {
                vp_state.renderer.camera_mut().fit_all(Vec3::ZERO, 2.0);
                ui.close();
            }
            ui.separator();
            if ui.button("Top View").clicked() {
                vp_state.renderer.camera_mut().set_top_view();
                ui.close();
            }
            if ui.button("Front View").clicked() {
                vp_state.renderer.camera_mut().set_front_view();
                ui.close();
            }
            if ui.button("Side View").clicked() {
                vp_state.renderer.camera_mut().set_side_view();
                ui.close();
            }
        });

        // Get camera state for axes indicator
        let yaw = vp_state.renderer.camera().yaw;
        let pitch = vp_state.renderer.camera().pitch;
        drop(vp_state);

        // Draw axes indicator overlay
        render_axes_indicator(ui, response.rect, yaw, pitch);

        // Draw gizmo mode toggle overlay (top-left) with slide animation
        render_gizmo_toggle(ui, response.rect, viewport_state, &mut self.gizmo_collapsed);

        // Draw camera settings overlay (top-right, Unity-style)
        render_camera_settings(
            ui,
            response.rect,
            viewport_state,
            &mut self.show_camera_settings,
            &mut self.camera_collapsed,
        );

        // Draw sketch toolbar (bottom-left) when in sketch mode
        {
            let app = app_state.lock();
            if let Some(sketch_state) = app.cad.editor_mode.sketch() {
                let current_tool = sketch_state.current_tool;
                let extrude_dialog_open = sketch_state.extrude_dialog.open;
                let dimension_dialog_open = sketch_state.dimension_dialog.open;
                drop(app); // Release lock before calling render
                render_sketch_toolbar(ui, response.rect, app_state, current_tool);

                // Draw extrude dialog if open
                if extrude_dialog_open {
                    render_extrude_dialog(ui, response.rect, app_state);
                }

                // Draw dimension dialog if open
                if dimension_dialog_open {
                    render_dimension_dialog(ui, response.rect, app_state);
                }
            }
        }

        // Draw plane selection hint when in plane selection mode
        {
            let app = app_state.lock();
            if app.cad.editor_mode.is_plane_selection() {
                drop(app);
                render_plane_selection_hint(ui, response.rect);
            }
        }

        self.last_size = available_size;
    }
}
