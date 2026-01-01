//! Part and JointPoint definitions

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::inertia::InertiaMatrix;

/// Maximum number of joint points per part
pub const MAX_JOINT_POINTS: usize = 8;

/// A part loaded from an STL file with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub id: Uuid,
    pub name: String,
    /// Original STL file path (for re-export)
    pub stl_path: Option<String>,
    /// Triangle vertices (3 floats per vertex, 3 vertices per triangle)
    pub vertices: Vec<[f32; 3]>,
    /// Triangle normals (one per triangle)
    pub normals: Vec<[f32; 3]>,
    /// Indices for indexed rendering
    pub indices: Vec<u32>,
    /// Transform applied to original mesh (origin adjustment)
    pub origin_transform: Mat4,
    /// Joint connection points (max 8)
    pub joint_points: Vec<JointPoint>,
    /// Mass in kg
    pub mass: f32,
    /// Inertia tensor
    pub inertia: InertiaMatrix,
    /// Bounding box min
    pub bbox_min: [f32; 3],
    /// Bounding box max
    pub bbox_max: [f32; 3],
    /// Material color (RGBA)
    pub color: [f32; 4],
    /// Material name for URDF
    pub material_name: Option<String>,
    /// Mirror pair information
    pub mirror_pair: Option<MirrorPair>,
}

impl Part {
    /// Create a new empty part
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            stl_path: None,
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            origin_transform: Mat4::IDENTITY,
            joint_points: Vec::new(),
            mass: 1.0,
            inertia: InertiaMatrix::default(),
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            color: [0.7, 0.7, 0.7, 1.0],
            material_name: None,
            mirror_pair: None,
        }
    }

    /// Calculate bounding box from vertices
    pub fn calculate_bounding_box(&mut self) {
        if self.vertices.is_empty() {
            self.bbox_min = [0.0; 3];
            self.bbox_max = [0.0; 3];
            return;
        }

        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        for v in &self.vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }

        self.bbox_min = min;
        self.bbox_max = max;
    }

    /// Get the center of the bounding box
    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.bbox_min[0] + self.bbox_max[0]) / 2.0,
            (self.bbox_min[1] + self.bbox_max[1]) / 2.0,
            (self.bbox_min[2] + self.bbox_max[2]) / 2.0,
        )
    }

    /// Get the size of the bounding box
    pub fn size(&self) -> Vec3 {
        Vec3::new(
            self.bbox_max[0] - self.bbox_min[0],
            self.bbox_max[1] - self.bbox_min[1],
            self.bbox_max[2] - self.bbox_min[2],
        )
    }

    /// Add a joint point
    pub fn add_joint_point(&mut self, point: JointPoint) -> Result<(), PartError> {
        if self.joint_points.len() >= MAX_JOINT_POINTS {
            return Err(PartError::MaxJointPointsReached);
        }
        self.joint_points.push(point);
        Ok(())
    }

    /// Remove a joint point by ID
    pub fn remove_joint_point(&mut self, id: Uuid) -> Option<JointPoint> {
        if let Some(pos) = self.joint_points.iter().position(|p| p.id == id) {
            Some(self.joint_points.remove(pos))
        } else {
            None
        }
    }

    /// Get a joint point by ID
    pub fn get_joint_point(&self, id: Uuid) -> Option<&JointPoint> {
        self.joint_points.iter().find(|p| p.id == id)
    }

    /// Get a mutable joint point by ID
    pub fn get_joint_point_mut(&mut self, id: Uuid) -> Option<&mut JointPoint> {
        self.joint_points.iter_mut().find(|p| p.id == id)
    }
}

/// Joint connection point on a part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointPoint {
    pub id: Uuid,
    pub name: String,
    /// Position relative to part origin
    pub position: Vec3,
    /// Orientation relative to part
    pub orientation: Quat,
    /// Joint type
    pub joint_type: JointType,
    /// Joint axis (for revolute/prismatic)
    pub axis: Vec3,
    /// Joint limits
    pub limits: Option<JointLimits>,
}

impl JointPoint {
    /// Create a new joint point at the given position
    pub fn new(name: impl Into<String>, position: Vec3) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            position,
            orientation: Quat::IDENTITY,
            joint_type: JointType::Fixed,
            axis: Vec3::Z,
            limits: None,
        }
    }

    /// Create a revolute joint point
    pub fn revolute(name: impl Into<String>, position: Vec3, axis: Vec3) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            position,
            orientation: Quat::IDENTITY,
            joint_type: JointType::Revolute,
            axis: axis.normalize(),
            limits: Some(JointLimits::default()),
        }
    }
}

/// Joint type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JointType {
    #[default]
    Fixed,
    Revolute,
    Continuous,
    Prismatic,
    Floating,
    Planar,
}

impl JointType {
    /// Check if this joint type has an axis
    pub fn has_axis(&self) -> bool {
        matches!(self, JointType::Revolute | JointType::Continuous | JointType::Prismatic)
    }

    /// Check if this joint type has limits
    pub fn has_limits(&self) -> bool {
        matches!(self, JointType::Revolute | JointType::Prismatic)
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            JointType::Fixed => "Fixed",
            JointType::Revolute => "Revolute",
            JointType::Continuous => "Continuous",
            JointType::Prismatic => "Prismatic",
            JointType::Floating => "Floating",
            JointType::Planar => "Planar",
        }
    }

    /// All joint types for UI
    pub fn all() -> &'static [JointType] {
        &[
            JointType::Fixed,
            JointType::Revolute,
            JointType::Continuous,
            JointType::Prismatic,
            JointType::Floating,
            JointType::Planar,
        ]
    }
}

/// Joint limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointLimits {
    /// Lower position limit (rad or m)
    pub lower: f32,
    /// Upper position limit (rad or m)
    pub upper: f32,
    /// Maximum effort (N or Nm)
    pub effort: f32,
    /// Maximum velocity (rad/s or m/s)
    pub velocity: f32,
}

impl Default for JointLimits {
    fn default() -> Self {
        Self {
            lower: -std::f32::consts::PI,
            upper: std::f32::consts::PI,
            effort: 100.0,
            velocity: 1.0,
        }
    }
}

/// Mirror pair information for symmetric parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorPair {
    /// Partner part ID (if exists)
    pub partner_id: Option<Uuid>,
    /// Which side this part is
    pub side: MirrorSide,
}

/// Mirror side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirrorSide {
    Left,
    Right,
}

/// Part-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum PartError {
    #[error("Maximum number of joint points ({}) reached", MAX_JOINT_POINTS)]
    MaxJointPointsReached,
    #[error("Joint point not found: {0}")]
    JointPointNotFound(Uuid),
}

