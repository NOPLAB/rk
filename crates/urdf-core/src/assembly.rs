//! Assembly (scene graph) for robot structure

use std::collections::{HashMap, HashSet};

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::geometry::GeometryType;
use crate::inertia::InertiaMatrix;
use crate::part::{JointLimits, JointPoint, JointType, Part};

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
                geometry: GeometryType::Mesh { path: None },
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
            geometry: GeometryType::Mesh { path: None },
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
            geometry: GeometryType::Mesh { path: None },
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
        let rotation = Quat::from_euler(glam::EulerRot::XYZ, self.rpy[0], self.rpy[1], self.rpy[2]);
        Mat4::from_rotation_translation(rotation, translation)
    }

    /// Convert to quaternion representation
    pub fn to_quat(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::XYZ, self.rpy[0], self.rpy[1], self.rpy[2])
    }

    /// Get position as Vec3
    pub fn position(&self) -> Vec3 {
        Vec3::from(self.xyz)
    }
}

impl From<&urdf_rs::Pose> for Pose {
    fn from(urdf_pose: &urdf_rs::Pose) -> Self {
        Self {
            xyz: [
                urdf_pose.xyz.0[0] as f32,
                urdf_pose.xyz.0[1] as f32,
                urdf_pose.xyz.0[2] as f32,
            ],
            rpy: [
                urdf_pose.rpy.0[0] as f32,
                urdf_pose.rpy.0[1] as f32,
                urdf_pose.rpy.0[2] as f32,
            ],
        }
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
    parent_joint_point: Option<Uuid>,
    child_joint_point: Option<Uuid>,
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
            parent_joint_point: None,
            child_joint_point: None,
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

    /// Set the parent joint point reference
    pub fn parent_joint_point(mut self, point_id: Uuid) -> Self {
        self.parent_joint_point = Some(point_id);
        self
    }

    /// Set the child joint point reference
    pub fn child_joint_point(mut self, point_id: Uuid) -> Self {
        self.child_joint_point = Some(point_id);
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
            parent_joint_point: self.parent_joint_point,
            child_joint_point: self.child_joint_point,
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
    /// All joint points (managed by assembly, not by parts)
    pub joint_points: HashMap<Uuid, JointPoint>,
    /// Children mapping: parent_link -> [(joint_id, child_link)]
    pub children: HashMap<Uuid, Vec<(Uuid, Uuid)>>,
    /// Parent mapping: child_link -> (joint_id, parent_link)
    pub parent: HashMap<Uuid, (Uuid, Uuid)>,
    /// Name to ID index for links (O(1) lookup)
    #[serde(skip)]
    link_name_index: HashMap<String, Uuid>,
    /// Name to ID index for joints (O(1) lookup)
    #[serde(skip)]
    joint_name_index: HashMap<String, Uuid>,
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
            joint_points: HashMap::new(),
            children: HashMap::new(),
            parent: HashMap::new(),
            link_name_index: HashMap::new(),
            joint_name_index: HashMap::new(),
        }
    }

    /// Rebuild name indices (call after deserialization)
    pub fn rebuild_indices(&mut self) {
        self.link_name_index.clear();
        self.joint_name_index.clear();
        for (id, link) in &self.links {
            self.link_name_index.insert(link.name.clone(), *id);
        }
        for (id, joint) in &self.joints {
            self.joint_name_index.insert(joint.name.clone(), *id);
        }
    }

    /// Add a link to the assembly (does not automatically set as root)
    pub fn add_link(&mut self, link: Link) -> Uuid {
        let id = link.id;
        self.link_name_index.insert(link.name.clone(), id);
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

        // Collect part IDs for joint point cleanup
        let part_ids: Vec<Uuid> = to_remove
            .iter()
            .filter_map(|link_id| self.links.get(link_id).and_then(|l| l.part_id))
            .collect();

        // Remove all collected links and their joints
        for link_id in &to_remove {
            if let Some(link) = self.links.remove(link_id) {
                self.link_name_index.remove(&link.name);
            }
            self.children.remove(link_id);
            if let Some((joint_id, _)) = self.parent.remove(link_id)
                && let Some(joint) = self.joints.remove(&joint_id)
            {
                self.joint_name_index.remove(&joint.name);
            }
        }

        // Remove associated joint points
        self.joint_points
            .retain(|_, jp| !part_ids.contains(&jp.part_id));

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

        // Add joint and update name index
        self.joint_name_index.insert(joint.name.clone(), joint_id);
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

        // Remove joint and update name index
        let joint = self
            .joints
            .remove(&joint_id)
            .ok_or(AssemblyError::JointNotFound(joint_id))?;
        self.joint_name_index.remove(&joint.name);

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
            if let Some((joint_id, _)) = self.parent.get(&id)
                && let Some(joint) = self.joints.get(joint_id)
            {
                transform *= joint.origin.to_mat4();
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

    /// Update all world transforms with joint positions applied
    pub fn update_world_transforms_with_positions(&mut self, joint_positions: &HashMap<Uuid, f32>) {
        if let Some(root_id) = self.root_link {
            self.update_transform_recursive_with_positions(
                root_id,
                Mat4::IDENTITY,
                joint_positions,
            );
        }
    }

    fn update_transform_recursive_with_positions(
        &mut self,
        link_id: Uuid,
        parent_transform: Mat4,
        joint_positions: &HashMap<Uuid, f32>,
    ) {
        let transform = if let Some((joint_id, _)) = self.parent.get(&link_id) {
            if let Some(joint) = self.joints.get(joint_id) {
                // Get joint position (defaults to 0)
                let position = joint_positions.get(joint_id).copied().unwrap_or(0.0);
                // Compute joint transform with position
                let joint_transform =
                    Self::compute_joint_transform(&joint.joint_type, joint.axis, position);
                parent_transform * joint.origin.to_mat4() * joint_transform
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
            self.update_transform_recursive_with_positions(child_id, transform, joint_positions);
        }
    }

    /// Compute the transform for a joint at a given position
    pub fn compute_joint_transform(joint_type: &JointType, axis: Vec3, position: f32) -> Mat4 {
        match joint_type {
            JointType::Revolute | JointType::Continuous => {
                // Rotation around the joint axis
                let rotation = Quat::from_axis_angle(axis, position);
                Mat4::from_quat(rotation)
            }
            JointType::Prismatic => {
                // Translation along the joint axis
                let translation = axis * position;
                Mat4::from_translation(translation)
            }
            JointType::Fixed | JointType::Floating | JointType::Planar => {
                // No transform for fixed joints, floating/planar would need more DOFs
                Mat4::IDENTITY
            }
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
                errors.push(AssemblyError::InvalidJointReference(
                    joint.id,
                    joint.parent_link,
                ));
            }
            if !self.links.contains_key(&joint.child_link) {
                errors.push(AssemblyError::InvalidJointReference(
                    joint.id,
                    joint.child_link,
                ));
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

    // ============== Query Helpers ==============

    /// Get a link's name by ID
    pub fn get_link_name(&self, link_id: Uuid) -> Option<&str> {
        self.links.get(&link_id).map(|l| l.name.as_str())
    }

    /// Get a joint's name by ID
    pub fn get_joint_name(&self, joint_id: Uuid) -> Option<&str> {
        self.joints.get(&joint_id).map(|j| j.name.as_str())
    }

    /// Find a link by name (O(1) lookup)
    pub fn find_link_by_name(&self, name: &str) -> Option<&Link> {
        self.link_name_index
            .get(name)
            .and_then(|id| self.links.get(id))
    }

    /// Find a joint by name (O(1) lookup)
    pub fn find_joint_by_name(&self, name: &str) -> Option<&Joint> {
        self.joint_name_index
            .get(name)
            .and_then(|id| self.joints.get(id))
    }

    /// Find a link ID by name (O(1) lookup)
    pub fn find_link_id_by_name(&self, name: &str) -> Option<Uuid> {
        self.link_name_index.get(name).copied()
    }

    /// Find a joint ID by name (O(1) lookup)
    pub fn find_joint_id_by_name(&self, name: &str) -> Option<Uuid> {
        self.joint_name_index.get(name).copied()
    }

    /// Find a link by its associated part ID
    pub fn find_link_by_part(&self, part_id: Uuid) -> Option<&Link> {
        self.links.values().find(|l| l.part_id == Some(part_id))
    }

    /// Get the chain of link IDs from a link to the root
    pub fn get_chain_to_root(&self, link_id: Uuid) -> Vec<Uuid> {
        let mut chain = vec![link_id];
        let mut current = link_id;

        while let Some((_, parent_id)) = self.parent.get(&current) {
            chain.push(*parent_id);
            current = *parent_id;
        }

        chain
    }

    /// Get the joint connecting a link to its parent
    pub fn get_parent_joint(&self, link_id: Uuid) -> Option<&Joint> {
        self.parent
            .get(&link_id)
            .and_then(|(joint_id, _)| self.joints.get(joint_id))
    }

    /// Get the parent link ID of a given link
    pub fn get_parent_link_id(&self, link_id: Uuid) -> Option<Uuid> {
        self.parent.get(&link_id).map(|(_, parent_id)| *parent_id)
    }

    /// Get the parent link of a given link
    pub fn get_parent_link(&self, link_id: Uuid) -> Option<&Link> {
        self.parent
            .get(&link_id)
            .and_then(|(_, parent_id)| self.links.get(parent_id))
    }

    /// Get all direct children of a link (returns vec of (joint_id, child_link_id))
    pub fn get_children(&self, link_id: Uuid) -> Vec<(Uuid, Uuid)> {
        self.children.get(&link_id).cloned().unwrap_or_default()
    }

    /// Get all descendant link IDs (breadth-first)
    pub fn get_all_descendants(&self, link_id: Uuid) -> Vec<Uuid> {
        let mut descendants = Vec::new();
        let mut queue = vec![link_id];

        while let Some(current) = queue.pop() {
            if let Some(children) = self.children.get(&current) {
                for (_, child_id) in children {
                    descendants.push(*child_id);
                    queue.push(*child_id);
                }
            }
        }

        descendants
    }

    /// Check if a link is an ancestor of another
    pub fn is_ancestor(&self, ancestor_id: Uuid, descendant_id: Uuid) -> bool {
        let chain = self.get_chain_to_root(descendant_id);
        chain.contains(&ancestor_id)
    }

    /// Get link depth from root (root = 0)
    pub fn get_link_depth(&self, link_id: Uuid) -> usize {
        self.get_chain_to_root(link_id).len() - 1
    }

    /// Get all joints in the chain from a link to root
    pub fn get_joints_to_root(&self, link_id: Uuid) -> Vec<&Joint> {
        let mut joints = Vec::new();
        let mut current = link_id;

        while let Some((joint_id, parent_id)) = self.parent.get(&current) {
            if let Some(joint) = self.joints.get(joint_id) {
                joints.push(joint);
            }
            current = *parent_id;
        }

        joints
    }

    /// Get a link by ID (convenience method)
    pub fn get_link(&self, link_id: Uuid) -> Option<&Link> {
        self.links.get(&link_id)
    }

    /// Get a mutable link by ID
    pub fn get_link_mut(&mut self, link_id: Uuid) -> Option<&mut Link> {
        self.links.get_mut(&link_id)
    }

    /// Get a joint by ID (convenience method)
    pub fn get_joint(&self, joint_id: Uuid) -> Option<&Joint> {
        self.joints.get(&joint_id)
    }

    /// Get a mutable joint by ID
    pub fn get_joint_mut(&mut self, joint_id: Uuid) -> Option<&mut Joint> {
        self.joints.get_mut(&joint_id)
    }

    /// Count total number of links
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Count total number of joints
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Check if assembly is empty
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    // ============== Joint Point Management ==============

    /// Add a joint point to the assembly
    pub fn add_joint_point(&mut self, joint_point: JointPoint) -> Uuid {
        let id = joint_point.id;
        self.joint_points.insert(id, joint_point);
        id
    }

    /// Remove a joint point by ID
    pub fn remove_joint_point(&mut self, id: Uuid) -> Option<JointPoint> {
        self.joint_points.remove(&id)
    }

    /// Get a joint point by ID
    pub fn get_joint_point(&self, id: Uuid) -> Option<&JointPoint> {
        self.joint_points.get(&id)
    }

    /// Get a mutable joint point by ID
    pub fn get_joint_point_mut(&mut self, id: Uuid) -> Option<&mut JointPoint> {
        self.joint_points.get_mut(&id)
    }

    /// Get all joint points for a specific part
    pub fn get_joint_points_for_part(&self, part_id: Uuid) -> Vec<&JointPoint> {
        self.joint_points
            .values()
            .filter(|jp| jp.part_id == part_id)
            .collect()
    }

    /// Count total number of joint points
    pub fn joint_point_count(&self) -> usize {
        self.joint_points.len()
    }
}

/// Assembly-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AssemblyError {
    #[error("Link not found: {0}")]
    LinkNotFound(Uuid),
    #[error("Joint not found: {0}")]
    JointNotFound(Uuid),
    #[error("Joint point not found: {0}")]
    JointPointNotFound(Uuid),
    #[error("Part not found: {0}")]
    PartNotFound(Uuid),
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
