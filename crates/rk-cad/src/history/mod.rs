//! Parametric History
//!
//! Manages the ordered list of features that define a CAD model,
//! supporting rollback, rebuild, and editing of historical features.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::feature::{CadBody, Feature, FeatureError, FeatureResult};
use crate::kernel::{CadKernel, Solid};
use crate::sketch::Sketch;

/// An entry in the feature history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The feature
    pub feature: Feature,
    /// Bodies that existed before this feature
    pub prior_bodies: Vec<Uuid>,
    /// Bodies created by this feature
    pub created_bodies: Vec<Uuid>,
    /// Bodies modified by this feature
    pub modified_bodies: Vec<Uuid>,
    /// Bodies deleted by this feature
    pub deleted_bodies: Vec<Uuid>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(feature: Feature) -> Self {
        Self {
            feature,
            prior_bodies: Vec::new(),
            created_bodies: Vec::new(),
            modified_bodies: Vec::new(),
            deleted_bodies: Vec::new(),
        }
    }
}

/// A named bundle of timeline features.
///
/// Grouping is presentation only — it never changes the order features are
/// rebuilt in, so a group can be added, renamed or dissolved without the
/// model changing shape. Members are listed in history order by the client;
/// a group is drawn where its first member sits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureGroup {
    pub id: Uuid,
    pub name: String,
    /// Feature IDs belonging to this group
    pub members: Vec<Uuid>,
    /// Whether the browser shows the group folded up
    #[serde(default)]
    pub collapsed: bool,
}

/// Manages the parametric feature history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureHistory {
    /// Ordered list of features
    entries: Vec<HistoryEntry>,
    /// Current rollback position (None = at end)
    rollback_position: Option<usize>,
    /// All sketches in the model
    sketches: HashMap<Uuid, Sketch>,
    /// Named bundles of features, for the browser tree only
    #[serde(default)]
    groups: Vec<FeatureGroup>,
    /// All bodies in the model
    #[serde(skip)]
    bodies: HashMap<Uuid, CadBody>,
}

impl FeatureHistory {
    /// Create a new empty history
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of features
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a feature by index
    pub fn get(&self, index: usize) -> Option<&Feature> {
        self.entries.get(index).map(|e| &e.feature)
    }

    /// Get a mutable feature by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Feature> {
        self.entries.get_mut(index).map(|e| &mut e.feature)
    }

    /// Get a feature by ID
    pub fn get_by_id(&self, id: Uuid) -> Option<&Feature> {
        self.entries
            .iter()
            .find(|e| e.feature.id() == id)
            .map(|e| &e.feature)
    }

    /// Get a mutable feature by ID
    pub fn get_by_id_mut(&mut self, id: Uuid) -> Option<&mut Feature> {
        self.entries
            .iter_mut()
            .find(|e| e.feature.id() == id)
            .map(|e| &mut e.feature)
    }

    /// Get the index of a feature by ID
    pub fn index_of(&self, id: Uuid) -> Option<usize> {
        self.entries.iter().position(|e| e.feature.id() == id)
    }

    /// Add a feature to the history
    pub fn add_feature(&mut self, feature: Feature) {
        // If we're rolled back, remove features after the rollback point
        if let Some(pos) = self.rollback_position {
            self.entries.truncate(pos);
            self.rollback_position = None;
        }

        self.entries.push(HistoryEntry::new(feature));
    }

    /// Remove a feature from the history
    pub fn remove_feature(&mut self, id: Uuid) -> Option<Feature> {
        let index = self.index_of(id)?;
        let entry = self.entries.remove(index);
        // A group that lost its last member would otherwise linger in the
        // browser as an empty folder nothing can be dropped into
        for group in &mut self.groups {
            group.members.retain(|m| *m != id);
        }
        self.groups.retain(|g| !g.members.is_empty());
        Some(entry.feature)
    }

    /// Move a feature to a new position
    pub fn move_feature(&mut self, id: Uuid, new_index: usize) -> Result<(), FeatureError> {
        let old_index = self.index_of(id).ok_or(FeatureError::FeatureNotFound(id))?;

        if new_index >= self.entries.len() {
            return Err(FeatureError::InvalidFeature("Invalid new index".into()));
        }

        let entry = self.entries.remove(old_index);
        self.entries.insert(new_index, entry);

        Ok(())
    }

    /// Get all features
    pub fn features(&self) -> impl Iterator<Item = &Feature> {
        self.entries.iter().map(|e| &e.feature)
    }

    /// Get all history entries
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    // ============== Grouping ==============

    /// All feature groups
    pub fn groups(&self) -> &[FeatureGroup] {
        &self.groups
    }

    /// Add a group. Members already in another group are moved into this one,
    /// so a feature is never listed twice in the browser.
    pub fn add_group(&mut self, group: FeatureGroup) {
        for existing in &mut self.groups {
            existing.members.retain(|m| !group.members.contains(m));
        }
        self.groups.retain(|g| !g.members.is_empty());
        self.groups.push(group);
    }

    /// Dissolve a group; its features stay in the history untouched
    pub fn remove_group(&mut self, id: Uuid) -> Option<FeatureGroup> {
        let index = self.groups.iter().position(|g| g.id == id)?;
        Some(self.groups.remove(index))
    }

    /// Get a group by ID
    pub fn get_group(&self, id: Uuid) -> Option<&FeatureGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Get a mutable group by ID
    pub fn get_group_mut(&mut self, id: Uuid) -> Option<&mut FeatureGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// The group a feature belongs to, if any
    pub fn group_of(&self, feature_id: Uuid) -> Option<&FeatureGroup> {
        self.groups.iter().find(|g| g.members.contains(&feature_id))
    }

    // ============== Sketch Management ==============

    /// Add a sketch
    pub fn add_sketch(&mut self, sketch: Sketch) -> Uuid {
        let id = sketch.id;
        self.sketches.insert(id, sketch);
        id
    }

    /// Get a sketch by ID
    pub fn get_sketch(&self, id: Uuid) -> Option<&Sketch> {
        self.sketches.get(&id)
    }

    /// Get a mutable sketch by ID
    pub fn get_sketch_mut(&mut self, id: Uuid) -> Option<&mut Sketch> {
        self.sketches.get_mut(&id)
    }

    /// Remove a sketch
    pub fn remove_sketch(&mut self, id: Uuid) -> Option<Sketch> {
        self.sketches.remove(&id)
    }

    /// Get all sketches
    pub fn sketches(&self) -> &HashMap<Uuid, Sketch> {
        &self.sketches
    }

    // ============== Body Management ==============

    /// Get a body by ID
    pub fn get_body(&self, id: Uuid) -> Option<&CadBody> {
        self.bodies.get(&id)
    }

    /// Get a mutable body by ID
    pub fn get_body_mut(&mut self, id: Uuid) -> Option<&mut CadBody> {
        self.bodies.get_mut(&id)
    }

    /// Get all bodies
    pub fn bodies(&self) -> &HashMap<Uuid, CadBody> {
        &self.bodies
    }

    /// Get mutable access to all bodies
    pub fn bodies_mut(&mut self) -> &mut HashMap<Uuid, CadBody> {
        &mut self.bodies
    }

    // ============== Rollback ==============

    /// Roll back to a specific feature (features after it are hidden)
    pub fn rollback_to(&mut self, id: Uuid) -> Result<(), FeatureError> {
        let index = self.index_of(id).ok_or(FeatureError::FeatureNotFound(id))?;

        self.rollback_position = Some(index + 1);
        Ok(())
    }

    /// Roll back to the end (show all features)
    pub fn rollback_to_end(&mut self) {
        self.rollback_position = None;
    }

    /// Get the current rollback position
    pub fn rollback_position(&self) -> Option<usize> {
        self.rollback_position
    }

    /// Get the effective number of features (accounting for rollback)
    pub fn effective_len(&self) -> usize {
        self.rollback_position.unwrap_or(self.entries.len())
    }

    /// Iterate over effective features (accounting for rollback)
    pub fn effective_features(&self) -> impl Iterator<Item = &Feature> {
        let end = self.effective_len();
        self.entries[..end].iter().map(|e| &e.feature)
    }

    // ============== Rebuild ==============

    /// Rebuild all geometry from features
    pub fn rebuild(&mut self, kernel: &dyn CadKernel) -> FeatureResult<()> {
        // Clear existing bodies
        self.bodies.clear();

        // Convert bodies to solids for feature execution
        let mut solids: HashMap<Uuid, Solid> = HashMap::new();

        // Execute each feature in order
        let end = self.effective_len();
        for entry in &mut self.entries[..end] {
            if entry.feature.is_suppressed() {
                continue;
            }

            match entry.feature.execute(kernel, &self.sketches, &solids) {
                Ok(solid) => {
                    // Reuse the previous body ID so references to this body
                    // (e.g. Boolean target_body) stay valid across rebuilds
                    let body_id = entry
                        .created_bodies
                        .first()
                        .copied()
                        .unwrap_or_else(Uuid::new_v4);
                    let mut body = CadBody::with_id(body_id, entry.feature.name());
                    body.source_feature = Some(entry.feature.id());

                    // Store the solid
                    solids.insert(body_id, solid.clone());
                    body.solid = Some(solid);

                    self.bodies.insert(body_id, body);
                    entry.created_bodies = vec![body_id];
                }
                Err(e) => {
                    // Log error but continue with other features
                    tracing::warn!("Feature {} failed: {}", entry.feature.name(), e);
                }
            }
        }

        Ok(())
    }

    /// Rebuild a single feature and all dependent features
    ///
    /// This is optimized to only rebuild features from the specified feature onwards,
    /// rather than rebuilding the entire history.
    pub fn rebuild_from(&mut self, id: Uuid, kernel: &dyn CadKernel) -> FeatureResult<()> {
        let start_index = self.index_of(id).ok_or(FeatureError::FeatureNotFound(id))?;
        let end = self.effective_len();

        // If start_index is 0, just do a full rebuild
        if start_index == 0 {
            return self.rebuild(kernel);
        }

        // Remove bodies created by features from start_index onwards
        for entry in &self.entries[start_index..end] {
            for body_id in &entry.created_bodies {
                self.bodies.remove(body_id);
            }
        }

        // Build solids map from existing bodies (before start_index)
        let mut solids: HashMap<Uuid, Solid> = self
            .bodies
            .iter()
            .filter_map(|(id, body)| body.solid.clone().map(|s| (*id, s)))
            .collect();

        // Re-execute features from start_index onwards
        for entry in &mut self.entries[start_index..end] {
            if entry.feature.is_suppressed() {
                continue;
            }

            // Clear previous results for this entry (created_bodies is kept so
            // the body ID can be reused on success)
            entry.modified_bodies.clear();
            entry.deleted_bodies.clear();

            match entry.feature.execute(kernel, &self.sketches, &solids) {
                Ok(solid) => {
                    let body_id = entry
                        .created_bodies
                        .first()
                        .copied()
                        .unwrap_or_else(Uuid::new_v4);
                    let mut body = CadBody::with_id(body_id, entry.feature.name());
                    body.source_feature = Some(entry.feature.id());

                    solids.insert(body_id, solid.clone());
                    body.solid = Some(solid);

                    self.bodies.insert(body_id, body);
                    entry.created_bodies = vec![body_id];
                }
                Err(e) => {
                    tracing::warn!("Feature {} failed: {}", entry.feature.name(), e);
                }
            }
        }

        Ok(())
    }
}

/// CAD data that can be stored in a project
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CadData {
    /// Feature history
    pub history: FeatureHistory,
}

impl CadData {
    /// Create new empty CAD data
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there's any CAD data
    pub fn is_empty(&self) -> bool {
        self.history.is_empty() && self.history.sketches().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::ExtrudeDirection;
    use crate::sketch::SketchPlane;
    use glam::Vec2;

    #[test]
    fn test_add_feature() {
        let mut history = FeatureHistory::new();
        let feature = Feature::extrude("Test", Uuid::new_v4(), 10.0, ExtrudeDirection::Positive);
        let id = feature.id();

        history.add_feature(feature);

        assert_eq!(history.len(), 1);
        assert!(history.get_by_id(id).is_some());
    }

    #[test]
    fn test_rollback() {
        let mut history = FeatureHistory::new();

        let f1 = Feature::extrude("F1", Uuid::new_v4(), 10.0, ExtrudeDirection::Positive);
        let f2 = Feature::extrude("F2", Uuid::new_v4(), 20.0, ExtrudeDirection::Positive);
        let f3 = Feature::extrude("F3", Uuid::new_v4(), 30.0, ExtrudeDirection::Positive);

        let f1_id = f1.id();

        history.add_feature(f1);
        history.add_feature(f2);
        history.add_feature(f3);

        assert_eq!(history.len(), 3);
        assert_eq!(history.effective_len(), 3);

        // Rollback to first feature
        history.rollback_to(f1_id).unwrap();
        assert_eq!(history.effective_len(), 1);
        assert_eq!(history.effective_features().count(), 1);

        // Roll forward
        history.rollback_to_end();
        assert_eq!(history.effective_len(), 3);
    }

    #[test]
    fn a_group_never_lists_a_feature_twice() {
        let mut history = FeatureHistory::new();
        let ids: Vec<Uuid> = (0..3)
            .map(|i| {
                let f = Feature::extrude(
                    format!("F{i}"),
                    Uuid::new_v4(),
                    1.0,
                    ExtrudeDirection::Positive,
                );
                let id = f.id();
                history.add_feature(f);
                id
            })
            .collect();

        history.add_group(FeatureGroup {
            id: Uuid::new_v4(),
            name: "First two".into(),
            members: vec![ids[0], ids[1]],
            collapsed: false,
        });
        let second = Uuid::new_v4();
        history.add_group(FeatureGroup {
            id: second,
            name: "Last two".into(),
            members: vec![ids[1], ids[2]],
            collapsed: false,
        });

        // The middle feature moved to the newer group rather than appearing
        // in both
        assert_eq!(history.groups().len(), 2);
        assert_eq!(history.groups()[0].members, vec![ids[0]]);
        assert_eq!(history.group_of(ids[1]).map(|g| g.id), Some(second));

        // Deleting the features a group holds takes the group with them
        history.remove_feature(ids[1]);
        history.remove_feature(ids[2]);
        assert_eq!(history.groups().len(), 1);
        assert!(history.get_group(second).is_none());
    }

    #[test]
    fn test_rebuild_keeps_body_ids_stable() {
        let kernel = crate::kernel::default_kernel();
        if !kernel.is_available() {
            return; // NullKernel build; nothing to execute
        }

        let mut history = FeatureHistory::new();
        let mut sketch = Sketch::new("Profile", SketchPlane::xy());
        sketch.add_rectangle(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let sketch_id = history.add_sketch(sketch);

        let f1 = Feature::extrude("E1", sketch_id, 5.0, ExtrudeDirection::Positive);
        let f2 = Feature::extrude("E2", sketch_id, 2.0, ExtrudeDirection::Negative);
        let f2_id = f2.id();
        history.add_feature(f1);
        history.add_feature(f2);

        history.rebuild(&*kernel).unwrap();
        let mut first: Vec<Uuid> = history.bodies().keys().copied().collect();
        first.sort();
        assert_eq!(first.len(), 2, "each extrude should create a body");

        // Full rebuild must not change body IDs
        history.rebuild(&*kernel).unwrap();
        let mut second: Vec<Uuid> = history.bodies().keys().copied().collect();
        second.sort();
        assert_eq!(first, second);

        // Partial rebuild must not change body IDs either
        history.rebuild_from(f2_id, &*kernel).unwrap();
        let mut third: Vec<Uuid> = history.bodies().keys().copied().collect();
        third.sort();
        assert_eq!(first, third);
    }
}
