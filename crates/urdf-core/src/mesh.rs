//! Mesh file loading (OBJ, DAE formats)

use std::path::Path;

use crate::part::Part;
use crate::stl::StlUnit;

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

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string();

    let mut part = Part::new(name);
    part.stl_path = Some(path.to_string_lossy().to_string());
    part.vertices = all_vertices;
    part.normals = all_normals;
    part.indices = all_indices;
    part.calculate_bounding_box();

    // Calculate default inertia from bounding box
    part.inertia =
        crate::inertia::InertiaMatrix::from_bounding_box(part.mass, part.bbox_min, part.bbox_max);

    Ok(part)
}

/// Calculate face normals from vertices and indices
fn calculate_face_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = Vec::new();

    for chunk in indices.chunks(3) {
        if chunk.len() != 3 {
            continue;
        }

        let v0 = vertices[chunk[0] as usize];
        let v1 = vertices[chunk[1] as usize];
        let v2 = vertices[chunk[2] as usize];

        // Calculate edge vectors
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        // Cross product
        let cross = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];

        // Normalize
        let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        let normal = if len > 0.0 {
            [cross[0] / len, cross[1] / len, cross[2] / len]
        } else {
            [0.0, 0.0, 1.0]
        };

        normals.push(normal);
    }

    normals
}

/// Detect mesh format from file extension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshFormat {
    Stl,
    Obj,
    Dae,
    Unknown,
}

impl MeshFormat {
    /// Detect format from file path
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("stl") => MeshFormat::Stl,
            Some("obj") => MeshFormat::Obj,
            Some("dae") => MeshFormat::Dae,
            _ => MeshFormat::Unknown,
        }
    }

    /// Check if the format is supported
    pub fn is_supported(&self) -> bool {
        matches!(self, MeshFormat::Stl | MeshFormat::Obj)
    }

    /// Get format name
    pub fn name(&self) -> &'static str {
        match self {
            MeshFormat::Stl => "STL",
            MeshFormat::Obj => "OBJ",
            MeshFormat::Dae => "DAE (COLLADA)",
            MeshFormat::Unknown => "Unknown",
        }
    }
}

/// Load any supported mesh format
pub fn load_mesh(path: impl AsRef<Path>, unit: StlUnit) -> Result<Part, MeshError> {
    let path = path.as_ref();
    let format = MeshFormat::from_path(path);

    match format {
        MeshFormat::Stl => {
            crate::stl::load_stl_with_unit(path, unit).map_err(|e| MeshError::Parse(e.to_string()))
        }
        MeshFormat::Obj => load_obj_with_unit(path, unit),
        MeshFormat::Dae => Err(MeshError::UnsupportedFormat(
            "DAE (COLLADA) format is not yet supported".to_string(),
        )),
        MeshFormat::Unknown => Err(MeshError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        )),
    }
}

/// Mesh-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MeshError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Empty mesh: no geometry found")]
    EmptyMesh,
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
