//! Mesh file loading (STL, OBJ, DAE formats)

mod dae;
mod normals;
mod obj;
mod stl;

use std::path::Path;

use crate::part::Part;

pub use dae::{load_dae, load_dae_with_unit};
pub use normals::{calculate_face_normals, calculate_triangle_normal};
pub use obj::{load_obj, load_obj_with_unit};
pub use stl::{StlError, StlUnit, load_stl, load_stl_from_bytes, load_stl_with_unit, save_stl};

/// Raw mesh data extracted from a file (before Part creation)
pub(crate) struct RawMeshData {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Finalize a Part from raw mesh data
///
/// This handles the common post-processing steps:
/// - Setting the mesh path
/// - Calculating bounding box
/// - Calculating default inertia from bounding box
pub(crate) fn finalize_part(part: &mut Part, mesh_path: Option<String>, mesh_data: RawMeshData) {
    part.stl_path = mesh_path;
    part.vertices = mesh_data.vertices;
    part.normals = mesh_data.normals;
    part.indices = mesh_data.indices;
    part.calculate_bounding_box();
    part.inertia =
        crate::inertia::InertiaMatrix::from_bounding_box(part.mass, part.bbox_min, part.bbox_max);
}

/// Extract name and path from a file path for Part creation
pub(crate) fn extract_name_and_path(path: &Path) -> (String, Option<String>) {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string();
    let mesh_path = Some(path.to_string_lossy().to_string());
    (name, mesh_path)
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
        matches!(self, MeshFormat::Stl | MeshFormat::Obj | MeshFormat::Dae)
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
            stl::load_stl_with_unit(path, unit).map_err(|e| MeshError::Parse(e.to_string()))
        }
        MeshFormat::Obj => load_obj_with_unit(path, unit),
        MeshFormat::Dae => load_dae_with_unit(path, unit),
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
