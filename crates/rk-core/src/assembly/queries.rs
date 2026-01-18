//! Query methods for Assembly

use uuid::Uuid;

use super::Assembly;
use super::joint::Joint;
use super::types::Link;

impl Assembly {
    // ============== Query Helpers ==============

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

    /// Find a mutable link by its associated part ID
    pub fn find_link_by_part_mut(&mut self, part_id: Uuid) -> Option<&mut Link> {
        self.links.values_mut().find(|l| l.part_id == Some(part_id))
    }

    /// Get the chain of link IDs from a link to the root
    pub fn get_chain_to_root(&self, link_id: Uuid) -> Vec<Uuid> {
        self.ensure_cache_valid();
        let cache = self.cache.borrow();
        if let Some(ancestors) = cache.ancestors.get(&link_id) {
            // Cache stores root-first order, we need link-first order
            ancestors.iter().rev().copied().collect()
        } else {
            // Link not in cache (shouldn't happen for valid links)
            vec![link_id]
        }
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

    /// Get all descendant link IDs
    pub fn get_all_descendants(&self, link_id: Uuid) -> Vec<Uuid> {
        self.ensure_cache_valid();
        self.cache
            .borrow()
            .descendants
            .get(&link_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a link is an ancestor of another
    pub fn is_ancestor(&self, ancestor_id: Uuid, descendant_id: Uuid) -> bool {
        self.ensure_cache_valid();
        let cache = self.cache.borrow();
        // Quick depth check: ancestor must have smaller depth
        if let (Some(&ancestor_depth), Some(&descendant_depth)) = (
            cache.depths.get(&ancestor_id),
            cache.depths.get(&descendant_id),
        ) && ancestor_depth >= descendant_depth
        {
            return ancestor_id == descendant_id;
        }
        // Check if ancestor is in the descendant's ancestor chain
        cache
            .ancestors
            .get(&descendant_id)
            .is_some_and(|chain| chain.contains(&ancestor_id))
    }

    /// Get link depth from root (root = 0)
    pub fn get_link_depth(&self, link_id: Uuid) -> usize {
        self.ensure_cache_valid();
        self.cache
            .borrow()
            .depths
            .get(&link_id)
            .copied()
            .unwrap_or(0)
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

    // ============== Extended Query API ==============

    /// Get all leaf links (links with no children)
    pub fn get_leaf_links(&self) -> Vec<Uuid> {
        self.links
            .keys()
            .filter(|id| self.children.get(id).map(|c| c.is_empty()).unwrap_or(true))
            .copied()
            .collect()
    }

    /// Get subtree size (count of link + all descendants)
    pub fn get_subtree_size(&self, link_id: Uuid) -> usize {
        1 + self.get_all_descendants(link_id).len()
    }

    /// Find the common ancestor of two links (None if no common ancestor)
    pub fn find_common_ancestor(&self, a: Uuid, b: Uuid) -> Option<Uuid> {
        self.ensure_cache_valid();
        let cache = self.cache.borrow();

        let ancestors_a = cache.ancestors.get(&a)?;
        let ancestors_b = cache.ancestors.get(&b)?;

        // Find the deepest common ancestor (iterate from root to leaf order)
        let mut common = None;
        for (ancestor_a, ancestor_b) in ancestors_a.iter().zip(ancestors_b.iter()) {
            if ancestor_a == ancestor_b {
                common = Some(*ancestor_a);
            } else {
                break;
            }
        }
        common
    }

    /// Find links matching a predicate
    pub fn find_links<F>(&self, predicate: F) -> Vec<&Link>
    where
        F: Fn(&Link) -> bool,
    {
        self.links.values().filter(|link| predicate(link)).collect()
    }

    /// Find joints matching a predicate
    pub fn find_joints<F>(&self, predicate: F) -> Vec<&Joint>
    where
        F: Fn(&Joint) -> bool,
    {
        self.joints
            .values()
            .filter(|joint| predicate(joint))
            .collect()
    }

    /// Get all links at a specific depth level
    pub fn get_links_at_depth(&self, depth: usize) -> Vec<Uuid> {
        self.ensure_cache_valid();
        self.cache
            .borrow()
            .depths
            .iter()
            .filter_map(|(id, &d)| if d == depth { Some(*id) } else { None })
            .collect()
    }
}
