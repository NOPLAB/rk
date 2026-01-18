//! Geometry processing and mesh path resolution

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::assembly::{CollisionElement, Pose, VisualElement};
use crate::inertia::InertiaMatrix;
use crate::mesh::{MeshFormat, load_mesh};
use crate::part::Part;
use crate::primitive::{generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh};
use crate::types::GeometryType;

use super::ImportError;
use super::options::ImportOptions;

/// Context for geometry processing operations
pub struct GeometryContext<'a> {
    /// Base directory for resolving relative paths
    pub base_dir: &'a Path,
    /// Import options
    pub options: &'a ImportOptions,
    /// Material name to color mapping
    pub material_colors: &'a HashMap<String, [f32; 4]>,
    /// Package name to path mapping for package:// URIs
    pub package_paths: &'a HashMap<String, PathBuf>,
}

/// Process visual geometry elements and create a Part if mesh is found
pub fn process_visual_geometry(
    visuals: &[urdf_rs::Visual],
    link_name: &str,
    ctx: &GeometryContext,
) -> Result<(Option<Part>, Vec<VisualElement>), ImportError> {
    if visuals.is_empty() {
        return Ok((None, Vec::new()));
    }

    // Process first visual element (primary) - used to create Part
    let first_visual = &visuals[0];
    let (color, material_name, _texture) =
        extract_material_info(first_visual, ctx.material_colors, ctx.options);
    let origin = Pose::from(&first_visual.origin);

    // Create part from first visual's geometry
    let mut part = process_geometry(
        &first_visual.geometry,
        link_name,
        ctx,
        color,
        material_name.clone(),
    )?;

    // Apply visual origin to part's origin_transform
    if let Some(ref mut p) = part {
        p.origin_transform = origin.to_mat4();
    }

    // Process all visual elements
    let mut visual_elements = Vec::new();
    for (i, visual) in visuals.iter().enumerate() {
        let (elem_color, elem_material, elem_texture) =
            extract_material_info(visual, ctx.material_colors, ctx.options);
        let elem_origin = Pose::from(&visual.origin);
        let elem_geometry = GeometryType::from(&visual.geometry);

        visual_elements.push(VisualElement {
            name: visual
                .name
                .clone()
                .or_else(|| Some(format!("visual_{}", i))),
            origin: elem_origin,
            color: elem_color,
            material_name: elem_material,
            texture: elem_texture,
            geometry: elem_geometry,
        });
    }

    Ok((part, visual_elements))
}

/// Extract material color, name, and texture from a visual element
pub fn extract_material_info(
    visual: &urdf_rs::Visual,
    material_colors: &HashMap<String, [f32; 4]>,
    options: &ImportOptions,
) -> ([f32; 4], Option<String>, Option<String>) {
    if let Some(ref mat) = visual.material {
        let color = mat
            .color
            .as_ref()
            .map(|c| {
                [
                    c.rgba.0[0] as f32,
                    c.rgba.0[1] as f32,
                    c.rgba.0[2] as f32,
                    c.rgba.0[3] as f32,
                ]
            })
            .or_else(|| material_colors.get(&mat.name).copied())
            .unwrap_or(options.default_color);

        let name = if mat.name.is_empty() {
            None
        } else {
            Some(mat.name.clone())
        };

        // Extract texture filename if present
        let texture = mat.texture.as_ref().map(|t| t.filename.clone());

        (color, name, texture)
    } else {
        (options.default_color, None, None)
    }
}

/// Process a single geometry element and create a Part if applicable
pub fn process_geometry(
    geometry: &urdf_rs::Geometry,
    link_name: &str,
    ctx: &GeometryContext,
    color: [f32; 4],
    material_name: Option<String>,
) -> Result<Option<Part>, ImportError> {
    let part = match geometry {
        urdf_rs::Geometry::Mesh { filename, scale } => {
            let mesh_path = resolve_mesh_path(filename, ctx.base_dir, ctx.package_paths)?;
            let mut part =
                load_mesh(&mesh_path, ctx.options.stl_unit).map_err(|e| ImportError::MeshLoad {
                    path: filename.clone(),
                    reason: e.to_string(),
                })?;

            // Apply scale if specified
            if let Some(s) = scale {
                apply_scale(&mut part, [s.0[0] as f32, s.0[1] as f32, s.0[2] as f32]);
            }

            part.name = link_name.to_string();
            part.color = color;
            part.material_name = material_name;

            Some(part)
        }

        urdf_rs::Geometry::Box { size } => {
            let size = [size.0[0] as f32, size.0[1] as f32, size.0[2] as f32];
            Some(create_primitive_part(
                link_name,
                generate_box_mesh(size),
                color,
                material_name,
            ))
        }

        urdf_rs::Geometry::Cylinder { radius, length } => Some(create_primitive_part(
            link_name,
            generate_cylinder_mesh(*radius as f32, *length as f32),
            color,
            material_name,
        )),

        urdf_rs::Geometry::Sphere { radius } => Some(create_primitive_part(
            link_name,
            generate_sphere_mesh(*radius as f32),
            color,
            material_name,
        )),

        // Approximate capsule as cylinder (capsule mesh generation would be more complex)
        urdf_rs::Geometry::Capsule { radius, length } => Some(create_primitive_part(
            link_name,
            generate_cylinder_mesh(*radius as f32, *length as f32),
            color,
            material_name,
        )),
    };

    Ok(part)
}

/// Process collision geometry elements
pub fn process_collision_geometry(collisions: &[urdf_rs::Collision]) -> Vec<CollisionElement> {
    collisions
        .iter()
        .enumerate()
        .map(|(i, collision)| CollisionElement {
            name: collision
                .name
                .clone()
                .or_else(|| Some(format!("collision_{}", i))),
            origin: Pose::from(&collision.origin),
            geometry: GeometryType::from(&collision.geometry),
        })
        .collect()
}

/// Create a Part from primitive geometry mesh data (box, cylinder, sphere, capsule)
fn create_primitive_part(
    name: &str,
    mesh_data: (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>),
    color: [f32; 4],
    material_name: Option<String>,
) -> Part {
    let (vertices, normals, indices) = mesh_data;
    create_part_from_mesh(name, vertices, normals, indices, color, material_name)
}

/// Create a Part from mesh data
pub fn create_part_from_mesh(
    name: &str,
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    color: [f32; 4],
    material_name: Option<String>,
) -> Part {
    let mut part = Part::new(name);
    part.vertices = vertices;
    part.normals = normals;
    part.indices = indices;
    part.color = color;
    part.material_name = material_name;
    part.calculate_bounding_box();

    // Calculate inertia from bounding box
    part.inertia = InertiaMatrix::from_bounding_box(part.mass, part.bbox_min, part.bbox_max);

    part
}

/// Resolve mesh path from URDF filename reference
pub fn resolve_mesh_path(
    filename: &str,
    base_dir: &Path,
    package_paths: &HashMap<String, PathBuf>,
) -> Result<PathBuf, ImportError> {
    // Handle package:// URI
    if let Some(rest) = filename.strip_prefix("package://") {
        return resolve_package_uri(rest, filename, package_paths, base_dir);
    }

    // Handle file:// URI
    let path_str = if let Some(stripped) = filename.strip_prefix("file://") {
        stripped
    } else {
        filename
    };

    // Check for supported format
    let format = MeshFormat::from_path(Path::new(path_str));
    if !format.is_supported() {
        return Err(ImportError::UnsupportedMeshFormat(format!(
            "{} ({})",
            filename,
            format.name()
        )));
    }

    // Resolve relative path
    let path = if Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else {
        base_dir.join(path_str)
    };

    if !path.exists() {
        return Err(ImportError::MeshNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    Ok(path)
}

/// Resolve a package:// URI to a filesystem path
pub fn resolve_package_uri(
    rest: &str,
    original_uri: &str,
    package_paths: &HashMap<String, PathBuf>,
    base_dir: &Path,
) -> Result<PathBuf, ImportError> {
    // Parse package://package_name/path/to/file
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    let package_name = parts[0];
    let relative_path = parts.get(1).unwrap_or(&"");

    // Check for supported format
    let format = MeshFormat::from_path(Path::new(relative_path));
    if !format.is_supported() {
        return Err(ImportError::UnsupportedMeshFormat(format!(
            "{} ({})",
            original_uri,
            format.name()
        )));
    }

    // Try explicit package path mapping first
    if let Some(package_root) = package_paths.get(package_name) {
        let path = package_root.join(relative_path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback: try to find package relative to base_dir
    // This handles common cases where URDF is inside a package
    let fallback_paths = [
        // Same directory as URDF
        base_dir.join(relative_path),
        // Parent directory (URDF might be in urdf/ subdirectory)
        base_dir.join("..").join(relative_path),
        // Look for package_name directory relative to base_dir
        base_dir.join("..").join(package_name).join(relative_path),
        // Two levels up (common in ROS workspace layouts)
        base_dir
            .join("..")
            .join("..")
            .join(package_name)
            .join(relative_path),
    ];

    for path in &fallback_paths {
        if let Ok(canonical) = path.canonicalize()
            && canonical.exists()
        {
            return Ok(canonical);
        }
    }

    Err(ImportError::PackageNotFound {
        package: package_name.to_string(),
        uri: original_uri.to_string(),
    })
}

/// Apply scale to part vertices
pub fn apply_scale(part: &mut Part, scale: [f32; 3]) {
    for vertex in &mut part.vertices {
        vertex[0] *= scale[0];
        vertex[1] *= scale[1];
        vertex[2] *= scale[2];
    }
    part.calculate_bounding_box();
}
