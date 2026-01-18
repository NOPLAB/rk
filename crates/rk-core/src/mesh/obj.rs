//! OBJ mesh file loading

use std::path::Path;

use crate::part::Part;

use super::MeshError;
use super::normals::calculate_face_normals;
use super::stl::StlUnit;

/// Load an OBJ file and create a Part
pub fn load_obj(path: impl AsRef<Path>) -> Result<Part, MeshError> {
    load_obj_with_unit(path, StlUnit::Meters)
}

/// Load an OBJ file with specified unit
pub fn load_obj_with_unit(path: impl AsRef<Path>, unit: StlUnit) -> Result<Part, MeshError> {
    let path = path.as_ref();

    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .map_err(|e| MeshError::Parse(e.to_string()))?;

    if models.is_empty() {
        return Err(MeshError::EmptyMesh);
    }

    let scale = unit.scale_factor();

    // Combine all meshes into one
    let mut all_vertices: Vec<[f32; 3]> = Vec::new();
    let mut all_normals: Vec<[f32; 3]> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    for model in &models {
        let mesh = &model.mesh;
        let vertex_offset = all_vertices.len() as u32;

        // Extract vertices (positions are in groups of 3)
        for chunk in mesh.positions.chunks(3) {
            if chunk.len() == 3 {
                all_vertices.push([chunk[0] * scale, chunk[1] * scale, chunk[2] * scale]);
            }
        }

        // Extract normals (in groups of 3)
        // OBJ normals are per-vertex, we'll use face normals for URDF compatibility
        let has_normals = !mesh.normals.is_empty();
        if has_normals {
            for chunk in mesh.normals.chunks(3) {
                if chunk.len() == 3 {
                    all_normals.push([chunk[0], chunk[1], chunk[2]]);
                }
            }
        }

        // Extract indices
        for &idx in &mesh.indices {
            all_indices.push(vertex_offset + idx);
        }
    }

    // Calculate face normals if not provided
    if all_normals.is_empty() {
        all_normals = calculate_face_normals(&all_vertices, &all_indices);
    }

    let (name, mesh_path) = super::extract_name_and_path(path);
    let mut part = Part::new(name);
    super::finalize_part(
        &mut part,
        mesh_path,
        super::RawMeshData {
            vertices: all_vertices,
            normals: all_normals,
            indices: all_indices,
        },
    );

    Ok(part)
}
