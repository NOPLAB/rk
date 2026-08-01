//! Mesh file loading (STL, OBJ, DAE, STEP formats)

mod dae;
mod normals;
mod obj;
#[cfg(feature = "cad")]
mod step;
mod stl;

use std::path::Path;

use crate::part::Part;

pub use dae::{load_dae, load_dae_from_bytes, load_dae_with_unit};
pub use normals::{calculate_face_normals, calculate_triangle_normal};
pub use obj::{load_obj, load_obj_from_bytes, load_obj_with_unit};
#[cfg(feature = "cad")]
pub use step::{StepLoadOptions, load_step, load_step_with_options};
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
    Step,
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
            Some("step") | Some("stp") => MeshFormat::Step,
            _ => MeshFormat::Unknown,
        }
    }

    /// Check if the format is supported
    pub fn is_supported(&self) -> bool {
        match self {
            MeshFormat::Stl | MeshFormat::Obj | MeshFormat::Dae => true,
            #[cfg(feature = "cad")]
            MeshFormat::Step => true,
            #[cfg(not(feature = "cad"))]
            MeshFormat::Step => false,
            MeshFormat::Unknown => false,
        }
    }

    /// Get format name
    pub fn name(&self) -> &'static str {
        match self {
            MeshFormat::Stl => "STL",
            MeshFormat::Obj => "OBJ",
            MeshFormat::Dae => "DAE (COLLADA)",
            MeshFormat::Step => "STEP",
            MeshFormat::Unknown => "Unknown",
        }
    }

    /// Check if format requires CAD kernel
    pub fn requires_cad_kernel(&self) -> bool {
        matches!(self, MeshFormat::Step)
    }
}

/// Load any supported mesh format
///
/// For STEP files with multiple solids, this returns only the first part.
/// Use `load_mesh_multi` to get all parts from a STEP file.
pub fn load_mesh(path: impl AsRef<Path>, unit: StlUnit) -> Result<Part, MeshError> {
    let path = path.as_ref();
    let format = MeshFormat::from_path(path);

    match format {
        MeshFormat::Stl => {
            stl::load_stl_with_unit(path, unit).map_err(|e| MeshError::Parse(e.to_string()))
        }
        MeshFormat::Obj => load_obj_with_unit(path, unit),
        MeshFormat::Dae => load_dae_with_unit(path, unit),
        #[cfg(feature = "cad")]
        MeshFormat::Step => {
            let parts = step::load_step(path)?;
            parts.into_iter().next().ok_or(MeshError::EmptyMesh)
        }
        #[cfg(not(feature = "cad"))]
        MeshFormat::Step => Err(MeshError::UnsupportedFormat(
            "STEP import requires the 'cad' feature to be enabled".into(),
        )),
        MeshFormat::Unknown => Err(MeshError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        )),
    }
}

/// Load any supported mesh format, returning multiple parts for multi-body files
///
/// This is useful for STEP files that may contain multiple solids.
pub fn load_mesh_multi(path: impl AsRef<Path>, unit: StlUnit) -> Result<Vec<Part>, MeshError> {
    let path = path.as_ref();
    let format = MeshFormat::from_path(path);

    match format {
        MeshFormat::Stl => {
            let part =
                stl::load_stl_with_unit(path, unit).map_err(|e| MeshError::Parse(e.to_string()))?;
            Ok(vec![part])
        }
        MeshFormat::Obj => {
            let part = load_obj_with_unit(path, unit)?;
            Ok(vec![part])
        }
        MeshFormat::Dae => {
            let part = load_dae_with_unit(path, unit)?;
            Ok(vec![part])
        }
        #[cfg(feature = "cad")]
        MeshFormat::Step => step::load_step(path),
        #[cfg(not(feature = "cad"))]
        MeshFormat::Step => Err(MeshError::UnsupportedFormat(
            "STEP import requires the 'cad' feature to be enabled".into(),
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
