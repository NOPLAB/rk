//! Joint types and builder for robot assembly

use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{JointDynamics, JointLimits, JointMimic, JointType, Pose};

/// A joint connecting two links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint {
    pub id: Uuid,
    pub name: String,
    pub joint_type: JointType,
    /// Parent link ID
    pub parent_link: Uuid,
    /// Child link ID
    pub child_link: Uuid,
    /// Transform from parent link to joint origin
    pub origin: Pose,
    /// Joint axis (for revolute/prismatic)
    pub axis: Vec3,
    /// Joint limits
    pub limits: Option<JointLimits>,
    /// Joint dynamics
    pub dynamics: Option<JointDynamics>,
    /// Joint mimic configuration (follows another joint)
    pub mimic: Option<JointMimic>,
}

impl Joint {
    /// Create a new fixed joint
    pub fn fixed(name: impl Into<String>, parent: Uuid, child: Uuid, origin: Pose) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            joint_type: JointType::Fixed,
            parent_link: parent,
            child_link: child,
            origin,
            axis: Vec3::Z,
            limits: None,
            dynamics: None,
            mimic: None,
        }
    }

    /// Create a new revolute joint
    pub fn revolute(
        name: impl Into<String>,
        parent: Uuid,
        child: Uuid,
        origin: Pose,
        axis: Vec3,
        limits: JointLimits,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            joint_type: JointType::Revolute,
            parent_link: parent,
            child_link: child,
            origin,
            axis: axis.normalize(),
            limits: Some(limits),
            dynamics: None,
            mimic: None,
        }
    }

    /// Create a builder for constructing joints with fluent API
    pub fn builder(name: impl Into<String>, parent: Uuid, child: Uuid) -> JointBuilder {
        JointBuilder::new(name, parent, child)
    }
}

/// Builder for creating joints with fluent API
#[derive(Debug, Clone)]
pub struct JointBuilder {
    name: String,
    joint_type: JointType,
    parent_link: Uuid,
    child_link: Uuid,
    origin: Pose,
    axis: Vec3,
    limits: Option<JointLimits>,
    dynamics: Option<JointDynamics>,
    mimic: Option<JointMimic>,
}

impl JointBuilder {
    /// Create a new joint builder
    pub fn new(name: impl Into<String>, parent: Uuid, child: Uuid) -> Self {
        Self {
            name: name.into(),
            joint_type: JointType::Fixed,
            parent_link: parent,
            child_link: child,
            origin: Pose::default(),
            axis: Vec3::Z,
            limits: None,
            dynamics: None,
            mimic: None,
        }
    }

    /// Set the joint type
    pub fn joint_type(mut self, joint_type: JointType) -> Self {
        self.joint_type = joint_type;
        self
    }

    /// Set as a fixed joint
    pub fn fixed(mut self) -> Self {
        self.joint_type = JointType::Fixed;
        self
    }

    /// Set as a revolute joint with default limits
    pub fn revolute(mut self) -> Self {
        self.joint_type = JointType::Revolute;
        if self.limits.is_none() {
            self.limits = Some(JointLimits::default_revolute());
        }
        self
    }

    /// Set as a continuous joint
    pub fn continuous(mut self) -> Self {
        self.joint_type = JointType::Continuous;
        self
    }

    /// Set as a prismatic joint with default limits
    pub fn prismatic(mut self) -> Self {
        self.joint_type = JointType::Prismatic;
        if self.limits.is_none() {
            self.limits = Some(JointLimits::default_prismatic());
        }
        self
    }

    /// Set the joint origin
    pub fn origin(mut self, pose: Pose) -> Self {
        self.origin = pose;
        self
    }

    /// Set the joint origin position
    pub fn xyz(mut self, x: f32, y: f32, z: f32) -> Self {
        self.origin.xyz = [x, y, z];
        self
    }

    /// Set the joint origin rotation (roll, pitch, yaw)
    pub fn rpy(mut self, roll: f32, pitch: f32, yaw: f32) -> Self {
        self.origin.rpy = [roll, pitch, yaw];
        self
    }

    /// Set the joint axis
    pub fn axis(mut self, axis: Vec3) -> Self {
        self.axis = axis.normalize();
        self
    }

    /// Set the joint axis from x, y, z components
    pub fn axis_xyz(mut self, x: f32, y: f32, z: f32) -> Self {
        self.axis = Vec3::new(x, y, z).normalize();
        self
    }

    /// Set the joint limits
    pub fn limits(mut self, limits: JointLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Set joint limits with a range
    pub fn limits_range(mut self, lower: f32, upper: f32) -> Self {
        self.limits = Some(JointLimits::with_range(lower, upper));
        self
    }

    /// Set the joint dynamics
    pub fn dynamics(mut self, damping: f32, friction: f32) -> Self {
        self.dynamics = Some(JointDynamics { damping, friction });
        self
    }

    /// Set mimic configuration to follow another joint
    pub fn mimic(mut self, joint_id: Uuid) -> Self {
        self.mimic = Some(JointMimic::new(joint_id));
        self
    }

    /// Set mimic configuration with multiplier and offset
    pub fn mimic_with_params(mut self, joint_id: Uuid, multiplier: f32, offset: f32) -> Self {
        self.mimic = Some(JointMimic::with_params(joint_id, multiplier, offset));
        self
    }

    /// Build the joint
    pub fn build(self) -> Joint {
        Joint {
            id: Uuid::new_v4(),
            name: self.name,
            joint_type: self.joint_type,
            parent_link: self.parent_link,
            child_link: self.child_link,
            origin: self.origin,
            axis: self.axis,
            limits: self.limits,
            dynamics: self.dynamics,
            mimic: self.mimic,
        }
    }
}
