//! Sketch entity state types

use glam::Vec2;
use uuid::Uuid;

/// Entity being drawn (in progress).
///
/// Holds coordinates only — nothing touches the engine until the final
/// click commits the whole shape as one command, so cancelling leaves
/// no orphan points behind.
#[derive(Debug, Clone)]
pub enum InProgressEntity {
    /// Line from a start position (awaiting end click)
    Line { start: Vec2, preview_end: Vec2 },
    /// Circle around a center (awaiting radius click)
    Circle { center: Vec2, preview_radius: f32 },
    /// Arc around a center (awaiting start and end clicks)
    Arc {
        center: Vec2,
        start: Option<Vec2>,
        preview_end: Vec2,
    },
    /// Rectangle from a first corner (awaiting second corner)
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
