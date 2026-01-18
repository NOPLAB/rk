//! Constraint Solver
//!
//! Pure Rust implementation of a Newton-Raphson based constraint solver
//! for 2D sketch constraints.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{Sketch, SketchConstraint, SketchEntity};

/// Result of solving sketch constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolveResult {
    /// All constraints are satisfied, no degrees of freedom remain
    FullyConstrained,

    /// Constraints are satisfied, but some degrees of freedom remain
    UnderConstrained {
        /// Number of remaining degrees of freedom
        dof: u32,
    },

    /// Too many constraints, some are conflicting
    OverConstrained {
        /// Conflicting constraint IDs
        conflicts: Vec<Uuid>,
    },

    /// Solver failed to converge
    Failed {
        /// Reason for failure
        reason: String,
    },
}

/// Constraint solver using Newton-Raphson iteration
pub struct ConstraintSolver {
    /// Tolerance for convergence
    tolerance: f32,
    /// Maximum number of iterations
    max_iterations: usize,
    /// Damping factor for Newton steps
    damping: f32,
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintSolver {
    /// Create a new solver with default parameters
    pub fn new() -> Self {
        Self {
            tolerance: 1e-4, // Relaxed for f32 precision
            max_iterations: 200,
            damping: 0.8, // Slight damping for stability
        }
    }

    /// Set the convergence tolerance
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set the maximum iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set the damping factor (0-1)
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping.clamp(0.1, 1.0);
        self
    }

    /// Solve the constraints in the given sketch
    pub fn solve(&mut self, sketch: &mut Sketch) -> SolveResult {
        // Build variable vector (point positions)
        let mut var_map = VariableMap::new();
        var_map.build_from_sketch(sketch);

        if var_map.is_empty() {
            return SolveResult::FullyConstrained;
        }

        // Get initial variable values
        let mut x = var_map.get_values(sketch);
        let n_vars = x.len();

        // Count constraint equations
        let n_equations: usize = sketch.constraints_iter().map(|c| c.equation_count()).sum();

        // Check for over/under constrained
        let dof = n_vars as i32 - n_equations as i32;

        if n_equations == 0 {
            return SolveResult::UnderConstrained { dof: n_vars as u32 };
        }

        // Newton-Raphson iteration
        for iteration in 0..self.max_iterations {
            // Apply current values to sketch
            var_map.set_values(sketch, &x);

            // Evaluate constraint errors
            let f = self.evaluate_constraints(sketch, &var_map);

            // Check for convergence
            let error = f.iter().map(|e| e * e).sum::<f32>().sqrt();
            if error < self.tolerance {
                if dof > 0 {
                    return SolveResult::UnderConstrained { dof: dof as u32 };
                } else if dof == 0 {
                    return SolveResult::FullyConstrained;
                } else {
                    // Over-constrained but still converged
                    return SolveResult::FullyConstrained;
                }
            }

            // Compute Jacobian
            let j = self.compute_jacobian(sketch, &var_map, &x);

            // Solve J * dx = -f using least squares
            match self.solve_linear_system(&j, &f, n_vars, n_equations) {
                Some(dx) => {
                    // Apply damped update
                    for i in 0..n_vars {
                        x[i] += self.damping * dx[i];
                    }
                }
                None => {
                    // Singular Jacobian - check for conflicts
                    return SolveResult::Failed {
                        reason: format!(
                            "Singular Jacobian at iteration {} (possibly over-constrained)",
                            iteration
                        ),
                    };
                }
            }
        }

        // Failed to converge
        SolveResult::Failed {
            reason: format!(
                "Failed to converge after {} iterations",
                self.max_iterations
            ),
        }
    }

    /// Evaluate all constraint equations
    fn evaluate_constraints(&self, sketch: &Sketch, var_map: &VariableMap) -> Vec<f32> {
        let mut errors = Vec::new();

        for constraint in sketch.constraints_iter() {
            match constraint {
                SketchConstraint::Coincident { point1, point2, .. } => {
                    let p1 = var_map.get_point_position(sketch, *point1);
                    let p2 = var_map.get_point_position(sketch, *point2);
                    errors.push(p1.x - p2.x);
                    errors.push(p1.y - p2.y);
                }

                SketchConstraint::Horizontal { line, .. } => {
                    if let Some((start, end)) = self.get_line_endpoints(sketch, *line) {
                        let p1 = var_map.get_point_position(sketch, start);
                        let p2 = var_map.get_point_position(sketch, end);
                        errors.push(p1.y - p2.y);
                    }
                }

                SketchConstraint::Vertical { line, .. } => {
                    if let Some((start, end)) = self.get_line_endpoints(sketch, *line) {
                        let p1 = var_map.get_point_position(sketch, start);
                        let p2 = var_map.get_point_position(sketch, end);
                        errors.push(p1.x - p2.x);
                    }
                }

                SketchConstraint::Distance {
                    entity1,
                    entity2,
                    value,
                    ..
                } => {
                    let p1 = var_map.get_point_position(sketch, *entity1);
                    let p2 = var_map.get_point_position(sketch, *entity2);
                    let dist = (p1 - p2).length();
                    errors.push(dist - value);
                }

                SketchConstraint::HorizontalDistance {
                    point1,
                    point2,
                    value,
                    ..
                } => {
                    let p1 = var_map.get_point_position(sketch, *point1);
                    let p2 = var_map.get_point_position(sketch, *point2);
                    errors.push((p2.x - p1.x) - value);
                }

                SketchConstraint::VerticalDistance {
                    point1,
                    point2,
                    value,
                    ..
                } => {
                    let p1 = var_map.get_point_position(sketch, *point1);
                    let p2 = var_map.get_point_position(sketch, *point2);
                    errors.push((p2.y - p1.y) - value);
                }

                SketchConstraint::Fixed { point, x, y, .. } => {
                    let p = var_map.get_point_position(sketch, *point);
                    errors.push(p.x - x);
                    errors.push(p.y - y);
                }

                SketchConstraint::Length { line, value, .. } => {
                    if let Some((start, end)) = self.get_line_endpoints(sketch, *line) {
                        let p1 = var_map.get_point_position(sketch, start);
                        let p2 = var_map.get_point_position(sketch, end);
                        let len = (p2 - p1).length();
                        errors.push(len - value);
                    }
                }

                SketchConstraint::Parallel { line1, line2, .. } => {
                    if let (Some((s1, e1)), Some((s2, e2))) = (
                        self.get_line_endpoints(sketch, *line1),
                        self.get_line_endpoints(sketch, *line2),
                    ) {
                        let d1 = var_map.get_point_position(sketch, e1)
                            - var_map.get_point_position(sketch, s1);
                        let d2 = var_map.get_point_position(sketch, e2)
                            - var_map.get_point_position(sketch, s2);
                        // Cross product should be zero for parallel lines
                        errors.push(d1.x * d2.y - d1.y * d2.x);
                    }
                }

                SketchConstraint::Perpendicular { line1, line2, .. } => {
                    if let (Some((s1, e1)), Some((s2, e2))) = (
                        self.get_line_endpoints(sketch, *line1),
                        self.get_line_endpoints(sketch, *line2),
                    ) {
                        let d1 = var_map.get_point_position(sketch, e1)
                            - var_map.get_point_position(sketch, s1);
                        let d2 = var_map.get_point_position(sketch, e2)
                            - var_map.get_point_position(sketch, s2);
                        // Dot product should be zero for perpendicular lines
                        errors.push(d1.x * d2.x + d1.y * d2.y);
                    }
                }

                SketchConstraint::EqualLength { line1, line2, .. } => {
                    if let (Some((s1, e1)), Some((s2, e2))) = (
                        self.get_line_endpoints(sketch, *line1),
                        self.get_line_endpoints(sketch, *line2),
                    ) {
                        let len1 = (var_map.get_point_position(sketch, e1)
                            - var_map.get_point_position(sketch, s1))
                        .length();
                        let len2 = (var_map.get_point_position(sketch, e2)
                            - var_map.get_point_position(sketch, s2))
                        .length();
                        errors.push(len1 - len2);
                    }
                }

                SketchConstraint::Radius { circle, value, .. } => {
                    if let Some(SketchEntity::Circle { radius, .. }) = sketch.get_entity(*circle) {
                        errors.push(*radius - *value);
                    }
                }

                SketchConstraint::Angle {
                    line1,
                    line2,
                    value,
                    ..
                } => {
                    if let (Some((s1, e1)), Some((s2, e2))) = (
                        self.get_line_endpoints(sketch, *line1),
                        self.get_line_endpoints(sketch, *line2),
                    ) {
                        let d1 = var_map.get_point_position(sketch, e1)
                            - var_map.get_point_position(sketch, s1);
                        let d2 = var_map.get_point_position(sketch, e2)
                            - var_map.get_point_position(sketch, s2);
                        let angle = d1.y.atan2(d1.x) - d2.y.atan2(d2.x);
                        errors.push(angle - value);
                    }
                }

                SketchConstraint::Midpoint { point, line, .. } => {
                    if let Some((start, end)) = self.get_line_endpoints(sketch, *line) {
                        let p = var_map.get_point_position(sketch, *point);
                        let s = var_map.get_point_position(sketch, start);
                        let e = var_map.get_point_position(sketch, end);
                        let mid = (s + e) * 0.5;
                        errors.push(p.x - mid.x);
                        errors.push(p.y - mid.y);
                    }
                }

                SketchConstraint::Diameter { circle, value, .. } => {
                    if let Some(SketchEntity::Circle { radius, .. }) = sketch.get_entity(*circle) {
                        // Diameter = 2 * radius
                        errors.push(2.0 * radius - value);
                    }
                }

                SketchConstraint::EqualRadius {
                    circle1, circle2, ..
                } => {
                    let r1 = self.get_circle_radius(sketch, *circle1);
                    let r2 = self.get_circle_radius(sketch, *circle2);
                    if let (Some(r1), Some(r2)) = (r1, r2) {
                        errors.push(r1 - r2);
                    }
                }

                SketchConstraint::PointOnCurve { point, curve, .. } => {
                    let p = var_map.get_point_position(sketch, *point);

                    if let Some(entity) = sketch.get_entity(*curve) {
                        match entity {
                            SketchEntity::Line { start, end, .. } => {
                                // Point should be collinear with line
                                let s = var_map.get_point_position(sketch, *start);
                                let e = var_map.get_point_position(sketch, *end);
                                // Cross product of (p - s) and (e - s) should be zero
                                let ps = p - s;
                                let es = e - s;
                                errors.push(ps.x * es.y - ps.y * es.x);
                            }
                            SketchEntity::Circle { center, radius, .. } => {
                                // Distance from point to center should equal radius
                                let c = var_map.get_point_position(sketch, *center);
                                let dist = (p - c).length();
                                errors.push(dist - radius);
                            }
                            SketchEntity::Arc { center, radius, .. } => {
                                // Distance from point to center should equal radius
                                let c = var_map.get_point_position(sketch, *center);
                                let dist = (p - c).length();
                                errors.push(dist - radius);
                            }
                            _ => {}
                        }
                    }
                }

                SketchConstraint::Tangent { curve1, curve2, .. } => {
                    if let (Some(e1), Some(e2)) =
                        (sketch.get_entity(*curve1), sketch.get_entity(*curve2))
                    {
                        match (e1, e2) {
                            // Line-Circle tangency: distance from line to center = radius
                            (
                                SketchEntity::Line { start, end, .. },
                                SketchEntity::Circle { center, radius, .. },
                            )
                            | (
                                SketchEntity::Circle { center, radius, .. },
                                SketchEntity::Line { start, end, .. },
                            ) => {
                                let s = var_map.get_point_position(sketch, *start);
                                let e = var_map.get_point_position(sketch, *end);
                                let c = var_map.get_point_position(sketch, *center);

                                // Distance from point to line
                                let line_vec = e - s;
                                let line_len = line_vec.length();
                                if line_len > 1e-6 {
                                    let t = ((c - s).dot(line_vec)) / (line_len * line_len);
                                    let closest = s + line_vec * t.clamp(0.0, 1.0);
                                    let dist = (c - closest).length();
                                    errors.push(dist - radius);
                                }
                            }
                            // Circle-Circle tangency: |d| = r1 + r2 (external) or |r1 - r2| (internal)
                            (
                                SketchEntity::Circle {
                                    center: c1,
                                    radius: r1,
                                    ..
                                },
                                SketchEntity::Circle {
                                    center: c2,
                                    radius: r2,
                                    ..
                                },
                            ) => {
                                let center1 = var_map.get_point_position(sketch, *c1);
                                let center2 = var_map.get_point_position(sketch, *c2);
                                let dist = (center2 - center1).length();
                                // External tangency: dist = r1 + r2
                                errors.push(dist - (r1 + r2));
                            }
                            // Line-Arc tangency
                            (
                                SketchEntity::Line { start, end, .. },
                                SketchEntity::Arc { center, radius, .. },
                            )
                            | (
                                SketchEntity::Arc { center, radius, .. },
                                SketchEntity::Line { start, end, .. },
                            ) => {
                                let s = var_map.get_point_position(sketch, *start);
                                let e = var_map.get_point_position(sketch, *end);
                                let c = var_map.get_point_position(sketch, *center);

                                let line_vec = e - s;
                                let line_len = line_vec.length();
                                if line_len > 1e-6 {
                                    let t = ((c - s).dot(line_vec)) / (line_len * line_len);
                                    let closest = s + line_vec * t.clamp(0.0, 1.0);
                                    let dist = (c - closest).length();
                                    errors.push(dist - radius);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                SketchConstraint::Symmetric {
                    entity1,
                    entity2,
                    axis,
                    ..
                } => {
                    // For point symmetry about a line axis:
                    // 1. Midpoint of p1-p2 lies on the axis
                    // 2. Line p1-p2 is perpendicular to the axis
                    if let Some((axis_start, axis_end)) = self.get_line_endpoints(sketch, *axis) {
                        let as_pos = var_map.get_point_position(sketch, axis_start);
                        let ae_pos = var_map.get_point_position(sketch, axis_end);
                        let axis_dir = ae_pos - as_pos;
                        let axis_len = axis_dir.length();

                        if axis_len > 1e-6 {
                            let axis_unit = axis_dir / axis_len;

                            // Get positions of entities (treat as points for now)
                            let p1 = var_map.get_point_position(sketch, *entity1);
                            let p2 = var_map.get_point_position(sketch, *entity2);

                            // Midpoint should lie on the axis line
                            let mid = (p1 + p2) * 0.5;
                            let mid_to_axis = mid - as_pos;
                            // Cross product for distance to line
                            let cross = mid_to_axis.x * axis_unit.y - mid_to_axis.y * axis_unit.x;
                            errors.push(cross);

                            // Line p1-p2 should be perpendicular to axis (dot product = 0)
                            let p1_to_p2 = p2 - p1;
                            let dot = p1_to_p2.dot(axis_unit);
                            errors.push(dot);
                        }
                    }
                }
            }
        }

        errors
    }

    /// Compute the Jacobian matrix numerically
    fn compute_jacobian(&self, sketch: &Sketch, var_map: &VariableMap, x: &[f32]) -> Vec<Vec<f32>> {
        let n_vars = x.len();
        let f0 = self.evaluate_constraints(sketch, var_map);
        let n_equations = f0.len();

        let h = 1e-5; // Finite difference step size (larger for f32 stability)

        let mut jacobian = vec![vec![0.0; n_vars]; n_equations];

        // Create a mutable copy of the sketch for perturbation
        let mut perturbed_sketch = sketch.clone();

        for j in 0..n_vars {
            // Perturb variable j
            let mut x_plus = x.to_vec();
            x_plus[j] += h;

            var_map.set_values(&mut perturbed_sketch, &x_plus);
            let f_plus = self.evaluate_constraints(&perturbed_sketch, var_map);

            // Compute derivative using forward difference
            for i in 0..n_equations {
                jacobian[i][j] = (f_plus[i] - f0[i]) / h;
            }
        }

        jacobian
    }

    /// Solve the linear system J * dx = -f using least squares
    #[allow(clippy::needless_range_loop)]
    fn solve_linear_system(
        &self,
        j: &[Vec<f32>],
        f: &[f32],
        n_vars: usize,
        n_equations: usize,
    ) -> Option<Vec<f32>> {
        // Use pseudo-inverse: dx = (J^T * J)^-1 * J^T * (-f)
        // This handles overdetermined and underdetermined systems

        // Compute J^T * J
        let mut jtj = vec![vec![0.0f32; n_vars]; n_vars];
        for i in 0..n_vars {
            for k in 0..n_vars {
                for eq in 0..n_equations {
                    jtj[i][k] += j[eq][i] * j[eq][k];
                }
            }
        }

        // Add regularization to handle singular/near-singular matrices
        let reg = 1e-6;
        for i in 0..n_vars {
            jtj[i][i] += reg;
        }

        // Compute J^T * (-f)
        let mut jtf = vec![0.0f32; n_vars];
        for i in 0..n_vars {
            for eq in 0..n_equations {
                jtf[i] -= j[eq][i] * f[eq];
            }
        }

        // Solve using Gaussian elimination with partial pivoting
        self.gaussian_elimination(&mut jtj, &mut jtf)
    }

    /// Gaussian elimination with partial pivoting
    #[allow(clippy::needless_range_loop)]
    fn gaussian_elimination(&self, a: &mut [Vec<f32>], b: &mut [f32]) -> Option<Vec<f32>> {
        let n = b.len();
        if n == 0 {
            return Some(vec![]);
        }

        // Forward elimination
        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            for k in (i + 1)..n {
                if a[k][i].abs() > max_val {
                    max_val = a[k][i].abs();
                    max_row = k;
                }
            }

            // Check for singular matrix
            if max_val < 1e-12 {
                return None;
            }

            // Swap rows
            if max_row != i {
                a.swap(i, max_row);
                b.swap(i, max_row);
            }

            // Eliminate column
            for k in (i + 1)..n {
                let factor = a[k][i] / a[i][i];
                for j in i..n {
                    a[k][j] -= factor * a[i][j];
                }
                b[k] -= factor * b[i];
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = b[i];
            for j in (i + 1)..n {
                x[i] -= a[i][j] * x[j];
            }
            x[i] /= a[i][i];
        }

        Some(x)
    }

    /// Get the start and end point IDs of a line entity
    fn get_line_endpoints(&self, sketch: &Sketch, line_id: Uuid) -> Option<(Uuid, Uuid)> {
        match sketch.get_entity(line_id) {
            Some(SketchEntity::Line { start, end, .. }) => Some((*start, *end)),
            _ => None,
        }
    }

    /// Get the radius of a circle or arc entity
    fn get_circle_radius(&self, sketch: &Sketch, entity_id: Uuid) -> Option<f32> {
        match sketch.get_entity(entity_id) {
            Some(SketchEntity::Circle { radius, .. }) => Some(*radius),
            Some(SketchEntity::Arc { radius, .. }) => Some(*radius),
            _ => None,
        }
    }
}

/// Maps point IDs to variable indices
struct VariableMap {
    /// Map from point ID to variable index (x = index, y = index + 1)
    point_indices: HashMap<Uuid, usize>,
    /// Total number of variables
    count: usize,
}

impl VariableMap {
    fn new() -> Self {
        Self {
            point_indices: HashMap::new(),
            count: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Build variable map from sketch entities
    fn build_from_sketch(&mut self, sketch: &Sketch) {
        self.point_indices.clear();
        self.count = 0;

        // Only points are variables (curves are defined by their control points)
        for entity in sketch.entities_iter() {
            if let SketchEntity::Point { id, .. } = entity {
                self.point_indices.insert(*id, self.count);
                self.count += 2; // x and y
            }
        }
    }

    /// Get all variable values from the sketch
    fn get_values(&self, sketch: &Sketch) -> Vec<f32> {
        let mut values = vec![0.0; self.count];

        for (point_id, index) in &self.point_indices {
            if let Some(SketchEntity::Point { position, .. }) = sketch.get_entity(*point_id) {
                values[*index] = position.x;
                values[index + 1] = position.y;
            }
        }

        values
    }

    /// Set variable values to the sketch
    fn set_values(&self, sketch: &mut Sketch, values: &[f32]) {
        for (point_id, index) in &self.point_indices {
            if let Some(entity) = sketch.get_entity_mut(*point_id)
                && let SketchEntity::Point { position, .. } = entity
            {
                position.x = values[*index];
                position.y = values[index + 1];
            }
        }
    }

    /// Get point position from sketch (for constraint evaluation)
    fn get_point_position(&self, sketch: &Sketch, point_id: Uuid) -> Vec2 {
        sketch
            .get_entity(point_id)
            .and_then(|e| {
                if let SketchEntity::Point { position, .. } = e {
                    Some(*position)
                } else {
                    None
                }
            })
            .unwrap_or(Vec2::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::SketchPlane;

    #[test]
    fn test_simple_horizontal_constraint() {
        let mut sketch = Sketch::new("test", SketchPlane::xy());

        // Create two points
        let p1 = sketch.add_point(Vec2::new(0.0, 0.0));
        let p2 = sketch.add_point(Vec2::new(10.0, 5.0)); // Not horizontal

        // Create a line
        let line = sketch.add_line(p1, p2);

        // Add horizontal constraint
        sketch
            .add_constraint(SketchConstraint::horizontal(line))
            .unwrap();

        // Solve
        let result = sketch.solve();

        // Check that solver didn't fail
        assert!(
            !matches!(result, SolveResult::Failed { .. }),
            "Solver should not fail"
        );

        // Check that the line is now horizontal (y coordinates should be close)
        let pos1 = sketch.get_entity(p1).unwrap().position().unwrap();
        let pos2 = sketch.get_entity(p2).unwrap().position().unwrap();
        assert!(
            (pos1.y - pos2.y).abs() < 0.01,
            "Line should be horizontal: y1={}, y2={}",
            pos1.y,
            pos2.y
        );
    }

    #[test]
    fn test_fixed_constraint() {
        let mut sketch = Sketch::new("test", SketchPlane::xy());

        let p = sketch.add_point(Vec2::new(5.0, 5.0));

        sketch
            .add_constraint(SketchConstraint::fixed(p, 0.0, 0.0))
            .unwrap();

        let result = sketch.solve();

        // Should be fully constrained or at least converge
        assert!(
            matches!(
                result,
                SolveResult::FullyConstrained | SolveResult::UnderConstrained { .. }
            ),
            "Solver should converge: {:?}",
            result
        );

        let pos = sketch.get_entity(p).unwrap().position().unwrap();
        assert!(pos.x.abs() < 0.01, "Point should be at x=0, got {}", pos.x);
        assert!(pos.y.abs() < 0.01, "Point should be at y=0, got {}", pos.y);
    }

    #[test]
    fn test_distance_constraint() {
        let mut sketch = Sketch::new("test", SketchPlane::xy());

        let p1 = sketch.add_point(Vec2::new(0.0, 0.0));
        let p2 = sketch.add_point(Vec2::new(5.0, 0.0));

        // Fix p1
        sketch
            .add_constraint(SketchConstraint::fixed(p1, 0.0, 0.0))
            .unwrap();

        // Set distance
        sketch
            .add_constraint(SketchConstraint::distance(p1, p2, 10.0))
            .unwrap();

        let result = sketch.solve();

        // Should converge (either under-constrained with 1 DOF or fully constrained)
        assert!(
            !matches!(result, SolveResult::Failed { .. }),
            "Solver should not fail: {:?}",
            result
        );

        // Check distance constraint is satisfied
        let pos1 = sketch.get_entity(p1).unwrap().position().unwrap();
        let pos2 = sketch.get_entity(p2).unwrap().position().unwrap();
        let dist = (pos2 - pos1).length();
        assert!(
            (dist - 10.0).abs() < 0.1,
            "Distance should be 10, got {}",
            dist
        );

        // p1 should still be at origin
        assert!(
            pos1.x.abs() < 0.01 && pos1.y.abs() < 0.01,
            "p1 should be at origin, got {:?}",
            pos1
        );
    }
}
