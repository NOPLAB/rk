//! Sketch Geometric Entities
//!
//! Defines the basic geometric elements that can be used in sketches.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A geometric entity in a sketch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SketchEntity {
    /// A point in 2D space
    Point {
        /// Unique identifier
        id: Uuid,
        /// Position in sketch coordinates
        position: Vec2,
    },

    /// A line segment between two points
    Line {
        /// Unique identifier
        id: Uuid,
        /// Start point ID
        start: Uuid,
        /// End point ID
        end: Uuid,
    },

    /// A circular arc
    Arc {
        /// Unique identifier
        id: Uuid,
        /// Center point ID
        center: Uuid,
        /// Start point ID (on the arc)
        start: Uuid,
        /// End point ID (on the arc)
        end: Uuid,
        /// Radius of the arc
        radius: f32,
    },

    /// A full circle
    Circle {
        /// Unique identifier
        id: Uuid,
        /// Center point ID
        center: Uuid,
        /// Radius
        radius: f32,
    },

    /// An ellipse
    Ellipse {
        /// Unique identifier
        id: Uuid,
        /// Center point ID
        center: Uuid,
        /// Major axis length
        major_radius: f32,
        /// Minor axis length
        minor_radius: f32,
        /// Rotation angle of major axis (radians)
        rotation: f32,
    },

    /// A spline curve through control points
    Spline {
        /// Unique identifier
        id: Uuid,
        /// Control point IDs
        control_points: Vec<Uuid>,
        /// Whether the spline is closed
        closed: bool,
    },
}

impl SketchEntity {
    /// Get the unique ID of this entity
    pub fn id(&self) -> Uuid {
        match self {
            SketchEntity::Point { id, .. } => *id,
            SketchEntity::Line { id, .. } => *id,
            SketchEntity::Arc { id, .. } => *id,
            SketchEntity::Circle { id, .. } => *id,
            SketchEntity::Ellipse { id, .. } => *id,
            SketchEntity::Spline { id, .. } => *id,
        }
    }

    /// Get the type name of this entity
    pub fn type_name(&self) -> &'static str {
        match self {
            SketchEntity::Point { .. } => "Point",
            SketchEntity::Line { .. } => "Line",
            SketchEntity::Arc { .. } => "Arc",
            SketchEntity::Circle { .. } => "Circle",
            SketchEntity::Ellipse { .. } => "Ellipse",
            SketchEntity::Spline { .. } => "Spline",
        }
    }

    /// Check if this entity is a point
    pub fn is_point(&self) -> bool {
        matches!(self, SketchEntity::Point { .. })
    }

    /// Check if this entity is a curve (line, arc, circle, etc.)
    pub fn is_curve(&self) -> bool {
        !self.is_point()
    }

    /// Get all point IDs referenced by this entity
    pub fn referenced_points(&self) -> Vec<Uuid> {
        match self {
            SketchEntity::Point { .. } => vec![],
            SketchEntity::Line { start, end, .. } => vec![*start, *end],
            SketchEntity::Arc {
                center, start, end, ..
            } => vec![*center, *start, *end],
            SketchEntity::Circle { center, .. } => vec![*center],
            SketchEntity::Ellipse { center, .. } => vec![*center],
            SketchEntity::Spline { control_points, .. } => control_points.clone(),
        }
    }

    /// Get the degrees of freedom for this entity type
    ///
    /// This is the number of independent parameters needed to fully define
    /// the entity (before constraints are applied).
    pub fn degrees_of_freedom(&self) -> u32 {
        match self {
            SketchEntity::Point { .. } => 2,   // x, y
            SketchEntity::Line { .. } => 0,    // defined by its endpoints
            SketchEntity::Arc { .. } => 1,     // radius (points define the rest)
            SketchEntity::Circle { .. } => 1,  // radius (center is a point)
            SketchEntity::Ellipse { .. } => 3, // major radius, minor radius, rotation
            SketchEntity::Spline { control_points, .. } => control_points.len() as u32 * 2,
        }
    }

    /// Create a new point entity
    pub fn point(position: Vec2) -> Self {
        SketchEntity::Point {
            id: Uuid::new_v4(),
            position,
        }
    }

    /// Create a new line entity
    pub fn line(start: Uuid, end: Uuid) -> Self {
        SketchEntity::Line {
            id: Uuid::new_v4(),
            start,
            end,
        }
    }

    /// Create a new circle entity
    pub fn circle(center: Uuid, radius: f32) -> Self {
        SketchEntity::Circle {
            id: Uuid::new_v4(),
            center,
            radius,
        }
    }

    /// Create a new arc entity
    pub fn arc(center: Uuid, start: Uuid, end: Uuid, radius: f32) -> Self {
        SketchEntity::Arc {
            id: Uuid::new_v4(),
            center,
            start,
            end,
            radius,
        }
    }

    /// Get position if this is a point
    pub fn position(&self) -> Option<Vec2> {
        match self {
            SketchEntity::Point { position, .. } => Some(*position),
            _ => None,
        }
    }

    /// Set position if this is a point
    pub fn set_position(&mut self, new_position: Vec2) -> bool {
        match self {
            SketchEntity::Point { position, .. } => {
                *position = new_position;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id() {
        let point = SketchEntity::point(Vec2::new(1.0, 2.0));
        let id = point.id();
        assert_eq!(point.id(), id);
    }

    #[test]
    fn test_entity_type() {
        let point = SketchEntity::point(Vec2::ZERO);
        assert!(point.is_point());
        assert!(!point.is_curve());
        assert_eq!(point.type_name(), "Point");

        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let line = SketchEntity::line(p1, p2);
        assert!(!line.is_point());
        assert!(line.is_curve());
        assert_eq!(line.type_name(), "Line");
    }

    #[test]
    fn test_referenced_points() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let line = SketchEntity::line(p1, p2);

        let refs = line.referenced_points();
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&p1));
        assert!(refs.contains(&p2));
    }
}
