//! Tree structure building for part hierarchy

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::state::AppState;

/// Actions collected during tree rendering
pub enum TreeAction {
    Select(Uuid),
    Delete(Uuid),
    Disconnect(Uuid),
    Connect { parent: Uuid, child: Uuid },
}

/// Build tree structure from Assembly state
///
/// Returns:
/// - root_link_part: Part belonging to the root link (if any)
/// - root_parts: Parts that are direct children of root_link
/// - children_map: Map of part_id -> child part_ids
/// - parts_with_parent: Set of parts that have a parent
/// - orphaned_parts: Parts not connected to root_link hierarchy
#[allow(clippy::type_complexity)]
pub fn build_tree_structure(
    state: &AppState,
) -> (
    Option<Uuid>,
    Vec<Uuid>,
    HashMap<Uuid, Vec<Uuid>>,
    HashSet<Uuid>,
    Vec<Uuid>,
) {
    let assembly = &state.project.assembly;

    // Map link_id -> part_id (only for links with parts)
    let link_to_part: HashMap<Uuid, Uuid> = assembly
        .links
        .iter()
        .filter_map(|(link_id, link)| link.part_id.map(|pid| (*link_id, pid)))
        .collect();

    // Map part_id -> link_id
    let part_to_link: HashMap<Uuid, Uuid> = assembly
        .links
        .iter()
        .filter_map(|(link_id, link)| link.part_id.map(|pid| (pid, *link_id)))
        .collect();

    // Build children map (part_id -> [child_part_ids])
    // For links without part_id, we track their children separately
    let mut children_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (parent_link_id, children) in &assembly.children {
        if let Some(&parent_part_id) = link_to_part.get(parent_link_id) {
            let child_parts: Vec<Uuid> = children
                .iter()
                .filter_map(|(_, child_link_id)| link_to_part.get(child_link_id).copied())
                .collect();
            if !child_parts.is_empty() {
                children_map.insert(parent_part_id, child_parts);
            }
        }
    }

    // Identify parts with parents
    let parts_with_parent: HashSet<Uuid> = assembly
        .parent
        .keys()
        .filter_map(|link_id| link_to_part.get(link_id).copied())
        .collect();

    // Find parts that are direct children of root_link
    let mut root_parts: Vec<Uuid> = Vec::new();
    if let Some(root_link_id) = assembly.root_link {
        // Get children of root_link
        if let Some(children) = assembly.children.get(&root_link_id) {
            for (_, child_link_id) in children {
                if let Some(&part_id) = link_to_part.get(child_link_id) {
                    root_parts.push(part_id);
                }
            }
        }
    }

    // Find orphaned parts (not connected to root_link hierarchy)
    // Note: root_link itself is not orphaned even though it has no parent
    let root_link_part_id: Option<Uuid> = assembly
        .root_link
        .and_then(|root_id| assembly.links.get(&root_id))
        .and_then(|link| link.part_id);

    let orphaned_parts: Vec<Uuid> = state
        .parts
        .keys()
        .filter(|part_id| {
            // Root link's part is not orphaned
            if Some(**part_id) == root_link_part_id {
                return false;
            }

            if let Some(&link_id) = part_to_link.get(part_id) {
                // Part has a link - check if it's disconnected (no parent)
                !assembly.parent.contains_key(&link_id)
            } else {
                // Part not in assembly at all
                true
            }
        })
        .copied()
        .collect();

    (
        root_link_part_id,
        root_parts,
        children_map,
        parts_with_parent,
        orphaned_parts,
    )
}

/// Check if connecting parent to child would be valid (no cycle)
pub fn can_connect(state: &AppState, parent_part: Uuid, child_part: Uuid) -> bool {
    if parent_part == child_part {
        return false;
    }

    let assembly = &state.project.assembly;

    // Find link IDs
    let parent_link = assembly
        .links
        .iter()
        .find(|(_, l)| l.part_id == Some(parent_part))
        .map(|(id, _)| *id);
    let child_link = assembly
        .links
        .iter()
        .find(|(_, l)| l.part_id == Some(child_part))
        .map(|(id, _)| *id);

    match (parent_link, child_link) {
        (Some(p), Some(c)) => {
            // Check if connecting would create a cycle
            // by checking if child is an ancestor of parent
            let mut current = Some(p);
            while let Some(id) = current {
                if id == c {
                    return false;
                }
                current = assembly.parent.get(&id).map(|(_, parent_id)| *parent_id);
            }
            true
        }
        _ => true, // If either isn't in assembly yet, no cycle possible
    }
}
