//! STL file loading

use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

use crate::part::Part;

/// Load an STL file and create a Part
pub fn load_stl(path: impl AsRef<Path>) -> Result<Part, StlError> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|e| StlError::Io(e.to_string()))?;
    let mut reader = BufReader::new(file);

    let mesh = stl_io::read_stl(&mut reader).map_err(|e| StlError::Parse(e.to_string()))?;

    // Convert to indexed mesh
    let (vertices, normals, indices) = index_mesh(&mesh);

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string();

    let mut part = Part::new(name);
    part.stl_path = Some(path.to_string_lossy().to_string());
    part.vertices = vertices;
    part.normals = normals;
    part.indices = indices;
    part.calculate_bounding_box();

    // Calculate default inertia from bounding box
    part.inertia = crate::inertia::InertiaMatrix::from_bounding_box(
        part.mass,
        part.bbox_min,
        part.bbox_max,
    );

    Ok(part)
}

/// Convert triangle soup to indexed mesh
fn index_mesh(mesh: &stl_io::IndexedMesh) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
    let mut unique_vertices: Vec<[f32; 3]> = Vec::new();
    let mut vertex_map: HashMap<[i32; 3], u32> = HashMap::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();

    // Precision for vertex comparison (multiply by this, then round to int)
    const PRECISION: f32 = 10000.0;

    for face in &mesh.faces {
        let normal = face.normal;
        normals.push([normal[0], normal[1], normal[2]]);

        for &vertex_idx in &face.vertices {
            let vertex = mesh.vertices[vertex_idx];
            let v = [vertex[0], vertex[1], vertex[2]];

            // Quantize for comparison
            let key = [
                (v[0] * PRECISION) as i32,
                (v[1] * PRECISION) as i32,
                (v[2] * PRECISION) as i32,
            ];

            let index = if let Some(&existing_idx) = vertex_map.get(&key) {
                existing_idx
            } else {
                let new_idx = unique_vertices.len() as u32;
                unique_vertices.push(v);
                vertex_map.insert(key, new_idx);
                new_idx
            };

            indices.push(index);
        }
    }

    (unique_vertices, normals, indices)
}

/// Save a Part as an STL file (with origin transform applied)
pub fn save_stl(part: &Part, path: impl AsRef<Path>) -> Result<(), StlError> {
    let path = path.as_ref();

    // Apply origin transform to vertices
    let transformed_vertices: Vec<[f32; 3]> = part
        .vertices
        .iter()
        .map(|v| {
            let p = part.origin_transform.transform_point3(glam::Vec3::from(*v));
            [p.x, p.y, p.z]
        })
        .collect();

    // Rebuild triangles
    let mut triangles = Vec::new();
    for (i, chunk) in part.indices.chunks(3).enumerate() {
        if chunk.len() != 3 {
            continue;
        }

        let v0 = transformed_vertices[chunk[0] as usize];
        let v1 = transformed_vertices[chunk[1] as usize];
        let v2 = transformed_vertices[chunk[2] as usize];

        // Get or calculate normal
        let normal = if i < part.normals.len() {
            let n = part.normals[i];
            // Transform normal
            let normal_mat = part.origin_transform.inverse().transpose();
            let transformed = normal_mat.transform_vector3(glam::Vec3::from(n)).normalize();
            [transformed.x, transformed.y, transformed.z]
        } else {
            // Calculate normal from vertices
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            if len > 0.0 {
                [cross[0] / len, cross[1] / len, cross[2] / len]
            } else {
                [0.0, 0.0, 1.0]
            }
        };

        triangles.push(stl_io::Triangle {
            normal: stl_io::Normal::new(normal),
            vertices: [
                stl_io::Vertex::new(v0),
                stl_io::Vertex::new(v1),
                stl_io::Vertex::new(v2),
            ],
        });
    }

    let mut file = std::fs::File::create(path).map_err(|e| StlError::Io(e.to_string()))?;
    stl_io::write_stl(&mut file, triangles.iter()).map_err(|e| StlError::Write(e.to_string()))?;

    Ok(())
}

/// STL-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum StlError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Write error: {0}")]
    Write(String),
}
