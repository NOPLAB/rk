//! Sketch tool types

/// Tool for sketch editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SketchTool {
    /// Select and move entities
    #[default]
    Select,
    /// Draw a point
    Point,
    /// Draw a line
    Line,
    /// Draw a rectangle from corner to corner
    RectangleCorner,
    /// Draw a rectangle from center outward
    RectangleCenter,
    /// Draw a rectangle from 3 points (corner, corner, height)
    Rectangle3Point,
    /// Draw a circle from center and radius
    CircleCenterRadius,
    /// Draw a circle from 2 points (diameter endpoints)
    Circle2Point,
    /// Draw a circle from 3 points on the circumference
    Circle3Point,
    /// Draw an arc from center, start angle, and end angle
    ArcCenterStartEnd,
    /// Draw an arc from 3 points (start, midpoint, end)
    Arc3Point,
    /// Add coincident constraint
    ConstrainCoincident,
    /// Add horizontal constraint
    ConstrainHorizontal,
    /// Add vertical constraint
    ConstrainVertical,
    /// Add parallel constraint
    ConstrainParallel,
    /// Add perpendicular constraint
    ConstrainPerpendicular,
    /// Add tangent constraint
    ConstrainTangent,
    /// Add equal length/radius constraint
    ConstrainEqual,
    /// Fix entity position
    ConstrainFixed,
    /// Add distance dimension
    DimensionDistance,
    /// Add horizontal distance dimension
    DimensionHorizontal,
    /// Add vertical distance dimension
    DimensionVertical,
    /// Add angle dimension
    DimensionAngle,
    /// Add radius dimension
    DimensionRadius,
}

impl SketchTool {
    /// Get the display name of the tool
    pub fn name(&self) -> &'static str {
        match self {
            SketchTool::Select => "Select",
            SketchTool::Point => "Point",
            SketchTool::Line => "Line",
            SketchTool::RectangleCorner => "Rectangle (Corner)",
            SketchTool::RectangleCenter => "Rectangle (Center)",
            SketchTool::Rectangle3Point => "Rectangle (3 Point)",
            SketchTool::CircleCenterRadius => "Circle (Center)",
            SketchTool::Circle2Point => "Circle (2 Point)",
            SketchTool::Circle3Point => "Circle (3 Point)",
            SketchTool::ArcCenterStartEnd => "Arc (Center)",
            SketchTool::Arc3Point => "Arc (3 Point)",
            SketchTool::ConstrainCoincident => "Coincident",
            SketchTool::ConstrainHorizontal => "Horizontal",
            SketchTool::ConstrainVertical => "Vertical",
            SketchTool::ConstrainParallel => "Parallel",
            SketchTool::ConstrainPerpendicular => "Perpendicular",
            SketchTool::ConstrainTangent => "Tangent",
            SketchTool::ConstrainEqual => "Equal",
            SketchTool::ConstrainFixed => "Fixed",
            SketchTool::DimensionDistance => "Distance",
            SketchTool::DimensionHorizontal => "Horizontal Dim",
            SketchTool::DimensionVertical => "Vertical Dim",
            SketchTool::DimensionAngle => "Angle",
            SketchTool::DimensionRadius => "Radius",
        }
    }

    /// Get a short label for the tool (for toolbar buttons)
    pub fn short_label(&self) -> &'static str {
        match self {
            SketchTool::Select => "⬚",
            SketchTool::Point => "•",
            SketchTool::Line => "╱",
            SketchTool::RectangleCorner => "▭",
            SketchTool::RectangleCenter => "⊞",
            SketchTool::Rectangle3Point => "▱",
            SketchTool::CircleCenterRadius => "○",
            SketchTool::Circle2Point => "⊖",
            SketchTool::Circle3Point => "◎",
            SketchTool::ArcCenterStartEnd => "⌒",
            SketchTool::Arc3Point => "◠",
            SketchTool::ConstrainCoincident => "⊙",
            SketchTool::ConstrainHorizontal => "─",
            SketchTool::ConstrainVertical => "│",
            SketchTool::ConstrainParallel => "∥",
            SketchTool::ConstrainPerpendicular => "⊥",
            SketchTool::ConstrainTangent => "⌢",
            SketchTool::ConstrainEqual => "=",
            SketchTool::ConstrainFixed => "⚓",
            SketchTool::DimensionDistance => "↔",
            SketchTool::DimensionHorizontal => "⟷",
            SketchTool::DimensionVertical => "⟷",
            SketchTool::DimensionAngle => "∠",
            SketchTool::DimensionRadius => "R",
        }
    }

    /// Check if this is a drawing tool
    pub fn is_drawing(&self) -> bool {
        matches!(
            self,
            SketchTool::Point
                | SketchTool::Line
                | SketchTool::RectangleCorner
                | SketchTool::RectangleCenter
                | SketchTool::Rectangle3Point
                | SketchTool::CircleCenterRadius
                | SketchTool::Circle2Point
                | SketchTool::Circle3Point
                | SketchTool::ArcCenterStartEnd
                | SketchTool::Arc3Point
        )
    }

    /// Check if this is a constraint tool
    pub fn is_constraint(&self) -> bool {
        matches!(
            self,
            SketchTool::ConstrainCoincident
                | SketchTool::ConstrainHorizontal
                | SketchTool::ConstrainVertical
                | SketchTool::ConstrainParallel
                | SketchTool::ConstrainPerpendicular
                | SketchTool::ConstrainTangent
                | SketchTool::ConstrainEqual
                | SketchTool::ConstrainFixed
                | SketchTool::DimensionDistance
                | SketchTool::DimensionHorizontal
                | SketchTool::DimensionVertical
                | SketchTool::DimensionAngle
                | SketchTool::DimensionRadius
        )
    }

    /// Check if this is a dimension tool
    pub fn is_dimension(&self) -> bool {
        matches!(
            self,
            SketchTool::DimensionDistance
                | SketchTool::DimensionHorizontal
                | SketchTool::DimensionVertical
                | SketchTool::DimensionAngle
                | SketchTool::DimensionRadius
        )
    }
}
