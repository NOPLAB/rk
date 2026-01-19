//! Reference plane picking logic for the viewport

use glam::Vec3;
use rk_renderer::Camera;

use crate::state::ReferencePlane;

/// Size of the reference planes for picking
const PLANE_SIZE: f32 = 2.0;

/// Pick a reference plane from screen coordinates.
///
/// Returns the closest plane that the ray intersects within the plane bounds,
/// or None if no plane is hit.
pub fn pick_reference_plane(
    camera: &Camera,
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
) -> Option<ReferencePlane> {
    let (ray_origin, ray_dir) = camera.screen_to_ray(screen_x, screen_y, width, height);

    let mut closest: Option<(ReferencePlane, f32)> = None;

    for plane in ReferencePlane::all() {
        if let Some(t) = ray_plane_intersection(ray_origin, ray_dir, plane) {
            // Check if the intersection point is within the plane bounds
            let hit_point = ray_origin + ray_dir * t;

            if is_point_in_plane_bounds(hit_point, plane, PLANE_SIZE) {
                // Check if this is closer than any previous hit
                if closest.is_none() || t < closest.unwrap().1 {
                    closest = Some((plane, t));
                }
            }
        }
    }

    closest.map(|(p, _)| p)
}

/// Calculate ray-plane intersection.
///
/// Returns the parameter t such that ray_origin + ray_dir * t is on the plane,
/// or None if the ray is parallel to the plane.
fn ray_plane_intersection(ray_origin: Vec3, ray_dir: Vec3, plane: ReferencePlane) -> Option<f32> {
    let normal = plane.normal();
    let denom = ray_dir.dot(normal);

    // Check if ray is parallel to plane
    if denom.abs() < 1e-6 {
        return None;
    }

    // Plane passes through origin, so d = 0
    let t = -ray_origin.dot(normal) / denom;

    // Only return positive t (intersection in front of camera)
    if t > 0.0 { Some(t) } else { None }
}

/// Check if a point is within the bounds of a reference plane.
fn is_point_in_plane_bounds(point: Vec3, plane: ReferencePlane, size: f32) -> bool {
    match plane {
        ReferencePlane::XY => point.x.abs() <= size && point.y.abs() <= size,
        ReferencePlane::XZ => point.x.abs() <= size && point.z.abs() <= size,
        ReferencePlane::YZ => point.y.abs() <= size && point.z.abs() <= size,
    }
}
