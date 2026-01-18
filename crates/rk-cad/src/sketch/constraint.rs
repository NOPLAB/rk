//! Sketch Constraints
//!
//! Defines geometric and dimensional constraints that can be applied
//! to sketch entities.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A constraint between sketch entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SketchConstraint {
    // ============== Geometric Constraints ==============
    /// Two points are at the same location
    Coincident {
        /// Unique identifier
        id: Uuid,
        /// First point
        point1: Uuid,
        /// Second point
        point2: Uuid,
    },

    /// A line is horizontal (parallel to X axis)
    Horizontal {
        /// Unique identifier
        id: Uuid,
        /// Line to constrain
        line: Uuid,
    },

    /// A line is vertical (parallel to Y axis)
    Vertical {
        /// Unique identifier
        id: Uuid,
        /// Line to constrain
        line: Uuid,
    },

    /// Two lines are parallel
    Parallel {
        /// Unique identifier
        id: Uuid,
        /// First line
        line1: Uuid,
        /// Second line
        line2: Uuid,
    },

    /// Two lines are perpendicular
    Perpendicular {
        /// Unique identifier
        id: Uuid,
        /// First line
        line1: Uuid,
        /// Second line
        line2: Uuid,
    },

    /// A curve is tangent to another curve
    Tangent {
        /// Unique identifier
        id: Uuid,
        /// First curve
        curve1: Uuid,
        /// Second curve
        curve2: Uuid,
    },

    /// Two lines have equal length
    EqualLength {
        /// Unique identifier
        id: Uuid,
        /// First line
        line1: Uuid,
        /// Second line
        line2: Uuid,
    },

    /// Two circles/arcs have equal radius
    EqualRadius {
        /// Unique identifier
        id: Uuid,
        /// First circle/arc
        circle1: Uuid,
        /// Second circle/arc
        circle2: Uuid,
    },

    /// A point lies on a curve
    PointOnCurve {
        /// Unique identifier
        id: Uuid,
        /// Point to constrain
        point: Uuid,
        /// Curve the point should lie on
        curve: Uuid,
    },

    /// A point lies at the midpoint of a line
    Midpoint {
        /// Unique identifier
        id: Uuid,
        /// Point to constrain
        point: Uuid,
        /// Line
        line: Uuid,
    },

    /// Two entities are symmetric about a line
    Symmetric {
        /// Unique identifier
        id: Uuid,
        /// First entity
        entity1: Uuid,
        /// Second entity
        entity2: Uuid,
        /// Symmetry axis (line)
        axis: Uuid,
    },

    /// A point is at a fixed position
    Fixed {
        /// Unique identifier
        id: Uuid,
        /// Point to fix
        point: Uuid,
        /// Fixed X coordinate
        x: f32,
        /// Fixed Y coordinate
        y: f32,
    },

    // ============== Dimensional Constraints ==============
    /// Distance between two points
    Distance {
        /// Unique identifier
        id: Uuid,
        /// First entity (point or curve)
        entity1: Uuid,
        /// Second entity (point or curve)
        entity2: Uuid,
        /// Required distance
        value: f32,
    },

    /// Horizontal distance between two points
    HorizontalDistance {
        /// Unique identifier
        id: Uuid,
        /// First point
        point1: Uuid,
        /// Second point
        point2: Uuid,
        /// Required horizontal distance
        value: f32,
    },

    /// Vertical distance between two points
    VerticalDistance {
        /// Unique identifier
        id: Uuid,
        /// First point
        point1: Uuid,
        /// Second point
        point2: Uuid,
        /// Required vertical distance
        value: f32,
    },

    /// Angle between two lines
    Angle {
        /// Unique identifier
        id: Uuid,
        /// First line
        line1: Uuid,
        /// Second line
        line2: Uuid,
        /// Angle in radians
        value: f32,
    },

    /// Radius of a circle or arc
    Radius {
        /// Unique identifier
        id: Uuid,
        /// Circle or arc to constrain
        circle: Uuid,
        /// Required radius
        value: f32,
    },

    /// Diameter of a circle
    Diameter {
        /// Unique identifier
        id: Uuid,
        /// Circle to constrain
        circle: Uuid,
        /// Required diameter
        value: f32,
    },

    /// Length of a line
    Length {
        /// Unique identifier
        id: Uuid,
        /// Line to constrain
        line: Uuid,
        /// Required length
        value: f32,
    },
}

impl SketchConstraint {
    /// Get the unique ID of this constraint
    pub fn id(&self) -> Uuid {
        match self {
            SketchConstraint::Coincident { id, .. } => *id,
            SketchConstraint::Horizontal { id, .. } => *id,
            SketchConstraint::Vertical { id, .. } => *id,
            SketchConstraint::Parallel { id, .. } => *id,
            SketchConstraint::Perpendicular { id, .. } => *id,
            SketchConstraint::Tangent { id, .. } => *id,
            SketchConstraint::EqualLength { id, .. } => *id,
            SketchConstraint::EqualRadius { id, .. } => *id,
            SketchConstraint::PointOnCurve { id, .. } => *id,
            SketchConstraint::Midpoint { id, .. } => *id,
            SketchConstraint::Symmetric { id, .. } => *id,
            SketchConstraint::Fixed { id, .. } => *id,
            SketchConstraint::Distance { id, .. } => *id,
            SketchConstraint::HorizontalDistance { id, .. } => *id,
            SketchConstraint::VerticalDistance { id, .. } => *id,
            SketchConstraint::Angle { id, .. } => *id,
            SketchConstraint::Radius { id, .. } => *id,
            SketchConstraint::Diameter { id, .. } => *id,
            SketchConstraint::Length { id, .. } => *id,
        }
    }

    /// Get the type name of this constraint
    pub fn type_name(&self) -> &'static str {
        match self {
            SketchConstraint::Coincident { .. } => "Coincident",
            SketchConstraint::Horizontal { .. } => "Horizontal",
            SketchConstraint::Vertical { .. } => "Vertical",
            SketchConstraint::Parallel { .. } => "Parallel",
            SketchConstraint::Perpendicular { .. } => "Perpendicular",
            SketchConstraint::Tangent { .. } => "Tangent",
            SketchConstraint::EqualLength { .. } => "Equal Length",
            SketchConstraint::EqualRadius { .. } => "Equal Radius",
            SketchConstraint::PointOnCurve { .. } => "Point on Curve",
            SketchConstraint::Midpoint { .. } => "Midpoint",
            SketchConstraint::Symmetric { .. } => "Symmetric",
            SketchConstraint::Fixed { .. } => "Fixed",
            SketchConstraint::Distance { .. } => "Distance",
            SketchConstraint::HorizontalDistance { .. } => "Horizontal Distance",
            SketchConstraint::VerticalDistance { .. } => "Vertical Distance",
            SketchConstraint::Angle { .. } => "Angle",
            SketchConstraint::Radius { .. } => "Radius",
            SketchConstraint::Diameter { .. } => "Diameter",
            SketchConstraint::Length { .. } => "Length",
        }
    }

    /// Get all entity IDs referenced by this constraint
    pub fn referenced_entities(&self) -> Vec<Uuid> {
        match self {
            SketchConstraint::Coincident { point1, point2, .. } => vec![*point1, *point2],
            SketchConstraint::Horizontal { line, .. } => vec![*line],
            SketchConstraint::Vertical { line, .. } => vec![*line],
            SketchConstraint::Parallel { line1, line2, .. } => vec![*line1, *line2],
            SketchConstraint::Perpendicular { line1, line2, .. } => vec![*line1, *line2],
            SketchConstraint::Tangent { curve1, curve2, .. } => vec![*curve1, *curve2],
            SketchConstraint::EqualLength { line1, line2, .. } => vec![*line1, *line2],
            SketchConstraint::EqualRadius {
                circle1, circle2, ..
            } => vec![*circle1, *circle2],
            SketchConstraint::PointOnCurve { point, curve, .. } => vec![*point, *curve],
            SketchConstraint::Midpoint { point, line, .. } => vec![*point, *line],
            SketchConstraint::Symmetric {
                entity1,
                entity2,
                axis,
                ..
            } => vec![*entity1, *entity2, *axis],
            SketchConstraint::Fixed { point, .. } => vec![*point],
            SketchConstraint::Distance {
                entity1, entity2, ..
            } => vec![*entity1, *entity2],
            SketchConstraint::HorizontalDistance { point1, point2, .. } => vec![*point1, *point2],
            SketchConstraint::VerticalDistance { point1, point2, .. } => vec![*point1, *point2],
            SketchConstraint::Angle { line1, line2, .. } => vec![*line1, *line2],
            SketchConstraint::Radius { circle, .. } => vec![*circle],
            SketchConstraint::Diameter { circle, .. } => vec![*circle],
            SketchConstraint::Length { line, .. } => vec![*line],
        }
    }

    /// Check if this constraint references a specific entity
    pub fn references_entity(&self, entity_id: Uuid) -> bool {
        self.referenced_entities().contains(&entity_id)
    }

    /// Get the number of equations this constraint adds to the system
    pub fn equation_count(&self) -> usize {
        match self {
            SketchConstraint::Coincident { .. } => 2, // x and y must match
            SketchConstraint::Horizontal { .. } => 1, // dy = 0
            SketchConstraint::Vertical { .. } => 1,   // dx = 0
            SketchConstraint::Parallel { .. } => 1,   // cross product = 0
            SketchConstraint::Perpendicular { .. } => 1, // dot product = 0
            SketchConstraint::Tangent { .. } => 1,    // tangent condition
            SketchConstraint::EqualLength { .. } => 1, // len1 = len2
            SketchConstraint::EqualRadius { .. } => 1, // r1 = r2
            SketchConstraint::PointOnCurve { .. } => 1, // distance to curve = 0
            SketchConstraint::Midpoint { .. } => 2,   // point = (start + end) / 2
            SketchConstraint::Symmetric { .. } => 2,  // symmetric about axis
            SketchConstraint::Fixed { .. } => 2,      // x and y fixed
            SketchConstraint::Distance { .. } => 1,   // distance = value
            SketchConstraint::HorizontalDistance { .. } => 1, // |x1 - x2| = value
            SketchConstraint::VerticalDistance { .. } => 1, // |y1 - y2| = value
            SketchConstraint::Angle { .. } => 1,      // angle = value
            SketchConstraint::Radius { .. } => 1,     // radius = value
            SketchConstraint::Diameter { .. } => 1,   // diameter = value
            SketchConstraint::Length { .. } => 1,     // length = value
        }
    }

    /// Whether this is a dimensional constraint (has a value)
    pub fn is_dimensional(&self) -> bool {
        matches!(
            self,
            SketchConstraint::Distance { .. }
                | SketchConstraint::HorizontalDistance { .. }
                | SketchConstraint::VerticalDistance { .. }
                | SketchConstraint::Angle { .. }
                | SketchConstraint::Radius { .. }
                | SketchConstraint::Diameter { .. }
                | SketchConstraint::Length { .. }
                | SketchConstraint::Fixed { .. }
        )
    }

    /// Get the dimensional value if this is a dimensional constraint
    pub fn value(&self) -> Option<f32> {
        match self {
            SketchConstraint::Distance { value, .. } => Some(*value),
            SketchConstraint::HorizontalDistance { value, .. } => Some(*value),
            SketchConstraint::VerticalDistance { value, .. } => Some(*value),
            SketchConstraint::Angle { value, .. } => Some(*value),
            SketchConstraint::Radius { value, .. } => Some(*value),
            SketchConstraint::Diameter { value, .. } => Some(*value),
            SketchConstraint::Length { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Set the dimensional value if this is a dimensional constraint
    pub fn set_value(&mut self, new_value: f32) -> bool {
        match self {
            SketchConstraint::Distance { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::HorizontalDistance { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::VerticalDistance { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::Angle { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::Radius { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::Diameter { value, .. } => {
                *value = new_value;
                true
            }
            SketchConstraint::Length { value, .. } => {
                *value = new_value;
                true
            }
            _ => false,
        }
    }

    // ============== Factory Methods ==============

    /// Create a coincident constraint
    pub fn coincident(point1: Uuid, point2: Uuid) -> Self {
        SketchConstraint::Coincident {
            id: Uuid::new_v4(),
            point1,
            point2,
        }
    }

    /// Create a horizontal constraint
    pub fn horizontal(line: Uuid) -> Self {
        SketchConstraint::Horizontal {
            id: Uuid::new_v4(),
            line,
        }
    }

    /// Create a vertical constraint
    pub fn vertical(line: Uuid) -> Self {
        SketchConstraint::Vertical {
            id: Uuid::new_v4(),
            line,
        }
    }

    /// Create a parallel constraint
    pub fn parallel(line1: Uuid, line2: Uuid) -> Self {
        SketchConstraint::Parallel {
            id: Uuid::new_v4(),
            line1,
            line2,
        }
    }

    /// Create a perpendicular constraint
    pub fn perpendicular(line1: Uuid, line2: Uuid) -> Self {
        SketchConstraint::Perpendicular {
            id: Uuid::new_v4(),
            line1,
            line2,
        }
    }

    /// Create a distance constraint
    pub fn distance(entity1: Uuid, entity2: Uuid, value: f32) -> Self {
        SketchConstraint::Distance {
            id: Uuid::new_v4(),
            entity1,
            entity2,
            value,
        }
    }

    /// Create a length constraint
    pub fn length(line: Uuid, value: f32) -> Self {
        SketchConstraint::Length {
            id: Uuid::new_v4(),
            line,
            value,
        }
    }

    /// Create a radius constraint
    pub fn radius(circle: Uuid, value: f32) -> Self {
        SketchConstraint::Radius {
            id: Uuid::new_v4(),
            circle,
            value,
        }
    }

    /// Create a fixed constraint
    pub fn fixed(point: Uuid, x: f32, y: f32) -> Self {
        SketchConstraint::Fixed {
            id: Uuid::new_v4(),
            point,
            x,
            y,
        }
    }

    /// Create an angle constraint
    pub fn angle(line1: Uuid, line2: Uuid, value: f32) -> Self {
        SketchConstraint::Angle {
            id: Uuid::new_v4(),
            line1,
            line2,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_id() {
        let c = SketchConstraint::horizontal(Uuid::new_v4());
        let id = c.id();
        assert_eq!(c.id(), id);
    }

    #[test]
    fn test_references() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let c = SketchConstraint::coincident(p1, p2);

        assert!(c.references_entity(p1));
        assert!(c.references_entity(p2));
        assert!(!c.references_entity(Uuid::new_v4()));
    }

    #[test]
    fn test_dimensional() {
        let c = SketchConstraint::distance(Uuid::new_v4(), Uuid::new_v4(), 10.0);
        assert!(c.is_dimensional());
        assert_eq!(c.value(), Some(10.0));

        let c2 = SketchConstraint::horizontal(Uuid::new_v4());
        assert!(!c2.is_dimensional());
        assert_eq!(c2.value(), None);
    }
}
