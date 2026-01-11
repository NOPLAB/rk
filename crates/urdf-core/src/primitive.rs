//! Primitive mesh generation for URDF geometry types
//!
//! Generates vertices, normals, and indices for basic shapes:
//! - Box (rectangular prism)
//! - Cylinder (with end caps)
//! - Sphere (UV sphere)

use std::f32::consts::PI;

/// Mesh data: vertices, normals, and triangle indices
pub type MeshData = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>);

/// Generate a box mesh with the given dimensions
///
/// # Arguments
/// * `size` - [width (x), depth (y), height (z)]
///
/// # Returns
/// (vertices, normals, indices) - 24 vertices (4 per face), 12 triangles
pub fn generate_box_mesh(size: [f32; 3]) -> MeshData {
    let hx = size[0] / 2.0;
    let hy = size[1] / 2.0;
    let hz = size[2] / 2.0;

    // 6 faces, 4 vertices each (for proper normals)
    let mut vertices = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    // Helper to add a face
    let mut add_face = |corners: [[f32; 3]; 4], normal: [f32; 3]| {
        let base = vertices.len() as u32;
        for corner in corners {
            vertices.push(corner);
            normals.push(normal);
        }
        // Two triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    // +X face
    add_face(
        [[hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz], [hx, -hy, hz]],
        [1.0, 0.0, 0.0],
    );

    // -X face
    add_face(
        [
            [-hx, hy, -hz],
            [-hx, -hy, -hz],
            [-hx, -hy, hz],
            [-hx, hy, hz],
        ],
        [-1.0, 0.0, 0.0],
    );

    // +Y face
    add_face(
        [[hx, hy, -hz], [-hx, hy, -hz], [-hx, hy, hz], [hx, hy, hz]],
        [0.0, 1.0, 0.0],
    );

    // -Y face
    add_face(
        [
            [-hx, -hy, -hz],
            [hx, -hy, -hz],
            [hx, -hy, hz],
            [-hx, -hy, hz],
        ],
        [0.0, -1.0, 0.0],
    );

    // +Z face (top)
    add_face(
        [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]],
        [0.0, 0.0, 1.0],
    );

    // -Z face (bottom)
    add_face(
        [
            [-hx, hy, -hz],
            [hx, hy, -hz],
            [hx, -hy, -hz],
            [-hx, -hy, -hz],
        ],
        [0.0, 0.0, -1.0],
    );

    (vertices, normals, indices)
}

/// Generate a cylinder mesh along the Z axis
///
/// # Arguments
/// * `radius` - Cylinder radius
/// * `length` - Cylinder length (height along Z)
/// * `segments` - Number of segments around circumference (default: 32)
///
/// # Returns
/// (vertices, normals, indices)
pub fn generate_cylinder_mesh(radius: f32, length: f32) -> MeshData {
    generate_cylinder_mesh_with_segments(radius, length, 32)
}

/// Generate a cylinder mesh with custom segment count
pub fn generate_cylinder_mesh_with_segments(radius: f32, length: f32, segments: u32) -> MeshData {
    let half_length = length / 2.0;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Generate side vertices
    for i in 0..=segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        let nx = theta.cos();
        let ny = theta.sin();

        // Bottom vertex
        vertices.push([x, y, -half_length]);
        normals.push([nx, ny, 0.0]);

        // Top vertex
        vertices.push([x, y, half_length]);
        normals.push([nx, ny, 0.0]);
    }

    // Side triangles
    for i in 0..segments {
        let base = i * 2;
        // Triangle 1
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);
        // Triangle 2
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    // Top cap center
    let top_center_idx = vertices.len() as u32;
    vertices.push([0.0, 0.0, half_length]);
    normals.push([0.0, 0.0, 1.0]);

    // Top cap rim vertices
    let top_rim_start = vertices.len() as u32;
    for i in 0..=segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        vertices.push([x, y, half_length]);
        normals.push([0.0, 0.0, 1.0]);
    }

    // Top cap triangles
    for i in 0..segments {
        indices.push(top_center_idx);
        indices.push(top_rim_start + i);
        indices.push(top_rim_start + i + 1);
    }

    // Bottom cap center
    let bottom_center_idx = vertices.len() as u32;
    vertices.push([0.0, 0.0, -half_length]);
    normals.push([0.0, 0.0, -1.0]);

    // Bottom cap rim vertices
    let bottom_rim_start = vertices.len() as u32;
    for i in 0..=segments {
        let theta = (i as f32 / segments as f32) * 2.0 * PI;
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        vertices.push([x, y, -half_length]);
        normals.push([0.0, 0.0, -1.0]);
    }

    // Bottom cap triangles (reversed winding)
    for i in 0..segments {
        indices.push(bottom_center_idx);
        indices.push(bottom_rim_start + i + 1);
        indices.push(bottom_rim_start + i);
    }

    (vertices, normals, indices)
}

/// Generate a UV sphere mesh
///
/// # Arguments
/// * `radius` - Sphere radius
///
/// # Returns
/// (vertices, normals, indices)
pub fn generate_sphere_mesh(radius: f32) -> MeshData {
    generate_sphere_mesh_with_segments(radius, 16, 32)
}

/// Generate a UV sphere mesh with custom resolution
///
/// # Arguments
/// * `radius` - Sphere radius
/// * `lat_segments` - Number of latitude bands (default: 16)
/// * `lon_segments` - Number of longitude segments (default: 32)
pub fn generate_sphere_mesh_with_segments(
    radius: f32,
    lat_segments: u32,
    lon_segments: u32,
) -> MeshData {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for lat in 0..=lat_segments {
        let theta = (lat as f32 / lat_segments as f32) * PI; // 0 to PI
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = (lon as f32 / lon_segments as f32) * 2.0 * PI; // 0 to 2*PI
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = sin_theta * sin_phi;
            let z = cos_theta;

            vertices.push([radius * x, radius * y, radius * z]);
            normals.push([x, y, z]);
        }
    }

    // Generate indices
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let current = lat * (lon_segments + 1) + lon;
            let next = current + lon_segments + 1;

            // Triangle 1
            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            // Triangle 2
            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    (vertices, normals, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_mesh() {
        let (vertices, normals, indices) = generate_box_mesh([1.0, 1.0, 1.0]);
        assert_eq!(vertices.len(), 24); // 6 faces * 4 vertices
        assert_eq!(normals.len(), 24);
        assert_eq!(indices.len(), 36); // 6 faces * 2 triangles * 3 indices
    }

    #[test]
    fn test_cylinder_mesh() {
        let (vertices, normals, indices) = generate_cylinder_mesh(0.5, 1.0);
        assert!(!vertices.is_empty());
        assert_eq!(vertices.len(), normals.len());
        assert!(!indices.is_empty());
        assert!(indices.len() % 3 == 0); // Valid triangles
    }

    #[test]
    fn test_sphere_mesh() {
        let (vertices, normals, indices) = generate_sphere_mesh(1.0);
        assert!(!vertices.is_empty());
        assert_eq!(vertices.len(), normals.len());
        assert!(!indices.is_empty());
        assert!(indices.len() % 3 == 0);
    }

    #[test]
    fn test_box_dimensions() {
        let (vertices, _, _) = generate_box_mesh([2.0, 4.0, 6.0]);
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for v in &vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        assert!((max[0] - min[0] - 2.0).abs() < 0.001);
        assert!((max[1] - min[1] - 4.0).abs() < 0.001);
        assert!((max[2] - min[2] - 6.0).abs() < 0.001);
    }
}
