//! URDF export functionality

mod options;
mod xml;

use std::collections::HashMap;

use uuid::Uuid;

use crate::assembly::Assembly;
use crate::mesh::save_stl;
use crate::part::Part;

pub use options::ExportOptions;
pub use xml::{sanitize_filename, xml_escape};

use xml::generate_urdf_string;

/// Export assembly to URDF (writes files to disk)
pub fn export_urdf(
    assembly: &Assembly,
    parts: &HashMap<Uuid, Part>,
    options: &ExportOptions,
) -> Result<String, ExportError> {
    // Validate assembly
    assembly
        .validate()
        .map_err(|errors| ExportError::Validation(format!("{:?}", errors)))?;

    // Create mesh directory
    let mesh_dir = options.output_dir.join(&options.mesh_prefix);
    std::fs::create_dir_all(&mesh_dir).map_err(|e| ExportError::Io(e.to_string()))?;

    // Export meshes and collect paths
    let mut mesh_paths = HashMap::new();
    for (part_id, part) in parts {
        let filename = sanitize_filename(&part.name) + ".stl";
        let mesh_path = mesh_dir.join(&filename);
        save_stl(part, &mesh_path).map_err(|e| ExportError::MeshExport(e.to_string()))?;

        let uri = if options.use_package_uri {
            format!("package://{}/{}", options.robot_name, options.mesh_prefix) + "/" + &filename
        } else {
            format!("{}/{}", options.mesh_prefix, filename)
        };
        mesh_paths.insert(*part_id, uri);
    }

    // Generate URDF string
    let urdf = generate_urdf_string(assembly, parts, &mesh_paths, &options.robot_name)?;

    // Write URDF file
    let urdf_path = options
        .output_dir
        .join(format!("{}.urdf", options.robot_name));
    std::fs::write(&urdf_path, &urdf).map_err(|e| ExportError::Io(e.to_string()))?;

    Ok(urdf)
}

/// Export assembly to URDF string only (no file I/O, for WASM support)
/// Note: Mesh URIs will be placeholder paths like "meshes/part_name.stl"
pub fn export_urdf_to_string(
    assembly: &Assembly,
    parts: &HashMap<Uuid, Part>,
    robot_name: &str,
) -> Result<String, ExportError> {
    // Validate assembly
    assembly
        .validate()
        .map_err(|errors| ExportError::Validation(format!("{:?}", errors)))?;

    // Generate placeholder mesh paths
    let mut mesh_paths = HashMap::new();
    for (part_id, part) in parts {
        let filename = sanitize_filename(&part.name) + ".stl";
        let uri = format!("meshes/{}", filename);
        mesh_paths.insert(*part_id, uri);
    }

    generate_urdf_string(assembly, parts, &mesh_paths, robot_name)
}

/// Export-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExportError {
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("No root link defined")]
    NoRootLink,
    #[error("IO error: {0}")]
    Io(String),
    #[error("Mesh export failed: {0}")]
    MeshExport(String),
    #[error("Link not found: {0}")]
    LinkNotFound(Uuid),
    #[error("Part not found: {0}")]
    PartNotFound(Uuid),
    #[error("Mesh path not found for part: {0}")]
    MeshNotFound(Uuid),
}
