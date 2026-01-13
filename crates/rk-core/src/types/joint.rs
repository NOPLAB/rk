//! Joint-related type definitions

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        matches!(
            self,
            JointType::Revolute | JointType::Continuous | JointType::Prismatic
        )
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

impl From<&urdf_rs::JointType> for JointType {
    fn from(urdf_type: &urdf_rs::JointType) -> Self {
        match urdf_type {
            urdf_rs::JointType::Fixed => JointType::Fixed,
            urdf_rs::JointType::Revolute => JointType::Revolute,
            urdf_rs::JointType::Continuous => JointType::Continuous,
            urdf_rs::JointType::Prismatic => JointType::Prismatic,
            urdf_rs::JointType::Floating => JointType::Floating,
            urdf_rs::JointType::Planar => JointType::Planar,
            urdf_rs::JointType::Spherical => JointType::Floating, // Approximate as floating
        }
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

impl JointLimits {
    /// Create default limits for revolute joints (-PI to PI)
    pub fn default_revolute() -> Self {
        Self::default()
    }

    /// Create default limits for prismatic joints (-1m to 1m)
    pub fn default_prismatic() -> Self {
        Self {
            lower: -1.0,
            upper: 1.0,
            effort: 100.0,
            velocity: 1.0,
        }
    }

    /// Create limits with specified range
    pub fn with_range(lower: f32, upper: f32) -> Self {
        Self {
            lower,
            upper,
            ..Self::default()
        }
    }
}

/// Joint dynamics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointDynamics {
    pub damping: f32,
    pub friction: f32,
}

impl Default for JointDynamics {
    fn default() -> Self {
        Self {
            damping: 0.0,
            friction: 0.0,
        }
    }
}

/// Joint mimic configuration
/// Makes this joint follow another joint's position: value = multiplier * other_joint + offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointMimic {
    /// ID of the joint to mimic
    pub joint_id: Uuid,
    /// Multiplier applied to the mimicked joint's position (default: 1.0)
    pub multiplier: f32,
    /// Offset added after multiplication (default: 0.0)
    pub offset: f32,
}

impl JointMimic {
    /// Create a new mimic configuration
    pub fn new(joint_id: Uuid) -> Self {
        Self {
            joint_id,
            multiplier: 1.0,
            offset: 0.0,
        }
    }

    /// Create a new mimic configuration with multiplier and offset
    pub fn with_params(joint_id: Uuid, multiplier: f32, offset: f32) -> Self {
        Self {
            joint_id,
            multiplier,
            offset,
        }
    }

    /// Calculate the mimic value from the source joint's position
    pub fn calculate(&self, source_position: f32) -> f32 {
        self.multiplier * source_position + self.offset
    }
}
