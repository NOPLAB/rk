//! URDF import functionality
//!
//! Imports URDF files and converts them to the internal Project format.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use glam::Vec3;
use uuid::Uuid;

use crate::assembly::{
    Assembly, CollisionElement, InertialProperties, Joint, JointDynamics, JointMimic, Link, Pose,
    VisualElement,
};
use crate::geometry::GeometryType;
use crate::inertia::InertiaMatrix;
use crate::mesh::{MeshFormat, load_mesh};
use crate::part::{JointLimits, JointType, Part};
use crate::primitive::{generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh};
use crate::project::{MaterialDef, Project};
use crate::stl::StlUnit;

/// Import options for URDF loading
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Base directory for resolving relative mesh paths
    pub base_dir: PathBuf,
    /// Unit for imported STL meshes (URDF typically uses meters)
    pub stl_unit: StlUnit,
    /// Default material color if not specified
    pub default_color: [f32; 4],
    /// Package path mappings for resolving package:// URIs
    /// Maps package name to its root directory
    pub package_paths: HashMap<String, PathBuf>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            stl_unit: StlUnit::Meters,
            default_color: [0.7, 0.7, 0.7, 1.0],
            package_paths: HashMap::new(),
        }
    }
}

impl ImportOptions {
    /// Create import options with package paths auto-discovered from ROS environment
    pub fn with_ros_packages() -> Self {
        Self {
            package_paths: discover_ros_packages(),
            ..Self::default()
        }
    }

    /// Add a package path mapping
    pub fn add_package_path(&mut self, package_name: impl Into<String>, path: impl Into<PathBuf>) {
        self.package_paths.insert(package_name.into(), path.into());
    }
}

/// Discover ROS packages from environment variables
fn discover_ros_packages() -> HashMap<String, PathBuf> {
    let mut packages = HashMap::new();

    // Try ROS_PACKAGE_PATH (ROS1 style, colon-separated)
    if let Ok(ros_package_path) = std::env::var("ROS_PACKAGE_PATH") {
        for path_str in ros_package_path.split(':') {
            let path = Path::new(path_str);
            if path.is_dir() {
                discover_packages_in_dir(path, &mut packages);
            }
        }
    }

    // Try AMENT_PREFIX_PATH (ROS2 style, colon-separated)
    if let Ok(ament_prefix_path) = std::env::var("AMENT_PREFIX_PATH") {
        for path_str in ament_prefix_path.split(':') {
            let share_path = Path::new(path_str).join("share");
            if share_path.is_dir() {
                discover_packages_in_dir(&share_path, &mut packages);
            }
        }
    }

    // Try COLCON_PREFIX_PATH (colcon workspaces)
    if let Ok(colcon_prefix_path) = std::env::var("COLCON_PREFIX_PATH") {
        for path_str in colcon_prefix_path.split(':') {
            let share_path = Path::new(path_str).join("share");
            if share_path.is_dir() {
                discover_packages_in_dir(&share_path, &mut packages);
            }
        }
    }

    packages
}

/// Discover packages in a directory (looks for package.xml files)
fn discover_packages_in_dir(dir: &Path, packages: &mut HashMap<String, PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if this directory contains a package.xml
                let package_xml = path.join("package.xml");
                if package_xml.exists()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    packages.insert(name.to_string(), path);
                }
            }
        }
    }
}

/// Errors that can occur during URDF import
#[derive(Debug, Clone, thiserror::Error)]
pub enum ImportError {
    #[error("Failed to parse URDF: {0}")]
    UrdfParse(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Mesh file not found: {path}")]
    MeshNotFound { path: String },

    #[error("Failed to load mesh '{path}': {reason}")]
    MeshLoad { path: String, reason: String },

    #[error("Unsupported mesh format: {0} (only STL is supported)")]
    UnsupportedMeshFormat(String),

    #[error("Package not found: {package} (from URI: {uri})")]
    PackageNotFound { package: String, uri: String },

    #[error("Link not found: {0}")]
    LinkNotFound(String),

    #[error("Empty URDF: no links defined")]
    EmptyUrdf,
}

/// Import a URDF file and create a Project
///
/// # Arguments
/// * `urdf_path` - Path to the URDF file
/// * `options` - Import options
///
/// # Returns
/// A Project containing all parts, links, joints, and materials from the URDF
pub fn import_urdf(urdf_path: &Path, options: &ImportOptions) -> Result<Project, ImportError> {
    // Read and parse URDF
    let robot = urdf_rs::read_file(urdf_path).map_err(|e| ImportError::UrdfParse(e.to_string()))?;

    // Set base_dir to URDF file's parent directory if not specified
    let base_dir = if options.base_dir.as_os_str() == "." {
        urdf_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        options.base_dir.clone()
    };

    if robot.links.is_empty() {
        return Err(ImportError::EmptyUrdf);
    }

    // Collect materials
    let materials: Vec<MaterialDef> = robot
        .materials
        .iter()
        .map(|m| {
            let color = m
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
                .unwrap_or(options.default_color);
            MaterialDef::new(&m.name, color)
        })
        .collect();

    // Create material lookup map
    let material_colors: HashMap<String, [f32; 4]> = materials
        .iter()
        .map(|m| (m.name.clone(), m.color))
        .collect();

    // Build link name -> ID mapping
    let mut link_name_to_id: HashMap<String, Uuid> = HashMap::new();

    // Process links: create Parts and Links
    let mut parts: Vec<Part> = Vec::new();
    let mut links: HashMap<Uuid, Link> = HashMap::new();

    for urdf_link in &robot.links {
        let link_id = Uuid::new_v4();
        link_name_to_id.insert(urdf_link.name.clone(), link_id);

        // Try to load mesh from visual geometry
        let (part_opt, visuals) = process_visual_geometry(
            &urdf_link.visual,
            &urdf_link.name,
            &base_dir,
            options,
            &material_colors,
            &options.package_paths,
        )?;

        // Process inertial properties
        let inertial_props = InertialProperties {
            origin: Pose::from(&urdf_link.inertial.origin),
            mass: urdf_link.inertial.mass.value as f32,
            inertia: InertiaMatrix::from(&urdf_link.inertial.inertia),
        };

        // Process collision properties (supports multiple collision elements)
        let collisions = process_collision_geometry(&urdf_link.collision);

        // Create Part if we have mesh data
        let part_id = if let Some(mut part) = part_opt {
            // Update part with inertial properties from URDF
            part.mass = inertial_props.mass;
            part.inertia = inertial_props.inertia;

            let id = part.id;
            parts.push(part);
            Some(id)
        } else {
            None
        };

        // Create Link
        let link = Link {
            id: link_id,
            name: urdf_link.name.clone(),
            part_id,
            world_transform: glam::Mat4::IDENTITY,
            visuals,
            collisions,
            inertial: inertial_props,
        };

        links.insert(link_id, link);
    }

    // Find root link (link that is not a child of any joint)
    let mut child_links: HashSet<String> = HashSet::new();
    for urdf_joint in &robot.joints {
        child_links.insert(urdf_joint.child.link.clone());
    }

    let root_link_name = robot
        .links
        .iter()
        .find(|l| !child_links.contains(&l.name))
        .map(|l| l.name.clone());

    let root_link_id = root_link_name.and_then(|name| link_name_to_id.get(&name).copied());

    // Build Assembly
    let mut assembly = Assembly::new(&robot.name);
    assembly.root_link = root_link_id;
    assembly.links = links;
    // Rebuild name indices
    assembly.rebuild_indices();

    // Build joint name to ID mapping for mimic resolution
    let mut joint_name_to_id: HashMap<String, Uuid> = HashMap::new();

    // First pass: create joints without mimic
    for urdf_joint in &robot.joints {
        let parent_link_id = link_name_to_id
            .get(&urdf_joint.parent.link)
            .ok_or_else(|| ImportError::LinkNotFound(urdf_joint.parent.link.clone()))?;

        let child_link_id = link_name_to_id
            .get(&urdf_joint.child.link)
            .ok_or_else(|| ImportError::LinkNotFound(urdf_joint.child.link.clone()))?;

        let joint = Joint {
            id: Uuid::new_v4(),
            name: urdf_joint.name.clone(),
            joint_type: JointType::from(&urdf_joint.joint_type),
            parent_link: *parent_link_id,
            child_link: *child_link_id,
            origin: Pose::from(&urdf_joint.origin),
            axis: Vec3::new(
                urdf_joint.axis.xyz.0[0] as f32,
                urdf_joint.axis.xyz.0[1] as f32,
                urdf_joint.axis.xyz.0[2] as f32,
            ),
            limits: Some(JointLimits {
                lower: urdf_joint.limit.lower as f32,
                upper: urdf_joint.limit.upper as f32,
                effort: urdf_joint.limit.effort as f32,
                velocity: urdf_joint.limit.velocity as f32,
            }),
            dynamics: urdf_joint.dynamics.as_ref().map(|d| JointDynamics {
                damping: d.damping as f32,
                friction: d.friction as f32,
            }),
            mimic: None, // Will be resolved in second pass
            parent_joint_point: None,
            child_joint_point: None,
        };

        let joint_id = joint.id;
        joint_name_to_id.insert(urdf_joint.name.clone(), joint_id);
        assembly.joints.insert(joint_id, joint);

        // Update parent-child relationships
        assembly
            .children
            .entry(*parent_link_id)
            .or_default()
            .push((joint_id, *child_link_id));
        assembly
            .parent
            .insert(*child_link_id, (joint_id, *parent_link_id));
    }

    // Second pass: resolve mimic references
    for urdf_joint in &robot.joints {
        if let Some(ref mimic) = urdf_joint.mimic
            && let Some(&mimic_joint_id) = joint_name_to_id.get(&mimic.joint)
            && let Some(&joint_id) = joint_name_to_id.get(&urdf_joint.name)
            && let Some(joint) = assembly.joints.get_mut(&joint_id)
        {
            joint.mimic = Some(JointMimic {
                joint_id: mimic_joint_id,
                multiplier: mimic.multiplier.unwrap_or(1.0) as f32,
                offset: mimic.offset.unwrap_or(0.0) as f32,
            });
        }
    }

    // Create joint points in assembly from joint information
    let joint_ids: Vec<Uuid> = assembly.joints.keys().copied().collect();
    for joint_id in joint_ids {
        let (
            parent_part_id,
            child_part_id,
            child_link_name,
            parent_link_name,
            joint_origin,
            joint_type,
            axis,
            limits,
        ) = {
            let joint = assembly.joints.get(&joint_id).unwrap();
            let parent_part_id = assembly
                .links
                .get(&joint.parent_link)
                .and_then(|l| l.part_id);
            let child_part_id = assembly
                .links
                .get(&joint.child_link)
                .and_then(|l| l.part_id);
            let child_link_name = assembly
                .links
                .get(&joint.child_link)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            let parent_link_name = assembly
                .links
                .get(&joint.parent_link)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            (
                parent_part_id,
                child_part_id,
                child_link_name,
                parent_link_name,
                joint.origin,
                joint.joint_type,
                joint.axis,
                joint.limits,
            )
        };

        // Create joint point on parent part (at origin, since joint origin is relative to parent)
        if let Some(part_id) = parent_part_id {
            let jp = crate::part::JointPoint {
                id: Uuid::new_v4(),
                name: format!("joint_to_{}", child_link_name),
                part_id,
                position: Vec3::new(
                    joint_origin.xyz[0],
                    joint_origin.xyz[1],
                    joint_origin.xyz[2],
                ),
                orientation: glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    joint_origin.rpy[0],
                    joint_origin.rpy[1],
                    joint_origin.rpy[2],
                ),
                joint_type,
                axis,
                limits,
            };
            let jp_id = jp.id;
            assembly.joint_points.insert(jp_id, jp);
            if let Some(joint) = assembly.joints.get_mut(&joint_id) {
                joint.parent_joint_point = Some(jp_id);
            }
        }

        // Create joint point on child part (at origin of child)
        if let Some(part_id) = child_part_id {
            let jp = crate::part::JointPoint {
                id: Uuid::new_v4(),
                name: format!("joint_from_{}", parent_link_name),
                part_id,
                position: Vec3::ZERO, // Child's joint point is at its own origin
                orientation: glam::Quat::IDENTITY,
                joint_type,
                axis,
                limits,
            };
            let jp_id = jp.id;
            assembly.joint_points.insert(jp_id, jp);
            if let Some(joint) = assembly.joints.get_mut(&joint_id) {
                joint.child_joint_point = Some(jp_id);
            }
        }
    }

    // Rebuild indices after all joints are added
    assembly.rebuild_indices();

    // Update world transforms
    assembly.update_world_transforms();

    // Apply link world transforms to parts
    // Final transform = link.world_transform * visual_origin
    for link in assembly.links.values() {
        if let Some(part_id) = link.part_id
            && let Some(part) = parts.iter_mut().find(|p| p.id == part_id)
        {
            // part.origin_transform already contains visual origin
            // Prepend link's world transform
            part.origin_transform = link.world_transform * part.origin_transform;
        }
    }

    // Create project
    let project = Project {
        version: 1,
        name: robot.name,
        parts,
        assembly,
        materials,
    };

    Ok(project)
}

/// Process visual geometry elements and create a Part if mesh is found
fn process_visual_geometry(
    visuals: &[urdf_rs::Visual],
    link_name: &str,
    base_dir: &Path,
    options: &ImportOptions,
    material_colors: &HashMap<String, [f32; 4]>,
    package_paths: &HashMap<String, PathBuf>,
) -> Result<(Option<Part>, Vec<VisualElement>), ImportError> {
    if visuals.is_empty() {
        return Ok((None, Vec::new()));
    }

    // Process first visual element (primary) - used to create Part
    let first_visual = &visuals[0];
    let (color, material_name) = extract_material_info(first_visual, material_colors, options);
    let origin = Pose::from(&first_visual.origin);

    // Create part from first visual's geometry
    let mut part = process_geometry(
        &first_visual.geometry,
        link_name,
        base_dir,
        options,
        package_paths,
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
        let (elem_color, elem_material) = extract_material_info(visual, material_colors, options);
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
            geometry: elem_geometry,
        });
    }

    Ok((part, visual_elements))
}

/// Extract material color and name from a visual element
fn extract_material_info(
    visual: &urdf_rs::Visual,
    material_colors: &HashMap<String, [f32; 4]>,
    options: &ImportOptions,
) -> ([f32; 4], Option<String>) {
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

        (color, name)
    } else {
        (options.default_color, None)
    }
}

/// Process a single geometry element and create a Part if applicable
fn process_geometry(
    geometry: &urdf_rs::Geometry,
    link_name: &str,
    base_dir: &Path,
    options: &ImportOptions,
    package_paths: &HashMap<String, PathBuf>,
    color: [f32; 4],
    material_name: Option<String>,
) -> Result<Option<Part>, ImportError> {
    let part = match geometry {
        urdf_rs::Geometry::Mesh { filename, scale } => {
            let mesh_path = resolve_mesh_path(filename, base_dir, package_paths)?;
            let mut part =
                load_mesh(&mesh_path, options.stl_unit).map_err(|e| ImportError::MeshLoad {
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
            let (vertices, normals, indices) = generate_box_mesh(size);
            Some(create_part_from_mesh(
                link_name,
                vertices,
                normals,
                indices,
                color,
                material_name,
            ))
        }

        urdf_rs::Geometry::Cylinder { radius, length } => {
            let (vertices, normals, indices) =
                generate_cylinder_mesh(*radius as f32, *length as f32);
            Some(create_part_from_mesh(
                link_name,
                vertices,
                normals,
                indices,
                color,
                material_name,
            ))
        }

        urdf_rs::Geometry::Sphere { radius } => {
            let (vertices, normals, indices) = generate_sphere_mesh(*radius as f32);
            Some(create_part_from_mesh(
                link_name,
                vertices,
                normals,
                indices,
                color,
                material_name,
            ))
        }

        urdf_rs::Geometry::Capsule { radius, length } => {
            // Approximate capsule as cylinder (capsule mesh generation would be more complex)
            let (vertices, normals, indices) =
                generate_cylinder_mesh(*radius as f32, *length as f32);
            Some(create_part_from_mesh(
                link_name,
                vertices,
                normals,
                indices,
                color,
                material_name,
            ))
        }
    };

    Ok(part)
}

/// Process collision geometry elements
fn process_collision_geometry(collisions: &[urdf_rs::Collision]) -> Vec<CollisionElement> {
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

/// Create a Part from mesh data
fn create_part_from_mesh(
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
fn resolve_mesh_path(
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
fn resolve_package_uri(
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
fn apply_scale(part: &mut Part, scale: [f32; 3]) {
    for vertex in &mut part.vertices {
        vertex[0] *= scale[0];
        vertex[1] *= scale[1];
        vertex[2] *= scale[2];
    }
    part.calculate_bounding_box();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pose_from() {
        let urdf_pose = urdf_rs::Pose {
            xyz: urdf_rs::Vec3([1.0, 2.0, 3.0]),
            rpy: urdf_rs::Vec3([0.1, 0.2, 0.3]),
        };

        let pose = Pose::from(&urdf_pose);
        assert_eq!(pose.xyz, [1.0, 2.0, 3.0]);
        assert_eq!(pose.rpy, [0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_joint_type_from() {
        assert!(matches!(
            JointType::from(&urdf_rs::JointType::Fixed),
            JointType::Fixed
        ));
        assert!(matches!(
            JointType::from(&urdf_rs::JointType::Revolute),
            JointType::Revolute
        ));
        assert!(matches!(
            JointType::from(&urdf_rs::JointType::Continuous),
            JointType::Continuous
        ));
    }

    #[test]
    fn test_resolve_mesh_path_package_uri() {
        let packages = HashMap::new();
        let result =
            resolve_mesh_path("package://robot/meshes/link.stl", Path::new("."), &packages);
        assert!(matches!(result, Err(ImportError::PackageNotFound { .. })));
    }

    #[test]
    fn test_resolve_mesh_path_with_package_mapping() {
        use std::fs;
        use tempfile::tempdir;

        let temp = tempdir().unwrap();
        let meshes_dir = temp.path().join("meshes");
        fs::create_dir_all(&meshes_dir).unwrap();
        let stl_path = meshes_dir.join("link.stl");
        fs::write(&stl_path, b"dummy stl content").unwrap();

        let mut packages = HashMap::new();
        packages.insert("robot".to_string(), temp.path().to_path_buf());

        let result =
            resolve_mesh_path("package://robot/meshes/link.stl", Path::new("."), &packages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), stl_path);
    }

    #[test]
    fn test_resolve_mesh_path_unsupported_format() {
        let packages = HashMap::new();
        let result = resolve_mesh_path("mesh.dae", Path::new("."), &packages);
        assert!(matches!(result, Err(ImportError::UnsupportedMeshFormat(_))));
    }

    #[test]
    fn test_create_part_from_mesh() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let indices = vec![0, 1, 2];

        let part = create_part_from_mesh(
            "test_part",
            vertices,
            normals,
            indices,
            [1.0, 0.0, 0.0, 1.0],
            Some("red".to_string()),
        );

        assert_eq!(part.name, "test_part");
        assert_eq!(part.vertices.len(), 3);
        assert_eq!(part.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(part.material_name, Some("red".to_string()));
    }
}
