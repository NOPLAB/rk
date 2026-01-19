//! 2D Sketch System
//!
//! Provides a 2D sketching system with:
//! - Geometric entities (points, lines, arcs, circles)
//! - Constraints (coincident, parallel, perpendicular, dimensions)
//! - Constraint solver using Newton-Raphson iteration

mod constraint;
mod entity;
mod solver;

pub use constraint::*;
pub use entity::*;
pub use solver::*;

use glam::{Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

/// Sketch-related errors
#[derive(Debug, Clone, Error)]
pub enum SketchError {
    #[error("Entity not found: {0}")]
    EntityNotFound(Uuid),

    #[error("Constraint not found: {0}")]
    ConstraintNotFound(Uuid),

    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),

    #[error("Solver failed: {0}")]
    SolverFailed(String),

    #[error("Profile extraction failed: {0}")]
    ProfileExtractionFailed(String),
}

/// A plane on which sketches are drawn
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SketchPlane {
    /// Origin of the plane in 3D space
    pub origin: Vec3,
    /// Normal vector of the plane
    pub normal: Vec3,
    /// X axis of the plane (for 2D to 3D mapping)
    pub x_axis: Vec3,
    /// Y axis of the plane (for 2D to 3D mapping)
    pub y_axis: Vec3,
}

impl Default for SketchPlane {
    fn default() -> Self {
        Self::xy()
    }
}

impl SketchPlane {
    /// XY plane at origin (Z = 0)
    pub fn xy() -> Self {
        Self {
            origin: Vec3::ZERO,
            normal: Vec3::Z,
            x_axis: Vec3::X,
            y_axis: Vec3::Y,
        }
    }

    /// XZ plane at origin (Y = 0)
    pub fn xz() -> Self {
        Self {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            x_axis: Vec3::X,
            y_axis: Vec3::Z,
        }
    }

    /// YZ plane at origin (X = 0)
    pub fn yz() -> Self {
        Self {
            origin: Vec3::ZERO,
            normal: Vec3::X,
            x_axis: Vec3::Y,
            y_axis: Vec3::Z,
        }
    }

    /// Create a custom plane
    pub fn new(origin: Vec3, normal: Vec3, x_axis: Vec3) -> Self {
        let normal = normal.normalize();
        let x_axis = x_axis.normalize();
        let y_axis = normal.cross(x_axis).normalize();
        Self {
            origin,
            normal,
            x_axis,
            y_axis,
        }
    }

    /// Get the Y axis of the plane (for backwards compatibility)
    pub fn y_axis(&self) -> Vec3 {
        self.y_axis
    }

    /// Convert a 2D point on the sketch to 3D world coordinates
    pub fn to_world(&self, point: Vec2) -> Vec3 {
        self.origin + self.x_axis * point.x + self.y_axis() * point.y
    }

    /// Convert a 3D world point to 2D sketch coordinates
    pub fn to_local(&self, point: Vec3) -> Vec2 {
        let local = point - self.origin;
        Vec2::new(local.dot(self.x_axis), local.dot(self.y_axis()))
    }

    /// Get the transform matrix from sketch space to world space
    pub fn transform(&self) -> Mat4 {
        let y_axis = self.y_axis();
        Mat4::from_cols(
            self.x_axis.extend(0.0),
            y_axis.extend(0.0),
            self.normal.extend(0.0),
            self.origin.extend(1.0),
        )
    }

    /// Get the rotation quaternion for this plane
    pub fn rotation(&self) -> Quat {
        let y_axis = self.y_axis();
        Quat::from_mat3(&glam::Mat3::from_cols(self.x_axis, y_axis, self.normal))
    }
}

/// A 2D sketch containing entities and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sketch {
    /// Unique identifier
    pub id: Uuid,
    /// Name of the sketch
    pub name: String,
    /// Plane on which the sketch is drawn
    pub plane: SketchPlane,
    /// Geometric entities (points, lines, arcs, etc.)
    entities: HashMap<Uuid, SketchEntity>,
    /// Constraints between entities
    constraints: HashMap<Uuid, SketchConstraint>,
    /// Construction geometry (not used for profiles)
    construction: HashSet<Uuid>,
    /// Whether the sketch is fully constrained
    #[serde(default)]
    is_solved: bool,
    /// Degrees of freedom remaining
    #[serde(default)]
    dof: u32,
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new("Sketch", SketchPlane::xy())
    }
}

impl Sketch {
    /// Create a new sketch on the given plane
    pub fn new(name: impl Into<String>, plane: SketchPlane) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            plane,
            entities: HashMap::new(),
            constraints: HashMap::new(),
            construction: HashSet::new(),
            is_solved: true,
            dof: 0,
        }
    }

    /// Create a sketch with a specific ID
    pub fn with_id(id: Uuid, name: impl Into<String>, plane: SketchPlane) -> Self {
        Self {
            id,
            name: name.into(),
            plane,
            entities: HashMap::new(),
            constraints: HashMap::new(),
            construction: HashSet::new(),
            is_solved: true,
            dof: 0,
        }
    }

    // ============== Entity Management ==============

    /// Add an entity to the sketch
    pub fn add_entity(&mut self, entity: SketchEntity) -> Uuid {
        let id = entity.id();
        self.entities.insert(id, entity);
        self.is_solved = false;
        id
    }

    /// Get an entity by ID
    pub fn get_entity(&self, id: Uuid) -> Option<&SketchEntity> {
        self.entities.get(&id)
    }

    /// Get a mutable entity by ID
    pub fn get_entity_mut(&mut self, id: Uuid) -> Option<&mut SketchEntity> {
        self.is_solved = false;
        self.entities.get_mut(&id)
    }

    /// Remove an entity
    pub fn remove_entity(&mut self, id: Uuid) -> Option<SketchEntity> {
        // Also remove related constraints
        let related_constraints: Vec<Uuid> = self
            .constraints
            .iter()
            .filter(|(_, c)| c.references_entity(id))
            .map(|(cid, _)| *cid)
            .collect();

        for cid in related_constraints {
            self.constraints.remove(&cid);
        }

        self.construction.remove(&id);
        self.is_solved = false;
        self.entities.remove(&id)
    }

    /// Get all entities
    pub fn entities(&self) -> &HashMap<Uuid, SketchEntity> {
        &self.entities
    }

    /// Iterate over entities
    pub fn entities_iter(&self) -> impl Iterator<Item = &SketchEntity> {
        self.entities.values()
    }

    // ============== Constraint Management ==============

    /// Add a constraint to the sketch
    pub fn add_constraint(&mut self, constraint: SketchConstraint) -> Result<Uuid, SketchError> {
        // Validate that referenced entities exist
        for entity_id in constraint.referenced_entities() {
            if !self.entities.contains_key(&entity_id) {
                return Err(SketchError::EntityNotFound(entity_id));
            }
        }

        let id = constraint.id();
        self.constraints.insert(id, constraint);
        self.is_solved = false;
        Ok(id)
    }

    /// Get a constraint by ID
    pub fn get_constraint(&self, id: Uuid) -> Option<&SketchConstraint> {
        self.constraints.get(&id)
    }

    /// Remove a constraint
    pub fn remove_constraint(&mut self, id: Uuid) -> Option<SketchConstraint> {
        self.is_solved = false;
        self.constraints.remove(&id)
    }

    /// Get all constraints
    pub fn constraints(&self) -> &HashMap<Uuid, SketchConstraint> {
        &self.constraints
    }

    /// Iterate over constraints
    pub fn constraints_iter(&self) -> impl Iterator<Item = &SketchConstraint> {
        self.constraints.values()
    }

    // ============== Construction Geometry ==============

    /// Mark an entity as construction geometry
    pub fn set_construction(&mut self, id: Uuid, is_construction: bool) {
        if is_construction {
            self.construction.insert(id);
        } else {
            self.construction.remove(&id);
        }
    }

    /// Check if an entity is construction geometry
    pub fn is_construction(&self, id: Uuid) -> bool {
        self.construction.contains(&id)
    }

    // ============== Solving ==============

    /// Check if the sketch is solved
    pub fn is_solved(&self) -> bool {
        self.is_solved
    }

    /// Get degrees of freedom
    pub fn degrees_of_freedom(&self) -> u32 {
        self.dof
    }

    /// Solve the sketch constraints
    pub fn solve(&mut self) -> SolveResult {
        let mut solver = ConstraintSolver::new();
        let result = solver.solve(self);

        match &result {
            SolveResult::FullyConstrained => {
                self.is_solved = true;
                self.dof = 0;
            }
            SolveResult::UnderConstrained { dof } => {
                self.is_solved = true;
                self.dof = *dof;
            }
            SolveResult::OverConstrained { .. } => {
                self.is_solved = false;
            }
            SolveResult::Failed { .. } => {
                self.is_solved = false;
            }
        }

        result
    }

    // ============== Profile Extraction ==============

    /// Extract closed profiles from the sketch for extrusion
    ///
    /// Returns a list of closed wire profiles (excluding construction geometry)
    pub fn extract_profiles(&self) -> Result<Vec<crate::kernel::Wire2D>, SketchError> {
        // For now, implement a simple profile extraction
        // that works with lines forming closed loops

        let mut profiles = Vec::new();
        let mut used_entities: HashSet<Uuid> = HashSet::new();

        // Find all line entities that are not construction
        let lines: Vec<&SketchEntity> = self
            .entities
            .values()
            .filter(|e| matches!(e, SketchEntity::Line { .. }) && !self.is_construction(e.id()))
            .collect();

        // Try to form closed loops
        for start_line in &lines {
            if used_entities.contains(&start_line.id()) {
                continue;
            }

            if let Some(profile) = self.trace_closed_loop(start_line.id(), &used_entities) {
                for id in &profile {
                    used_entities.insert(*id);
                }

                // Convert to Wire2D
                let points = self.entities_to_points(&profile)?;
                if points.len() >= 3 {
                    profiles.push(crate::kernel::Wire2D::new(points, true));
                }
            }
        }

        // Also check for circles (single entity profiles)
        for entity in self.entities.values() {
            if self.is_construction(entity.id()) {
                continue;
            }

            if let SketchEntity::Circle { center, radius, .. } = entity {
                let center_pos = self.get_point_position(*center)?;
                profiles.push(crate::kernel::Wire2D::circle(center_pos, *radius, 32));
            }
        }

        if profiles.is_empty() {
            return Err(SketchError::ProfileExtractionFailed(
                "No closed profiles found".into(),
            ));
        }

        Ok(profiles)
    }

    /// Trace a closed loop starting from a line
    fn trace_closed_loop(&self, start_id: Uuid, used: &HashSet<Uuid>) -> Option<Vec<Uuid>> {
        let start = self.entities.get(&start_id)?;
        let SketchEntity::Line {
            start: start_point,
            end: first_end,
            ..
        } = start
        else {
            return None;
        };

        let mut loop_entities = vec![start_id];
        let mut current_end = *first_end;
        let target = *start_point;

        // Follow connected lines
        for _ in 0..100 {
            // Limit iterations
            if current_end == target {
                return Some(loop_entities);
            }

            // Find next connected line
            let next = self.entities.values().find(|e| {
                if used.contains(&e.id()) || loop_entities.contains(&e.id()) {
                    return false;
                }
                if let SketchEntity::Line { start, end, .. } = e {
                    *start == current_end || *end == current_end
                } else {
                    false
                }
            });

            match next {
                Some(SketchEntity::Line { id, start, end, .. }) => {
                    loop_entities.push(*id);
                    current_end = if *start == current_end { *end } else { *start };
                }
                _ => return None,
            }
        }

        None
    }

    /// Convert entity IDs to a list of 2D points
    fn entities_to_points(&self, entity_ids: &[Uuid]) -> Result<Vec<Vec2>, SketchError> {
        let mut points = Vec::new();

        for id in entity_ids {
            let entity = self
                .entities
                .get(id)
                .ok_or(SketchError::EntityNotFound(*id))?;

            if let SketchEntity::Line { start, .. } = entity {
                let pos = self.get_point_position(*start)?;
                points.push(pos);
            }
        }

        Ok(points)
    }

    /// Get the position of a point entity
    fn get_point_position(&self, id: Uuid) -> Result<Vec2, SketchError> {
        let entity = self
            .entities
            .get(&id)
            .ok_or(SketchError::EntityNotFound(id))?;

        match entity {
            SketchEntity::Point { position, .. } => Ok(*position),
            _ => Err(SketchError::InvalidConstraint(format!(
                "Entity {} is not a point",
                id
            ))),
        }
    }

    // ============== Helper Methods ==============

    /// Add a point at the given position
    pub fn add_point(&mut self, position: Vec2) -> Uuid {
        self.add_entity(SketchEntity::Point {
            id: Uuid::new_v4(),
            position,
        })
    }

    /// Add a line between two points
    pub fn add_line(&mut self, start: Uuid, end: Uuid) -> Uuid {
        self.add_entity(SketchEntity::Line {
            id: Uuid::new_v4(),
            start,
            end,
        })
    }

    /// Add a circle with the given center and radius
    pub fn add_circle(&mut self, center: Uuid, radius: f32) -> Uuid {
        self.add_entity(SketchEntity::Circle {
            id: Uuid::new_v4(),
            center,
            radius,
        })
    }

    /// Add an arc
    pub fn add_arc(&mut self, center: Uuid, start: Uuid, end: Uuid, radius: f32) -> Uuid {
        self.add_entity(SketchEntity::Arc {
            id: Uuid::new_v4(),
            center,
            start,
            end,
            radius,
        })
    }

    /// Create a rectangle and return the corner point IDs and line IDs
    pub fn add_rectangle(&mut self, corner1: Vec2, corner2: Vec2) -> (Vec<Uuid>, Vec<Uuid>) {
        let corners = [
            corner1,
            Vec2::new(corner2.x, corner1.y),
            corner2,
            Vec2::new(corner1.x, corner2.y),
        ];

        let point_ids: Vec<Uuid> = corners.iter().map(|&p| self.add_point(p)).collect();

        let line_ids = vec![
            self.add_line(point_ids[0], point_ids[1]),
            self.add_line(point_ids[1], point_ids[2]),
            self.add_line(point_ids[2], point_ids[3]),
            self.add_line(point_ids[3], point_ids[0]),
        ];

        (point_ids, line_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sketch_plane_transform() {
        let plane = SketchPlane::xy();
        let point_2d = Vec2::new(1.0, 2.0);
        let point_3d = plane.to_world(point_2d);

        assert_eq!(point_3d, Vec3::new(1.0, 2.0, 0.0));

        let back = plane.to_local(point_3d);
        assert!((back - point_2d).length() < 0.001);
    }

    #[test]
    fn test_add_rectangle() {
        let mut sketch = Sketch::default();
        let (points, lines) = sketch.add_rectangle(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0));

        assert_eq!(points.len(), 4);
        assert_eq!(lines.len(), 4);
        assert_eq!(sketch.entities().len(), 8); // 4 points + 4 lines
    }
}
