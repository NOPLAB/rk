//! Assembly (scene graph) for robot structure

use std::collections::{HashMap, HashSet};

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::inertia::InertiaMatrix;
use crate::part::{JointLimits, JointType, Part};

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
    /// Visual properties
    pub visual: VisualProperties,
    /// Collision properties
    pub collision: CollisionProperties,
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
            visual: VisualProperties::default(),
            collision: CollisionProperties::default(),
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
            visual: VisualProperties {
                origin: Pose::default(),
                color: part.color,
                material_name: part.material_name.clone(),
                elements: Vec::new(),
            },
            collision: CollisionProperties::default(),
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
    /// Geometry type (for primitive shapes)
    pub geometry: Option<GeometryType>,
}

impl Default for VisualElement {
    fn default() -> Self {
        Self {
            name: None,
            origin: Pose::default(),
            color: [0.5, 0.5, 0.5, 1.0],
            material_name: None,
            geometry: None,
        }
    }
}

/// Single collision element for a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionElement {
    /// Optional name for this collision element
    pub name: Option<String>,
    pub origin: Pose,
    /// Geometry type (for primitive shapes)
    pub geometry: Option<GeometryType>,
}

impl Default for CollisionElement {
    fn default() -> Self {
        Self {
            name: None,
            origin: Pose::default(),
            geometry: None,
        }
    }
}

/// Geometry type for visual/collision elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeometryType {
    Mesh,
    Box { size: [f32; 3] },
    Cylinder { radius: f32, length: f32 },
    Sphere { radius: f32 },
    Capsule { radius: f32, length: f32 },
}

/// Visual properties for a link (supports multiple visual elements)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualProperties {
    /// Primary visual element (for backward compatibility)
    pub origin: Pose,
    pub color: [f32; 4],
    pub material_name: Option<String>,
    /// Additional visual elements (if any)
    #[serde(default)]
    pub elements: Vec<VisualElement>,
}

impl Default for VisualProperties {
    fn default() -> Self {
        Self {
            origin: Pose::default(),
            color: [0.5, 0.5, 0.5, 1.0],
            material_name: None,
            elements: Vec::new(),
        }
    }
}

impl VisualProperties {
    /// Get all visual elements (including primary)
    pub fn all_elements(&self) -> Vec<VisualElement> {
        let mut result = vec![VisualElement {
            name: None,
            origin: self.origin,
            color: self.color,
            material_name: self.material_name.clone(),
            geometry: None,
        }];
        result.extend(self.elements.clone());
        result
    }
}

/// Collision properties for a link (supports multiple collision elements)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionProperties {
    /// Primary collision element (for backward compatibility)
    pub origin: Pose,
    /// Additional collision elements (if any)
    #[serde(default)]
    pub elements: Vec<CollisionElement>,
}

impl Default for CollisionProperties {
    fn default() -> Self {
        Self {
            origin: Pose::default(),
            elements: Vec::new(),
        }
    }
}

impl CollisionProperties {
    /// Get all collision elements (including primary)
    pub fn all_elements(&self) -> Vec<CollisionElement> {
        let mut result = vec![CollisionElement {
            name: None,
            origin: self.origin,
            geometry: None,
        }];
        result.extend(self.elements.clone());
        result
    }
}

/// Inertial properties for a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InertialProperties {
    pub origin: Pose,
    pub mass: f32,
    pub inertia: InertiaMatrix,
}

/// Pose (position and orientation)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Pose {
    pub xyz: [f32; 3],
    pub rpy: [f32; 3], // roll, pitch, yaw in radians
}

impl Pose {
    pub fn new(xyz: [f32; 3], rpy: [f32; 3]) -> Self {
        Self { xyz, rpy }
    }

    pub fn from_position(xyz: [f32; 3]) -> Self {
        Self { xyz, rpy: [0.0; 3] }
    }

    pub fn to_mat4(&self) -> Mat4 {
        let translation = Vec3::from(self.xyz);
        let rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rpy[0],
            self.rpy[1],
            self.rpy[2],
        );
        Mat4::from_rotation_translation(rotation, translation)
    }
}

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
    /// Which joint point on parent was used
    pub parent_joint_point: Option<Uuid>,
    /// Which joint point on child was used
    pub child_joint_point: Option<Uuid>,
}

/// Joint mimic configuration
/// Makes this joint follow another joint's position: value = multiplier * other_joint + offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointMimic {
    /// Name of the joint to mimic
    pub joint: String,
    /// Multiplier applied to the mimicked joint's position (default: 1.0)
    pub multiplier: f32,
    /// Offset added after multiplication (default: 0.0)
    pub offset: f32,
}

impl Default for JointMimic {
    fn default() -> Self {
        Self {
            joint: String::new(),
            multiplier: 1.0,
            offset: 0.0,
        }
    }
}

impl JointMimic {
    /// Create a new mimic configuration
    pub fn new(joint: impl Into<String>) -> Self {
        Self {
            joint: joint.into(),
            multiplier: 1.0,
            offset: 0.0,
        }
    }

    /// Create a new mimic configuration with multiplier and offset
    pub fn with_params(joint: impl Into<String>, multiplier: f32, offset: f32) -> Self {
        Self {
            joint: joint.into(),
            multiplier,
            offset,
        }
    }

    /// Calculate the mimic value from the source joint's position
    pub fn calculate(&self, source_position: f32) -> f32 {
        self.multiplier * source_position + self.offset
    }
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
            parent_joint_point: None,
            child_joint_point: None,
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
            parent_joint_point: None,
            child_joint_point: None,
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

/// Robot assembly (scene graph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assembly {
    pub name: String,
    /// Root link ID
    pub root_link: Option<Uuid>,
    /// All links
    pub links: HashMap<Uuid, Link>,
    /// All joints
    pub joints: HashMap<Uuid, Joint>,
    /// Children mapping: parent_link -> [(joint_id, child_link)]
    pub children: HashMap<Uuid, Vec<(Uuid, Uuid)>>,
    /// Parent mapping: child_link -> (joint_id, parent_link)
    pub parent: HashMap<Uuid, (Uuid, Uuid)>,
}

impl Default for Assembly {
    fn default() -> Self {
        Self::new("robot")
    }
}

impl Assembly {
    /// Create a new empty assembly
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            root_link: None,
            links: HashMap::new(),
            joints: HashMap::new(),
            children: HashMap::new(),
            parent: HashMap::new(),
        }
    }

    /// Add a link to the assembly (does not automatically set as root)
    pub fn add_link(&mut self, link: Link) -> Uuid {
        let id = link.id;
        self.links.insert(id, link);
        id
    }

    /// Remove a link and all its children
    pub fn remove_link(&mut self, id: Uuid) -> Result<(), AssemblyError> {
        if !self.links.contains_key(&id) {
            return Err(AssemblyError::LinkNotFound(id));
        }

        // Collect all descendants
        let mut to_remove = vec![id];
        let mut i = 0;
        while i < to_remove.len() {
            let link_id = to_remove[i];
            if let Some(children) = self.children.get(&link_id) {
                for (_, child_id) in children {
                    to_remove.push(*child_id);
                }
            }
            i += 1;
        }

        // Remove all collected links and their joints
        for link_id in &to_remove {
            self.links.remove(link_id);
            self.children.remove(link_id);
            if let Some((joint_id, _)) = self.parent.remove(link_id) {
                self.joints.remove(&joint_id);
            }
        }

        // Clean up children references
        for children in self.children.values_mut() {
            children.retain(|(_, child_id)| !to_remove.contains(child_id));
        }

        // Update root if needed
        if self.root_link == Some(id) {
            self.root_link = self.links.keys().next().copied();
        }

        Ok(())
    }

    /// Connect two links with a joint
    pub fn connect(
        &mut self,
        parent_id: Uuid,
        child_id: Uuid,
        joint: Joint,
    ) -> Result<Uuid, AssemblyError> {
        // Validate links exist
        if !self.links.contains_key(&parent_id) {
            return Err(AssemblyError::LinkNotFound(parent_id));
        }
        if !self.links.contains_key(&child_id) {
            return Err(AssemblyError::LinkNotFound(child_id));
        }

        // Check for cycles
        if self.would_create_cycle(parent_id, child_id) {
            return Err(AssemblyError::WouldCreateCycle);
        }

        // Check if child already has a parent
        if self.parent.contains_key(&child_id) {
            return Err(AssemblyError::AlreadyHasParent(child_id));
        }

        let joint_id = joint.id;

        // Add joint
        self.joints.insert(joint_id, joint);

        // Update mappings
        self.children
            .entry(parent_id)
            .or_default()
            .push((joint_id, child_id));
        self.parent.insert(child_id, (joint_id, parent_id));

        // Update root if child was root
        if self.root_link == Some(child_id) {
            self.root_link = Some(parent_id);
        }

        Ok(joint_id)
    }

    /// Disconnect a link from its parent
    pub fn disconnect(&mut self, child_id: Uuid) -> Result<Joint, AssemblyError> {
        let (joint_id, parent_id) = self
            .parent
            .remove(&child_id)
            .ok_or(AssemblyError::NoParent(child_id))?;

        // Remove from children
        if let Some(children) = self.children.get_mut(&parent_id) {
            children.retain(|(_, cid)| *cid != child_id);
        }

        // Remove joint
        let joint = self
            .joints
            .remove(&joint_id)
            .ok_or(AssemblyError::JointNotFound(joint_id))?;

        Ok(joint)
    }

    /// Check if connecting parent to child would create a cycle
    fn would_create_cycle(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        // Check if child is an ancestor of parent
        let mut current = Some(parent_id);
        while let Some(id) = current {
            if id == child_id {
                return true;
            }
            current = self.parent.get(&id).map(|(_, p)| *p);
        }
        false
    }

    /// Get the world transform of a link
    pub fn get_world_transform(&self, link_id: Uuid) -> Mat4 {
        let mut transform = Mat4::IDENTITY;
        let mut current = Some(link_id);

        // Build transform chain from root to link
        let mut chain = Vec::new();
        while let Some(id) = current {
            chain.push(id);
            current = self.parent.get(&id).map(|(_, p)| *p);
        }

        // Apply transforms from root to link
        for id in chain.into_iter().rev() {
            if let Some((joint_id, _)) = self.parent.get(&id) {
                if let Some(joint) = self.joints.get(joint_id) {
                    transform = transform * joint.origin.to_mat4();
                }
            }
        }

        transform
    }

    /// Update all world transforms
    pub fn update_world_transforms(&mut self) {
        if let Some(root_id) = self.root_link {
            self.update_transform_recursive(root_id, Mat4::IDENTITY);
        }
    }

    fn update_transform_recursive(&mut self, link_id: Uuid, parent_transform: Mat4) {
        let transform = if let Some((joint_id, _)) = self.parent.get(&link_id) {
            if let Some(joint) = self.joints.get(joint_id) {
                parent_transform * joint.origin.to_mat4()
            } else {
                parent_transform
            }
        } else {
            parent_transform
        };

        if let Some(link) = self.links.get_mut(&link_id) {
            link.world_transform = transform;
        }

        // Get children IDs first to avoid borrow issues
        let children: Vec<Uuid> = self
            .children
            .get(&link_id)
            .map(|c| c.iter().map(|(_, child_id)| *child_id).collect())
            .unwrap_or_default();

        for child_id in children {
            self.update_transform_recursive(child_id, transform);
        }
    }

    /// Validate the assembly
    pub fn validate(&self) -> Result<(), Vec<AssemblyError>> {
        let mut errors = Vec::new();

        // Check for root
        if self.root_link.is_none() && !self.links.is_empty() {
            errors.push(AssemblyError::NoRoot);
        }

        // Check for orphaned links
        let mut reachable = HashSet::new();
        if let Some(root_id) = self.root_link {
            self.collect_reachable(root_id, &mut reachable);
        }

        for link_id in self.links.keys() {
            if !reachable.contains(link_id) {
                errors.push(AssemblyError::OrphanedLink(*link_id));
            }
        }

        // Check joint references
        for joint in self.joints.values() {
            if !self.links.contains_key(&joint.parent_link) {
                errors.push(AssemblyError::InvalidJointReference(joint.id, joint.parent_link));
            }
            if !self.links.contains_key(&joint.child_link) {
                errors.push(AssemblyError::InvalidJointReference(joint.id, joint.child_link));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn collect_reachable(&self, link_id: Uuid, reachable: &mut HashSet<Uuid>) {
        if !reachable.insert(link_id) {
            return; // Already visited
        }
        if let Some(children) = self.children.get(&link_id) {
            for (_, child_id) in children {
                self.collect_reachable(*child_id, reachable);
            }
        }
    }

    /// Get all links in depth-first order from root
    pub fn links_depth_first(&self) -> Vec<Uuid> {
        let mut result = Vec::new();
        if let Some(root_id) = self.root_link {
            self.collect_depth_first(root_id, &mut result);
        }
        result
    }

    fn collect_depth_first(&self, link_id: Uuid, result: &mut Vec<Uuid>) {
        result.push(link_id);
        if let Some(children) = self.children.get(&link_id) {
            for (_, child_id) in children {
                self.collect_depth_first(*child_id, result);
            }
        }
    }
}

/// Assembly-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AssemblyError {
    #[error("Link not found: {0}")]
    LinkNotFound(Uuid),
    #[error("Joint not found: {0}")]
    JointNotFound(Uuid),
    #[error("Connection would create a cycle")]
    WouldCreateCycle,
    #[error("Link already has a parent: {0}")]
    AlreadyHasParent(Uuid),
    #[error("Link has no parent: {0}")]
    NoParent(Uuid),
    #[error("No root link defined")]
    NoRoot,
    #[error("Orphaned link: {0}")]
    OrphanedLink(Uuid),
    #[error("Invalid joint reference: joint {0} references non-existent link {1}")]
    InvalidJointReference(Uuid, Uuid),
}
