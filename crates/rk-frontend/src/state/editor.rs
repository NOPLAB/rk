//! Editor state types

/// Editor tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Select,
    Move,
    Rotate,
}

impl EditorTool {
    pub fn name(&self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Move => "Move",
            EditorTool::Rotate => "Rotate",
        }
    }
}

/// Primitive type for creating geometric shapes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Box,
    Cylinder,
    Sphere,
}

impl PrimitiveType {
    pub fn name(&self) -> &'static str {
        match self {
            PrimitiveType::Box => "Box",
            PrimitiveType::Cylinder => "Cylinder",
            PrimitiveType::Sphere => "Sphere",
        }
    }

    /// Engine primitive spec with the editor's default dimensions (0.1 m)
    pub fn to_spec(self) -> rk_engine::PrimitiveSpec {
        match self {
            PrimitiveType::Box => rk_engine::PrimitiveSpec::Box {
                size: [0.1, 0.1, 0.1],
            },
            PrimitiveType::Cylinder => rk_engine::PrimitiveSpec::Cylinder {
                radius: 0.05,
                height: 0.1,
            },
            PrimitiveType::Sphere => rk_engine::PrimitiveSpec::Sphere { radius: 0.05 },
        }
    }
}
