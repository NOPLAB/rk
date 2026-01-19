//! Object picking functionality

use glam::{Mat4, Vec3};
use uuid::Uuid;

/// Data needed for picking a single part
pub struct PickablePartData {
    pub id: Uuid,
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub transform: Mat4,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

/// Pick the closest object at the given screen position
/// Returns the UUID of the hit object and the distance, if any
pub fn pick_object(
    camera: &rk_renderer::Camera,
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
    parts: &[PickablePartData],
) -> Option<(Uuid, f32)> {
    let (ray_origin, ray_dir) = camera.screen_to_ray(screen_x, screen_y, width, height);

    let mut closest_hit: Option<(Uuid, f32)> = None;

    for part in parts {
        let transform = part.transform;

        // Transform bounding box to world space (approximate with corners)
        let bbox_min = Vec3::from(part.bbox_min);
        let bbox_max = Vec3::from(part.bbox_max);

        // Transform all 8 corners of the bounding box
        let corners = [
            transform.transform_point3(Vec3::new(bbox_min.x, bbox_min.y, bbox_min.z)),
            transform.transform_point3(Vec3::new(bbox_max.x, bbox_min.y, bbox_min.z)),
            transform.transform_point3(Vec3::new(bbox_min.x, bbox_max.y, bbox_min.z)),
            transform.transform_point3(Vec3::new(bbox_max.x, bbox_max.y, bbox_min.z)),
            transform.transform_point3(Vec3::new(bbox_min.x, bbox_min.y, bbox_max.z)),
            transform.transform_point3(Vec3::new(bbox_max.x, bbox_min.y, bbox_max.z)),
            transform.transform_point3(Vec3::new(bbox_min.x, bbox_max.y, bbox_max.z)),
            transform.transform_point3(Vec3::new(bbox_max.x, bbox_max.y, bbox_max.z)),
        ];

        // Compute world-space AABB from transformed corners
        let mut world_min = corners[0];
        let mut world_max = corners[0];
        for corner in &corners[1..] {
            world_min = world_min.min(*corner);
            world_max = world_max.max(*corner);
        }

        // First check AABB for early rejection
        if ray_aabb_intersection(ray_origin, ray_dir, world_min, world_max).is_none() {
            continue;
        }

        // Test each triangle
        for chunk in part.indices.chunks(3) {
            if chunk.len() != 3 {
                continue;
            }

            let v0 = transform.transform_point3(Vec3::from(part.vertices[chunk[0] as usize]));
            let v1 = transform.transform_point3(Vec3::from(part.vertices[chunk[1] as usize]));
            let v2 = transform.transform_point3(Vec3::from(part.vertices[chunk[2] as usize]));

            if let Some(t) = ray_triangle_intersection(ray_origin, ray_dir, v0, v1, v2) {
                match closest_hit {
                    None => closest_hit = Some((part.id, t)),
                    Some((_, current_t)) if t < current_t => closest_hit = Some((part.id, t)),
                    _ => {}
                }
            }
        }
    }

    closest_hit
}

/// Ray-AABB (Axis-Aligned Bounding Box) intersection test
/// Returns the distance to intersection if hit, None otherwise
fn ray_aabb_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    bbox_min: Vec3,
    bbox_max: Vec3,
) -> Option<f32> {
    let inv_dir = Vec3::new(1.0 / ray_dir.x, 1.0 / ray_dir.y, 1.0 / ray_dir.z);

    let t1 = (bbox_min.x - ray_origin.x) * inv_dir.x;
    let t2 = (bbox_max.x - ray_origin.x) * inv_dir.x;
    let t3 = (bbox_min.y - ray_origin.y) * inv_dir.y;
    let t4 = (bbox_max.y - ray_origin.y) * inv_dir.y;
    let t5 = (bbox_min.z - ray_origin.z) * inv_dir.z;
    let t6 = (bbox_max.z - ray_origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }

    Some(if tmin < 0.0 { tmax } else { tmin })
}

/// Ray-triangle intersection using Möller–Trumbore algorithm
/// Returns the distance to intersection if hit, None otherwise
fn ray_triangle_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    const EPSILON: f32 = 1e-6;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_dir.cross(edge2);
    let a = edge1.dot(h);

    if a.abs() < EPSILON {
        return None; // Ray is parallel to triangle
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_dir.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);

    if t > EPSILON { Some(t) } else { None }
}
