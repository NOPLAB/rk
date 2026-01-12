//! Gizmo collision detection utilities
//!
//! This module implements ray-casting collision detection for gizmo handle picking.
//! It provides functions for testing ray-cylinder and ray-ring intersections.

use glam::Vec3;

/// Ray-cylinder intersection test.
///
/// Tests if a ray intersects with a finite cylinder defined by its axis
/// endpoints and radius.
///
/// # Algorithm
///
/// The algorithm works in two steps:
///
/// 1. **Infinite cylinder intersection**: Projects the ray and cylinder axis
///    into the plane perpendicular to the cylinder axis, then solves the
///    resulting 2D quadratic equation.
///
/// 2. **Finite bounds check**: Verifies that the intersection point lies
///    within the finite cylinder bounds (between `cylinder_start` and
///    `cylinder_end`).
///
/// The quadratic equation is derived from the parametric ray equation:
/// ```text
/// P(t) = ray_origin + t * ray_dir
/// ```
///
/// For a point on the cylinder surface, the distance from the axis must
/// equal the radius. This gives us:
/// ```text
/// |P(t) - axis_projection(P(t))| = radius
/// ```
///
/// Expanding and solving for `t` yields a quadratic equation `at² + bt + c = 0`.
///
/// # Arguments
///
/// * `ray_origin` - The starting point of the ray.
/// * `ray_dir` - The direction of the ray (should be normalized).
/// * `cylinder_start` - The starting point of the cylinder axis.
/// * `cylinder_end` - The ending point of the cylinder axis.
/// * `radius` - The radius of the cylinder.
///
/// # Returns
///
/// * `Some(t)` - The ray parameter at the closest intersection point.
///   The hit point can be computed as `ray_origin + t * ray_dir`.
/// * `None` - If the ray does not intersect the cylinder.
///
/// # Example
///
/// ```ignore
/// let ray_origin = Vec3::new(0.0, 0.0, 5.0);
/// let ray_dir = Vec3::new(0.0, 0.0, -1.0);
/// let cylinder_start = Vec3::ZERO;
/// let cylinder_end = Vec3::new(1.0, 0.0, 0.0);
/// let radius = 0.1;
///
/// if let Some(t) = ray_cylinder_intersection(
///     ray_origin, ray_dir, cylinder_start, cylinder_end, radius
/// ) {
///     let hit_point = ray_origin + ray_dir * t;
///     println!("Hit at: {:?}", hit_point);
/// }
/// ```
pub fn ray_cylinder_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    cylinder_start: Vec3,
    cylinder_end: Vec3,
    radius: f32,
) -> Option<f32> {
    // Calculate cylinder axis properties
    let cylinder_axis = (cylinder_end - cylinder_start).normalize();
    let cylinder_length = (cylinder_end - cylinder_start).length();

    // Project ray direction and origin offset onto the plane perpendicular to cylinder axis
    // d = ray_dir - (ray_dir · axis) * axis
    let d = ray_dir - cylinder_axis * ray_dir.dot(cylinder_axis);
    // o = (ray_origin - cylinder_start) - ((ray_origin - cylinder_start) · axis) * axis
    let o = (ray_origin - cylinder_start)
        - cylinder_axis * (ray_origin - cylinder_start).dot(cylinder_axis);

    // Quadratic coefficients: at² + bt + c = 0
    let a = d.dot(d);
    let b = 2.0 * d.dot(o);
    let c = o.dot(o) - radius * radius;

    // Solve quadratic equation
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    // Get the closest positive intersection
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    if t < 0.0 {
        return None;
    }

    // Check if hit point is within cylinder length bounds
    let hit_point = ray_origin + ray_dir * t;
    let projection = (hit_point - cylinder_start).dot(cylinder_axis);
    if projection < 0.0 || projection > cylinder_length {
        return None;
    }

    Some(t)
}

/// Ray-ring intersection test.
///
/// Tests if a ray intersects with a ring (circle with thickness) in 3D space.
/// The ring is defined by its center, normal (axis), radius, and thickness.
///
/// # Algorithm
///
/// 1. Find intersection of ray with the ring's plane
/// 2. Check if the intersection point is within the ring's annular region
///    (distance from center is close to ring radius, within thickness)
///
/// # Arguments
///
/// * `ray_origin` - The starting point of the ray.
/// * `ray_dir` - The direction of the ray (should be normalized).
/// * `ring_center` - The center point of the ring.
/// * `ring_normal` - The normal vector of the ring's plane (axis of rotation).
/// * `ring_radius` - The radius of the ring (distance from center to ring center line).
/// * `thickness` - The thickness of the ring (hit tolerance).
///
/// # Returns
///
/// * `Some(t)` - The ray parameter at the intersection point.
/// * `None` - If the ray does not intersect the ring.
pub fn ray_ring_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    ring_center: Vec3,
    ring_normal: Vec3,
    ring_radius: f32,
    thickness: f32,
) -> Option<f32> {
    // Find ray-plane intersection
    let denom = ray_dir.dot(ring_normal);

    // Ray is nearly parallel to the plane
    if denom.abs() < 1e-6 {
        return None;
    }

    let t = (ring_center - ray_origin).dot(ring_normal) / denom;

    // Intersection is behind the ray origin
    if t < 0.0 {
        return None;
    }

    // Calculate intersection point
    let hit_point = ray_origin + ray_dir * t;

    // Calculate distance from ring center in the plane
    let offset = hit_point - ring_center;
    let distance_from_center = offset.length();

    // Check if the point is within the ring's annular region
    let distance_from_ring = (distance_from_center - ring_radius).abs();

    if distance_from_ring <= thickness {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_hits_cylinder() {
        // Ray pointing straight at X-axis cylinder
        let ray_origin = Vec3::new(0.5, 0.0, 1.0);
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        let cylinder_start = Vec3::ZERO;
        let cylinder_end = Vec3::new(1.0, 0.0, 0.0);
        let radius = 0.1;

        let result =
            ray_cylinder_intersection(ray_origin, ray_dir, cylinder_start, cylinder_end, radius);
        assert!(result.is_some());
    }

    #[test]
    fn test_ray_misses_cylinder() {
        // Ray pointing away from cylinder
        let ray_origin = Vec3::new(0.5, 0.0, 1.0);
        let ray_dir = Vec3::new(0.0, 0.0, 1.0);
        let cylinder_start = Vec3::ZERO;
        let cylinder_end = Vec3::new(1.0, 0.0, 0.0);
        let radius = 0.1;

        let result =
            ray_cylinder_intersection(ray_origin, ray_dir, cylinder_start, cylinder_end, radius);
        assert!(result.is_none());
    }

    #[test]
    fn test_ray_outside_cylinder_bounds() {
        // Ray hits infinite cylinder but outside finite bounds
        let ray_origin = Vec3::new(2.0, 0.0, 1.0);
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        let cylinder_start = Vec3::ZERO;
        let cylinder_end = Vec3::new(1.0, 0.0, 0.0);
        let radius = 0.1;

        let result =
            ray_cylinder_intersection(ray_origin, ray_dir, cylinder_start, cylinder_end, radius);
        assert!(result.is_none());
    }
}
