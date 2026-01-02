//! URDF import functionality
//!
//! Imports URDF files and converts them to the internal Project format.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use glam::Vec3;
use uuid::Uuid;

use crate::assembly::{
    Assembly, CollisionProperties, InertialProperties, Joint, JointDynamics, Link, Pose,
    VisualProperties,
};
use crate::inertia::InertiaMatrix;
use crate::part::{JointLimits, JointType, Part};
use crate::primitive::{generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh};
use crate::project::{MaterialDef, Project};
use crate::stl::{load_stl_with_unit, StlUnit};

/// Import options for URDF loading
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Base directory for resolving relative mesh paths
    pub base_dir: PathBuf,
    /// Unit for imported STL meshes (URDF typically uses meters)
    pub stl_unit: StlUnit,
    /// Default material color if not specified
    pub default_color: [f32; 4],
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            stl_unit: StlUnit::Meters,
            default_color: [0.7, 0.7, 0.7, 1.0],
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

    #[error("package:// URIs are not supported: {0}")]
    PackageUriNotSupported(String),

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
    let base_dir = if options.base_dir == PathBuf::from(".") {
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
        let (part_opt, visual_props) = process_visual_geometry(
            &urdf_link.visual,
            &urdf_link.name,
            &base_dir,
            options,
            &material_colors,
        )?;

        // Process inertial properties
        let inertial_props = InertialProperties {
            origin: convert_pose(&urdf_link.inertial.origin),
            mass: urdf_link.inertial.mass.value as f32,
            inertia: convert_inertia(&urdf_link.inertial.inertia),
        };

        // Process collision properties
        let collision_props = urdf_link
            .collision
            .first()
            .map(|c| CollisionProperties {
                origin: convert_pose(&c.origin),
            })
            .unwrap_or_default();

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
            visual: visual_props,
            collision: collision_props,
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
    let mut assembly = Assembly {
        name: robot.name.clone(),
        root_link: root_link_id,
        links,
        joints: HashMap::new(),
        children: HashMap::new(),
        parent: HashMap::new(),
    };

    // Process joints
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
            joint_type: convert_joint_type(&urdf_joint.joint_type),
            parent_link: *parent_link_id,
            child_link: *child_link_id,
            origin: convert_pose(&urdf_joint.origin),
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
            parent_joint_point: None,
            child_joint_point: None,
        };

        let joint_id = joint.id;
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

    // Update world transforms
    assembly.update_world_transforms();

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
) -> Result<(Option<Part>, VisualProperties), ImportError> {
    // Use first visual element if available
    let visual = match visuals.first() {
        Some(v) => v,
        None => {
            return Ok((
                None,
                VisualProperties {
                    origin: Pose::default(),
                    color: options.default_color,
                    material_name: None,
                },
            ))
        }
    };

    let origin = convert_pose(&visual.origin);

    // Get color from material
    let (color, material_name) = if let Some(ref mat) = visual.material {
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
    };

    let visual_props = VisualProperties {
        origin,
        color,
        material_name: material_name.clone(),
    };

    // Process geometry
    let part = match &visual.geometry {
        urdf_rs::Geometry::Mesh { filename, scale } => {
            let mesh_path = resolve_mesh_path(filename, base_dir)?;
            let mut part =
                load_stl_with_unit(&mesh_path, options.stl_unit).map_err(|e| ImportError::MeshLoad {
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

    Ok((part, visual_props))
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
    part.inertia =
        InertiaMatrix::from_bounding_box(part.mass, part.bbox_min, part.bbox_max);

    part
}

/// Resolve mesh path from URDF filename reference
fn resolve_mesh_path(filename: &str, base_dir: &Path) -> Result<PathBuf, ImportError> {
    // Check for package:// URI (not supported)
    if filename.starts_with("package://") {
        return Err(ImportError::PackageUriNotSupported(filename.to_string()));
    }

    // Handle file:// URI
    let path_str = if let Some(stripped) = filename.strip_prefix("file://") {
        stripped
    } else {
        filename
    };

    // Check for supported format
    let lower = path_str.to_lowercase();
    if !lower.ends_with(".stl") {
        return Err(ImportError::UnsupportedMeshFormat(filename.to_string()));
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

/// Apply scale to part vertices
fn apply_scale(part: &mut Part, scale: [f32; 3]) {
    for vertex in &mut part.vertices {
        vertex[0] *= scale[0];
        vertex[1] *= scale[1];
        vertex[2] *= scale[2];
    }
    part.calculate_bounding_box();
}

/// Convert urdf_rs::Pose to internal Pose
fn convert_pose(urdf_pose: &urdf_rs::Pose) -> Pose {
    Pose {
        xyz: [
            urdf_pose.xyz.0[0] as f32,
            urdf_pose.xyz.0[1] as f32,
            urdf_pose.xyz.0[2] as f32,
        ],
        rpy: [
            urdf_pose.rpy.0[0] as f32,
            urdf_pose.rpy.0[1] as f32,
            urdf_pose.rpy.0[2] as f32,
        ],
    }
}

/// Convert urdf_rs::JointType to internal JointType
fn convert_joint_type(urdf_type: &urdf_rs::JointType) -> JointType {
    match urdf_type {
        urdf_rs::JointType::Fixed => JointType::Fixed,
        urdf_rs::JointType::Revolute => JointType::Revolute,
        urdf_rs::JointType::Continuous => JointType::Continuous,
        urdf_rs::JointType::Prismatic => JointType::Prismatic,
        urdf_rs::JointType::Floating => JointType::Floating,
        urdf_rs::JointType::Planar => JointType::Planar,
        urdf_rs::JointType::Spherical => JointType::Floating, // Approximate as floating
    }
}

/// Convert urdf_rs::Inertia to internal InertiaMatrix
fn convert_inertia(urdf_inertia: &urdf_rs::Inertia) -> InertiaMatrix {
    InertiaMatrix {
        ixx: urdf_inertia.ixx as f32,
        ixy: urdf_inertia.ixy as f32,
        ixz: urdf_inertia.ixz as f32,
        iyy: urdf_inertia.iyy as f32,
        iyz: urdf_inertia.iyz as f32,
        izz: urdf_inertia.izz as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_pose() {
        let urdf_pose = urdf_rs::Pose {
            xyz: urdf_rs::Vec3([1.0, 2.0, 3.0]),
            rpy: urdf_rs::Vec3([0.1, 0.2, 0.3]),
        };

        let pose = convert_pose(&urdf_pose);
        assert_eq!(pose.xyz, [1.0, 2.0, 3.0]);
        assert_eq!(pose.rpy, [0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_convert_joint_type() {
        assert!(matches!(
            convert_joint_type(&urdf_rs::JointType::Fixed),
            JointType::Fixed
        ));
        assert!(matches!(
            convert_joint_type(&urdf_rs::JointType::Revolute),
            JointType::Revolute
        ));
        assert!(matches!(
            convert_joint_type(&urdf_rs::JointType::Continuous),
            JointType::Continuous
        ));
    }

    #[test]
    fn test_resolve_mesh_path_package_uri() {
        let result = resolve_mesh_path("package://robot/meshes/link.stl", Path::new("."));
        assert!(matches!(result, Err(ImportError::PackageUriNotSupported(_))));
    }

    #[test]
    fn test_resolve_mesh_path_unsupported_format() {
        let result = resolve_mesh_path("mesh.dae", Path::new("."));
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
