//! Link and element types for robot assembly

use glam::Mat4;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::inertia::InertiaMatrix;
use crate::part::Part;
use crate::types::{GeometryType, Pose};

/// A link in the robot assembly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: Uuid,
    pub name: String,
    /// Reference to the part this link uses (None for empty links)
    pub part_id: Option<Uuid>,
    /// Transform of this link in world space (computed)
    #[serde(skip)]
    pub world_transform: Mat4,
    /// Visual elements
    pub visuals: Vec<VisualElement>,
    /// Collision elements
    pub collisions: Vec<CollisionElement>,
    /// Inertial properties
    pub inertial: InertialProperties,
}

impl Link {
    /// Create a new empty link (no geometry)
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            part_id: None,
            world_transform: Mat4::IDENTITY,
            visuals: Vec::new(),
            collisions: Vec::new(),
            inertial: InertialProperties {
                origin: Pose::default(),
                mass: 0.0,
                inertia: InertiaMatrix::default(),
            },
        }
    }

    /// Create a new link from a part
    pub fn from_part(part: &Part) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: part.name.clone(),
            part_id: Some(part.id),
            world_transform: Mat4::IDENTITY,
            visuals: vec![VisualElement {
                name: None,
                origin: Pose::default(),
                color: part.color,
                material_name: part.material_name.clone(),
                texture: None,
                geometry: GeometryType::Mesh {
                    path: None,
                    scale: None,
                },
            }],
            collisions: vec![CollisionElement::default()],
            inertial: InertialProperties {
                origin: Pose::default(),
                mass: part.mass,
                inertia: part.inertia,
            },
        }
    }
}

/// Single visual element for a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualElement {
    /// Optional name for this visual element
    pub name: Option<String>,
    pub origin: Pose,
    pub color: [f32; 4],
    pub material_name: Option<String>,
    /// Texture filename (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    /// Geometry type
    pub geometry: GeometryType,
}

impl Default for VisualElement {
    fn default() -> Self {
        Self {
            name: None,
            origin: Pose::default(),
            color: [0.5, 0.5, 0.5, 1.0],
            material_name: None,
            texture: None,
            geometry: GeometryType::Mesh {
                path: None,
                scale: None,
            },
        }
    }
}

/// Single collision element for a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionElement {
    /// Optional name for this collision element
    pub name: Option<String>,
    pub origin: Pose,
    /// Geometry type
    pub geometry: GeometryType,
}

impl Default for CollisionElement {
    fn default() -> Self {
        Self {
            name: None,
            origin: Pose::default(),
            geometry: GeometryType::Box {
                size: [0.1, 0.1, 0.1],
            },
        }
    }
}

/// Inertial properties for a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InertialProperties {
    pub origin: Pose,
    pub mass: f32,
    pub inertia: InertiaMatrix,
}
