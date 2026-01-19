//! Render pass execution.
//!
//! This module provides functions for executing shadow and main render passes.

use crate::sub_renderers::{
    AxisRenderer, CollisionRenderer, GizmoRenderer, GridRenderer, MarkerRenderer, MeshRenderer,
    PlaneSelectorRenderer, SketchRenderer,
};

use super::{CadBodyManager, DisplayOptions, LightingSystem, PartManager, PreviewManager};

/// Shadow pass parameters.
pub struct ShadowPassParams<'a> {
    /// Lighting system.
    pub lighting: &'a LightingSystem,
    /// Part manager.
    pub parts: &'a PartManager,
    /// CAD body manager.
    pub cad_bodies: &'a CadBodyManager,
    /// Mesh renderer.
    pub mesh_renderer: &'a MeshRenderer,
}

/// Execute the shadow pass.
pub fn render_shadow_pass(
    encoder: &mut wgpu::CommandEncoder,
    params: &ShadowPassParams<'_>,
) {
    if !params.lighting.shadows_enabled() {
        return;
    }
    if params.parts.is_empty() && params.cad_bodies.is_empty() {
        return;
    }

    let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Shadow Pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: params.lighting.shadow_view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    let size = params.lighting.shadow_map_size() as f32;
    shadow_pass.set_viewport(0.0, 0.0, size, size, 0.0, 1.0);

    // Render parts
    for entry in params.parts.iter() {
        params.mesh_renderer.render_shadow(
            &mut shadow_pass,
            &entry.data,
            &entry.bind_group,
            params.lighting.shadow_light_bind_group(),
        );
    }

    // Render CAD bodies
    for entry in params.cad_bodies.iter() {
        params.mesh_renderer.render_shadow(
            &mut shadow_pass,
            &entry.data,
            &entry.bind_group,
            params.lighting.shadow_light_bind_group(),
        );
    }
}

/// Main pass parameters.
pub struct MainPassParams<'a> {
    /// Display options.
    pub display_options: &'a DisplayOptions,
    /// Lighting system.
    pub lighting: &'a LightingSystem,
    /// Part manager.
    pub parts: &'a PartManager,
    /// CAD body manager.
    pub cad_bodies: &'a CadBodyManager,
    /// Preview manager.
    pub preview: &'a PreviewManager,
    /// Grid renderer.
    pub grid_renderer: &'a GridRenderer,
    /// Mesh renderer.
    pub mesh_renderer: &'a MeshRenderer,
    /// Axis renderer.
    pub axis_renderer: &'a AxisRenderer,
    /// Marker renderer.
    pub marker_renderer: &'a MarkerRenderer,
    /// Collision renderer.
    pub collision_renderer: &'a CollisionRenderer,
    /// Sketch renderer.
    pub sketch_renderer: &'a SketchRenderer,
    /// Plane selector renderer.
    pub plane_selector_renderer: &'a PlaneSelectorRenderer,
    /// Gizmo renderer.
    pub gizmo_renderer: &'a GizmoRenderer,
    /// Depth view.
    pub depth_view: &'a wgpu::TextureView,
    /// MSAA view (if MSAA is enabled).
    pub msaa_view: Option<&'a wgpu::TextureView>,
    /// Clear color.
    pub clear_color: wgpu::Color,
}

/// Execute the main render pass.
pub fn render_main_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    params: &MainPassParams<'_>,
) {
    // Set up color attachment with MSAA if enabled
    let color_attachment = if let Some(msaa_view) = params.msaa_view {
        // MSAA enabled: render to multisample texture, resolve to output
        wgpu::RenderPassColorAttachment {
            view: msaa_view,
            resolve_target: Some(view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(params.clear_color),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        }
    } else {
        // MSAA disabled: render directly to output
        wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(params.clear_color),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        }
    };

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Main Render Pass"),
        color_attachments: &[Some(color_attachment)],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: params.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    // Render grid
    if params.display_options.show_grid {
        params.grid_renderer.render(&mut render_pass);
    }

    // Render meshes with lighting and shadows
    for entry in params.parts.iter() {
        params.mesh_renderer.render(
            &mut render_pass,
            &entry.data,
            &entry.bind_group,
            params.lighting.light_bind_group(),
        );
    }

    // Render CAD bodies with lighting and shadows
    for entry in params.cad_bodies.iter() {
        params.mesh_renderer.render(
            &mut render_pass,
            &entry.data,
            &entry.bind_group,
            params.lighting.light_bind_group(),
        );
    }

    // Render axes
    if params.display_options.show_axes {
        params.axis_renderer.render(&mut render_pass);
    }

    // Render markers
    if params.display_options.show_markers {
        params.marker_renderer.render(&mut render_pass);
    }

    // Render collision shapes (semi-transparent, after markers)
    params.collision_renderer.render(&mut render_pass);

    // Render preview mesh (semi-transparent, for extrude preview)
    if let Some(entry) = params.preview.preview_mesh() {
        params.mesh_renderer.render(
            &mut render_pass,
            &entry.data,
            &entry.bind_group,
            params.lighting.light_bind_group(),
        );
    }

    // Render plane selector (semi-transparent, for sketch plane selection)
    params.plane_selector_renderer.render(&mut render_pass);

    // Render sketches
    params.sketch_renderer.render_pass(&mut render_pass);

    // Render gizmo (always on top)
    if params.display_options.show_gizmo {
        params.gizmo_renderer.render(&mut render_pass);
    }
}
