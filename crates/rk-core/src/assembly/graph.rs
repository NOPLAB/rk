//! Graph operations for Assembly (add, remove, connect, disconnect)

use uuid::Uuid;

use super::{Assembly, AssemblyError, Joint};

impl Assembly {
    /// Add a link to the assembly (does not automatically set as root)
    pub fn add_link(&mut self, link: super::Link) -> Uuid {
        let id = link.id;
        self.link_name_index.insert(link.name.clone(), id);
        self.links.insert(id, link);
        self.invalidate_cache();
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

        // Clean up children references
        for children in self.children.values_mut() {
            children.retain(|(_, child_id)| !to_remove.contains(child_id));
        }

        self.invalidate_cache();
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

        self.invalidate_cache();
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

        self.invalidate_cache();
        Ok(joint)
    }

    /// Check if connecting parent to child would create a cycle
    pub(crate) fn would_create_cycle(&self, parent_id: Uuid, child_id: Uuid) -> bool {
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
}
