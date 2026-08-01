//! The engine: owns the document, applies commands, emits events.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Mat4;
use uuid::Uuid;

use rk_cad::{CadKernel, HistoryEntry, Sketch, TessellatedMesh};
use rk_core::{Assembly, Part, Project};

use crate::command::Command;
use crate::document::Document;
use crate::error::EngineError;
use crate::event::{Event, ResetReason};
use crate::history::{CommandJournal, JournalEntry, Snapshot, UndoStack};

/// Tessellation tolerance for display meshes
pub(crate) const DISPLAY_MESH_TOLERANCE: f32 = 0.1;

/// Identifies one continuous interaction (a gizmo drag, a DragValue edit).
/// Commands applied under the same ID coalesce into a single undo step.
pub type InteractionId = Uuid;

/// The engine shared between the UI and other clients
pub type SharedEngine = Arc<parking_lot::Mutex<Engine>>;

enum UndoMode {
    /// Use the command's own snapshot policy
    Auto,
    /// Force a snapshot (first command of an interaction)
    Snapshot,
    /// Skip the snapshot (subsequent commands of an interaction)
    NoSnapshot,
}

/// Headless CAD engine: the single owner and mutator of the document
pub struct Engine {
    pub(crate) doc: Document,
    pub(crate) kernel: Arc<dyn CadKernel>,
    pub(crate) undo: UndoStack,
    pub(crate) journal: CommandJournal,
    pub(crate) doc_path: Option<PathBuf>,
    pub(crate) modified: bool,
    revision: u64,
    active_interaction: Option<InteractionId>,
}

impl Engine {
    pub fn new(kernel: Arc<dyn CadKernel>) -> Self {
        Self {
            doc: Document::default(),
            kernel,
            undo: UndoStack::default(),
            journal: CommandJournal::default(),
            doc_path: None,
            modified: false,
            revision: 0,
            active_interaction: None,
        }
    }

    // ================= Write API =================

    /// Apply a command, returning the events it produced.
    ///
    /// Atomic: on error the document is unchanged. Ends any active
    /// interaction session.
    pub fn apply(&mut self, cmd: Command) -> Result<Vec<Event>, EngineError> {
        self.active_interaction = None;
        self.apply_with(cmd, UndoMode::Auto)
    }

    /// Apply a command as part of an interaction session (gizmo drag,
    /// continuous value edit). The first command of a session takes the
    /// undo snapshot; the rest skip it, so the whole drag is one undo step.
    pub fn apply_interactive(
        &mut self,
        session: InteractionId,
        cmd: Command,
    ) -> Result<Vec<Event>, EngineError> {
        let mode = if self.active_interaction == Some(session) {
            UndoMode::NoSnapshot
        } else {
            UndoMode::Snapshot
        };
        let result = self.apply_with(cmd, mode);
        if result.is_ok() {
            self.active_interaction = Some(session);
        }
        result
    }

    /// End an interaction session. With `cancel: true` the document is
    /// rolled back to the state before the session started.
    pub fn end_interaction(
        &mut self,
        session: InteractionId,
        cancel: bool,
    ) -> Result<Vec<Event>, EngineError> {
        let mut events = Vec::new();
        if self.active_interaction == Some(session) {
            self.active_interaction = None;
            if cancel && let Some(snap) = self.undo.pop_undo() {
                self.restore_snapshot(snap.doc, ResetReason::UndoRedo, &mut events);
                self.modified = true;
                events.push(self.history_changed_event());
                self.revision += 1;
            }
        }
        Ok(events)
    }

    fn apply_with(&mut self, cmd: Command, mode: UndoMode) -> Result<Vec<Event>, EngineError> {
        // Undo/Redo restore snapshots instead of executing
        match cmd {
            Command::Undo => return self.undo_command(),
            Command::Redo => return self.redo_command(),
            _ => {}
        }

        let take_snapshot = match mode {
            UndoMode::Auto => cmd.takes_snapshot(),
            UndoMode::Snapshot => true,
            UndoMode::NoSnapshot => false,
        };
        let snapshot = take_snapshot.then(|| Snapshot {
            doc: self.doc.clone(),
            description: cmd.description().to_string(),
        });

        let mut events = Vec::new();
        match self.execute(&cmd, &mut events) {
            Ok(()) => {
                if let Some(snap) = snapshot {
                    self.undo.push(snap);
                    events.push(self.history_changed_event());
                }
                if let Some(modified) = cmd.marks_modified() {
                    self.set_modified(modified, &mut events);
                }
                self.revision += 1;
                self.journal.record(cmd);
                Ok(events)
            }
            Err(e) => {
                // Roll back so a failed command leaves no partial state
                if let Some(snap) = snapshot {
                    self.doc = snap.doc;
                }
                Err(e)
            }
        }
    }

    fn execute(&mut self, cmd: &Command, events: &mut Vec<Event>) -> Result<(), EngineError> {
        use Command::*;
        match cmd.clone() {
            NewDocument => self.exec_new_document(events),
            LoadDocument { path } => self.exec_load_document(path, events),
            SaveDocument { path } => self.exec_save_document(path, events),
            ImportMesh { path, unit } => self.exec_import_mesh(path, unit, events),
            ImportUrdf { path, stl_unit } => self.exec_import_urdf(path, stl_unit, events),
            ExportUrdf { path, robot_name } => self.exec_export_urdf(path, robot_name),
            RenameProject { name } => {
                self.doc.project.name = name;
                Ok(())
            }

            CreatePrimitive {
                id,
                primitive,
                name,
            } => self.exec_create_primitive(id, primitive, name, events),
            CreateEmptyPart { id, name } => self.exec_create_empty_part(id, name, events),
            DeletePart { part_id } => self.exec_delete_part(part_id, events),
            RenamePart { part_id, name } => self.exec_rename_part(part_id, name, events),
            SetPartTransform { part_id, transform } => {
                self.exec_set_part_transform(part_id, transform, events)
            }
            SetPartColor { part_id, color } => self.exec_set_part_color(part_id, color, events),
            SetPartMaterial {
                part_id,
                material_name,
            } => self.exec_set_part_material(part_id, material_name, events),
            SetPartMass { part_id, mass } => self.exec_set_part_mass(part_id, mass, events),
            SetPartInertia { part_id, inertia } => {
                self.exec_set_part_inertia(part_id, inertia, events)
            }

            ConnectParts {
                parent_part,
                child_part,
            } => self.exec_connect_parts(parent_part, child_part, events),
            DisconnectPart { child_part } => self.exec_disconnect_part(child_part, events),
            SetJointPosition { joint_id, position } => {
                self.exec_set_joint_position(joint_id, position, events)
            }
            ResetJointPosition { joint_id } => self.exec_reset_joint_position(joint_id, events),
            ResetAllJointPositions => self.exec_reset_all_joint_positions(events),
            SetJointType {
                joint_id,
                joint_type,
            } => self.exec_set_joint_type(joint_id, joint_type, events),
            SetJointOrigin {
                joint_id,
                origin,
                keep_child_world_pose,
            } => self.exec_set_joint_origin(joint_id, origin, keep_child_world_pose, events),
            SetJointAxis { joint_id, axis } => self.exec_set_joint_axis(joint_id, axis, events),
            SetJointLimits { joint_id, limits } => {
                self.exec_set_joint_limits(joint_id, limits, events)
            }

            AddCollision {
                link_id,
                geometry,
                origin,
            } => self.exec_add_collision(link_id, geometry, origin, events),
            RemoveCollision { link_id, index } => {
                self.exec_remove_collision(link_id, index, events)
            }
            SetCollisionOrigin {
                link_id,
                index,
                origin,
            } => self.exec_set_collision_origin(link_id, index, origin, events),
            SetCollisionGeometry {
                link_id,
                index,
                geometry,
            } => self.exec_set_collision_geometry(link_id, index, geometry, events),

            CreateSketch { id, name, plane } => self.exec_create_sketch(id, name, plane, events),
            DeleteSketch { sketch_id } => self.exec_delete_sketch(sketch_id, events),
            RenameSketch { sketch_id, name } => self.exec_rename_sketch(sketch_id, name, events),
            AddSketchEntities {
                sketch_id,
                entities,
            } => self.exec_add_sketch_entities(sketch_id, entities, events),
            UpdateSketchEntity { sketch_id, entity } => {
                self.exec_update_sketch_entity(sketch_id, entity, events)
            }
            DeleteSketchEntities {
                sketch_id,
                entity_ids,
            } => self.exec_delete_sketch_entities(sketch_id, entity_ids, events),
            AddSketchConstraint {
                sketch_id,
                constraint,
            } => self.exec_add_sketch_constraint(sketch_id, constraint, events),
            DeleteSketchConstraint {
                sketch_id,
                constraint_id,
            } => self.exec_delete_sketch_constraint(sketch_id, constraint_id, events),
            SolveSketch { sketch_id } => self.exec_solve_sketch(sketch_id, events),
            SetSketchConstruction {
                sketch_id,
                entity_ids,
                construction,
            } => self.exec_set_sketch_construction(sketch_id, entity_ids, construction, events),

            AddExtrude {
                id,
                name,
                sketch_id,
                profiles,
                distance,
                direction,
                boolean_op,
                target_body,
            } => self.exec_add_extrude(
                id,
                name,
                sketch_id,
                profiles,
                distance,
                direction,
                boolean_op,
                target_body,
                events,
            ),
            AddRevolve {
                id,
                name,
                sketch_id,
                profiles,
                axis_origin,
                axis_direction,
                angle,
                boolean_op,
                target_body,
            } => self.exec_add_revolve(
                id,
                name,
                sketch_id,
                profiles,
                axis_origin,
                axis_direction,
                angle,
                boolean_op,
                target_body,
                events,
            ),
            DeleteFeature { feature_id } => self.exec_delete_feature(feature_id, events),
            RenameFeature { feature_id, name } => {
                self.exec_rename_feature(feature_id, name, events)
            }
            SetFeatureSuppressed {
                feature_id,
                suppressed,
            } => self.exec_set_feature_suppressed(feature_id, suppressed, events),
            GroupFeatures {
                id,
                name,
                feature_ids,
            } => self.exec_group_features(id, name, feature_ids, events),
            UngroupFeatures { group_id } => self.exec_ungroup_features(group_id, events),
            RenameFeatureGroup { group_id, name } => {
                self.exec_rename_feature_group(group_id, name, events)
            }
            SetFeatureGroupCollapsed {
                group_id,
                collapsed,
            } => self.exec_set_feature_group_collapsed(group_id, collapsed, events),
            RollbackTo { feature_id } => self.exec_rollback_to(feature_id, events),
            RebuildFeatures => self.exec_rebuild_features(events),

            Undo | Redo => unreachable!("handled in apply_with"),
        }
    }

    // ================= Undo / Redo =================

    fn undo_command(&mut self) -> Result<Vec<Event>, EngineError> {
        let mut events = Vec::new();
        if let Some(snap) = self.undo.pop_undo() {
            let current = Snapshot {
                doc: self.doc.clone(),
                description: snap.description.clone(),
            };
            self.undo.push_redo(current);
            self.restore_snapshot(snap.doc, ResetReason::UndoRedo, &mut events);
            self.modified = true;
            events.push(self.history_changed_event());
            self.revision += 1;
            self.journal.record(Command::Undo);
        }
        Ok(events)
    }

    fn redo_command(&mut self) -> Result<Vec<Event>, EngineError> {
        let mut events = Vec::new();
        if let Some(snap) = self.undo.pop_redo() {
            let current = Snapshot {
                doc: self.doc.clone(),
                description: snap.description.clone(),
            };
            self.undo.push_undo_only(current);
            self.restore_snapshot(snap.doc, ResetReason::UndoRedo, &mut events);
            self.modified = true;
            events.push(self.history_changed_event());
            self.revision += 1;
            self.journal.record(Command::Redo);
        }
        Ok(events)
    }

    /// Replace the document and bring derived state (kinematics, CAD
    /// bodies) back in sync. Used by undo/redo, load, and cancel.
    pub(crate) fn restore_snapshot(
        &mut self,
        doc: Document,
        reason: ResetReason,
        events: &mut Vec<Event>,
    ) {
        self.doc = doc;
        self.doc
            .project
            .assembly
            .update_world_transforms_with_current_positions();
        if let Err(e) = self.doc.cad.history.rebuild(&*self.kernel) {
            tracing::warn!("feature rebuild after restore failed: {}", e);
        }
        events.push(Event::DocumentReset { reason });
    }

    // ================= Shared helpers for exec modules =================

    pub(crate) fn set_modified(&mut self, modified: bool, events: &mut Vec<Event>) {
        if self.modified != modified {
            self.modified = modified;
            events.push(Event::ModifiedChanged { modified });
        }
    }

    pub(crate) fn history_changed_event(&self) -> Event {
        Event::HistoryChanged {
            can_undo: self.undo.can_undo(),
            can_redo: self.undo.can_redo(),
            undo_description: self.undo.undo_description().map(str::to_string),
        }
    }

    /// Recompute forward kinematics and emit the resulting render transforms
    pub(crate) fn update_kinematics(&mut self, events: &mut Vec<Event>) {
        self.doc
            .project
            .assembly
            .update_world_transforms_with_current_positions();
        events.push(Event::WorldTransformsChanged {
            transforms: self.part_render_transforms(),
        });
    }

    // ================= Read API =================

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn project(&self) -> &Project {
        &self.doc.project
    }

    pub fn assembly(&self) -> &Assembly {
        &self.doc.project.assembly
    }

    pub fn part(&self, id: Uuid) -> Option<&Part> {
        self.doc.project.get_part(id)
    }

    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        self.doc.project.parts_iter()
    }

    pub fn sketch(&self, id: Uuid) -> Option<&Sketch> {
        self.doc.cad.history.get_sketch(id)
    }

    pub fn features(&self) -> &[HistoryEntry] {
        self.doc.cad.history.entries()
    }

    pub fn body_ids(&self) -> Vec<Uuid> {
        self.doc.cad.history.bodies().keys().copied().collect()
    }

    /// Get (and lazily tessellate) a body's display mesh
    pub fn body_mesh(&mut self, id: Uuid) -> Result<&TessellatedMesh, EngineError> {
        let kernel = self.kernel.clone();
        let body = self
            .doc
            .cad
            .history
            .get_body_mut(id)
            .ok_or(EngineError::NotFound { kind: "body", id })?;
        body.get_mesh(&*kernel, DISPLAY_MESH_TOLERANCE)
            .ok_or_else(|| EngineError::Cad(format!("tessellation failed for body {id}")))
    }

    /// Final render transform for every part: parts attached to a link get
    /// `link.world_transform * part.origin_transform`, free parts get their
    /// origin transform as-is.
    pub fn part_render_transforms(&self) -> Vec<(Uuid, Mat4)> {
        let project = &self.doc.project;
        let mut out = Vec::new();
        let mut linked: HashSet<Uuid> = HashSet::new();
        for link in project.assembly.links.values() {
            if let Some(part_id) = link.part_id
                && let Some(part) = project.get_part(part_id)
            {
                out.push((part_id, link.world_transform * part.origin_transform));
                linked.insert(part_id);
            }
        }
        for part in project.parts_iter() {
            if !linked.contains(&part.id) {
                out.push((part.id, part.origin_transform));
            }
        }
        out
    }

    /// The CAD kernel instance. Solids are handles into this kernel, so
    /// callers doing their own geometry (previews) must use this instance.
    pub fn kernel(&self) -> Arc<dyn CadKernel> {
        self.kernel.clone()
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn doc_path(&self) -> Option<&Path> {
        self.doc_path.as_deref()
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    pub fn undo_description(&self) -> Option<&str> {
        self.undo.undo_description()
    }

    /// Monotonic counter incremented on every successful apply
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Append-only log of every applied command
    pub fn command_log(&self) -> &[JournalEntry] {
        self.journal.entries()
    }
}
