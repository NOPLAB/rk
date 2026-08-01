//! STEP file loading via CAD kernel
//!
//! This module requires the `cad` feature to be enabled.

use std::path::Path;

use crate::mesh::{MeshError, RawMeshData, extract_name_and_path, finalize_part};
use crate::part::Part;

/// Options for STEP import
#[derive(Debug, Clone)]
pub struct StepLoadOptions {
    /// Tessellation tolerance (lower = finer mesh, default 0.1)
    pub tessellation_tolerance: f32,
    /// Unit scaling factor (default 0.001, since the kernel reads STEP in mm)
    pub unit_scale: f32,
}

impl Default for StepLoadOptions {
    fn default() -> Self {
        Self {
            tessellation_tolerance: 0.1,
            // STEP files carry their own units and OpenCASCADE's reader
            // normalises whatever it finds to millimetres. The scene is in
            // metres, so a 100 mm bracket imported at 1.0 would land 100 m
            // across. This is not the caller's `StlUnit` choice — that is for
            // formats which declare nothing.
            unit_scale: 0.001,
        }
    }
}

/// Load a STEP file using the CAD kernel, returns multiple parts
pub fn load_step(path: impl AsRef<Path>) -> Result<Vec<Part>, MeshError> {
    load_step_with_options(path, StepLoadOptions::default())
}

/// Load a STEP file with custom options
pub fn load_step_with_options(
    path: impl AsRef<Path>,
    options: StepLoadOptions,
) -> Result<Vec<Part>, MeshError> {
    use rk_cad::{StepImportOptions, default_kernel};

    let path = path.as_ref();
    let kernel = default_kernel();

    // Only a build that asked for no kernel at all lands here — and truck,
    // which reports itself available and then declines the import
    if !kernel.is_available() {
        return Err(MeshError::UnsupportedFormat(
            "STEP import needs the OpenCASCADE kernel, which this build was \
             configured without"
                .into(),
        ));
    }

    let import_options = StepImportOptions {
        tessellation_tolerance: Some(options.tessellation_tolerance),
        import_as_solids: false, // Tessellate immediately for Part creation
    };

    let result = kernel
        .import_step(path, &import_options)
        .map_err(|e| MeshError::Parse(e.to_string()))?;

    let (base_name, mesh_path) = extract_name_and_path(path);

    let parts: Vec<Part> = result
        .meshes
        .into_iter()
        .enumerate()
        .map(|(i, mesh)| {
            let name = result
                .names
                .get(i)
                .and_then(|n| n.clone())
                .unwrap_or_else(|| {
                    if result.names.len() > 1 {
                        format!("{}_{}", base_name, i + 1)
                    } else {
                        base_name.clone()
                    }
                });

            // Apply unit scaling
            let vertices: Vec<[f32; 3]> = mesh
                .vertices
                .into_iter()
                .map(|v| {
                    [
                        v[0] * options.unit_scale,
                        v[1] * options.unit_scale,
                        v[2] * options.unit_scale,
                    ]
                })
                .collect();

            let mut part = Part::new(&name);
            finalize_part(
                &mut part,
                mesh_path.clone(),
                RawMeshData {
                    vertices,
                    normals: mesh.normals,
                    indices: mesh.indices,
                },
            );

            part
        })
        .collect();

    if parts.is_empty() {
        return Err(MeshError::EmptyMesh);
    }

    Ok(parts)
}
