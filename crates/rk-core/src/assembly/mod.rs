//! Assembly (scene graph) for robot structure

mod graph;
mod joint;
mod queries;
mod transforms;
mod tree_cache;
mod types;

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use joint::{Joint, JointBuilder};
pub use types::{CollisionElement, InertialProperties, Link, VisualElement};
// Re-export from types for backwards compatibility
pub use crate::types::{JointDynamics, JointMimic, Pose};

use tree_cache::TreeCache;

/// Raw assembly data for deserialization (used internally)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssemblyData {
    name: String,
    links: HashMap<Uuid, Link>,
    joints: HashMap<Uuid, Joint>,
    children: HashMap<Uuid, Vec<(Uuid, Uuid)>>,
    parent: HashMap<Uuid, (Uuid, Uuid)>,
}

/// Robot assembly (scene graph)
#[derive(Debug, Clone, Serialize)]
#[serde(into = "AssemblyData")]
pub struct Assembly {
    pub name: String,
    /// All links
    pub links: HashMap<Uuid, Link>,
    /// All joints
    pub joints: HashMap<Uuid, Joint>,
    /// Children mapping: parent_link -> [(joint_id, child_link)]
    pub children: HashMap<Uuid, Vec<(Uuid, Uuid)>>,
    /// Parent mapping: child_link -> (joint_id, parent_link)
    pub parent: HashMap<Uuid, (Uuid, Uuid)>,
    /// Name to ID index for links (O(1) lookup)
    pub(crate) link_name_index: HashMap<String, Uuid>,
    /// Name to ID index for joints (O(1) lookup)
    pub(crate) joint_name_index: HashMap<String, Uuid>,
    /// Cached tree structure for efficient traversal (interior mutability for lazy evaluation)
    cache: RefCell<TreeCache>,
    /// Current joint positions (joint_id -> position in radians or meters)
    /// Runtime state only - not serialized
    pub joint_positions: HashMap<Uuid, f32>,
}

impl From<Assembly> for AssemblyData {
    fn from(assembly: Assembly) -> Self {
        Self {
            name: assembly.name,
            links: assembly.links,
            joints: assembly.joints,
            children: assembly.children,
            parent: assembly.parent,
        }
    }
}

impl From<AssemblyData> for Assembly {
    fn from(data: AssemblyData) -> Self {
        let mut assembly = Self {
            name: data.name,
            links: data.links,
            joints: data.joints,
            children: data.children,
            parent: data.parent,
            link_name_index: HashMap::new(),
            joint_name_index: HashMap::new(),
            cache: RefCell::new(TreeCache::default()),
            joint_positions: HashMap::new(),
        };
        assembly.rebuild_indices();
        assembly.update_world_transforms();
        assembly
    }
}

impl<'de> Deserialize<'de> for Assembly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = AssemblyData::deserialize(deserializer)?;
        Ok(Assembly::from(data))
    }
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
            links: HashMap::new(),
            joints: HashMap::new(),
            children: HashMap::new(),
            parent: HashMap::new(),
            link_name_index: HashMap::new(),
            joint_name_index: HashMap::new(),
            cache: RefCell::new(TreeCache::default()),
            joint_positions: HashMap::new(),
        }
    }

    /// Get all root links (links without parents)
    pub fn get_root_links(&self) -> Vec<Uuid> {
        self.ensure_cache_valid();
        self.cache.borrow().roots.clone()
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
        self.invalidate_cache();
    }

    /// Invalidate the tree cache (call after any structural change)
    pub(crate) fn invalidate_cache(&self) {
        self.cache.borrow_mut().invalidate();
    }

    /// Ensure the cache is valid, rebuilding if necessary
    pub(crate) fn ensure_cache_valid(&self) {
        let mut cache = self.cache.borrow_mut();
        if !cache.valid {
            cache.rebuild(&self.links, &self.parent, &self.children);
        }
    }

    /// Validate the assembly
    pub fn validate(&self) -> Result<(), Vec<AssemblyError>> {
        let mut errors = Vec::new();

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

    /// Get all links in depth-first order from all roots
    pub fn links_depth_first(&self) -> Vec<Uuid> {
        let mut result = Vec::new();
        for root_id in self.get_root_links() {
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

    // ============== Joint Position Management ==============

    /// Set a joint position (in radians for revolute, meters for prismatic)
    pub fn set_joint_position(&mut self, joint_id: Uuid, position: f32) {
        self.joint_positions.insert(joint_id, position);
    }

    /// Get a joint position (defaults to 0.0)
    pub fn get_joint_position(&self, joint_id: Uuid) -> f32 {
        self.joint_positions.get(&joint_id).copied().unwrap_or(0.0)
    }

    /// Reset a joint position to 0
    pub fn reset_joint_position(&mut self, joint_id: Uuid) {
        self.joint_positions.remove(&joint_id);
    }

    /// Reset all joint positions to 0
    pub fn reset_all_joint_positions(&mut self) {
        self.joint_positions.clear();
    }
}

/// Assembly-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AssemblyError {
    #[error("Link not found: {0}")]
    LinkNotFound(Uuid),
    #[error("Joint not found: {0}")]
    JointNotFound(Uuid),
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
