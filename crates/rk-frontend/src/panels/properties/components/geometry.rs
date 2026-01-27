//! Geometry component - read-only mesh information

use egui::Ui;

use crate::panels::properties::{PropertyComponent, PropertyContext};

/// Read-only geometry information component
pub struct GeometryComponent;

impl GeometryComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeometryComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for GeometryComponent {
    fn name(&self) -> &str {
        "Geometry"
    }

    fn default_open(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        let part = &ctx.part;

        ui.label(format!("Vertices: {}", part.vertices.len()));
        ui.label(format!("Triangles: {}", part.indices.len() / 3));
        ui.label(format!(
            "Bounding Box: [{:.3}, {:.3}, {:.3}] to [{:.3}, {:.3}, {:.3}]",
            part.bbox_min[0],
            part.bbox_min[1],
            part.bbox_min[2],
            part.bbox_max[0],
            part.bbox_max[1],
            part.bbox_max[2]
        ));

        let size = part.size();
        ui.label(format!(
            "Size: {:.3} x {:.3} x {:.3}",
            size.x, size.y, size.z
        ));

        if let Some(ref path) = part.stl_path {
            ui.label(format!("STL: {}", path));
        }

        false // Read-only, never changes
    }
}
