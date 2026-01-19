//! Sketch entity state types

use glam::Vec2;
use uuid::Uuid;

/// Entity being drawn (in progress)
#[derive(Debug, Clone)]
pub enum InProgressEntity {
    /// Line from start point (awaiting end point)
    Line {
        start_point: Uuid,
        preview_end: Vec2,
    },
    /// Circle with center (awaiting radius click)
    Circle {
        center_point: Uuid,
        preview_radius: f32,
    },
    /// Arc with center (awaiting start and end points)
    Arc {
        center_point: Uuid,
        start_point: Option<Uuid>,
        preview_end: Vec2,
    },
    /// Rectangle with first corner (awaiting second corner)
    Rectangle {
        corner1: Vec2,
        preview_corner2: Vec2,
    },
}

/// State for constraint tool selection workflow
#[derive(Debug, Clone, Default)]
pub enum ConstraintToolState {
    /// Waiting for first entity selection
    #[default]
    WaitingForFirst,
    /// First entity selected, waiting for second (if needed)
    WaitingForSecond { first_entity: Uuid },
}
