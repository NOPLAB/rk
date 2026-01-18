//! URDF import functionality
//!
//! Imports URDF files and converts them to the internal Project format.

mod geometry;
mod options;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec3;
use uuid::Uuid;

use crate::assembly::{Assembly, InertialProperties, Joint, JointDynamics, JointMimic, Link, Pose};
use crate::inertia::InertiaMatrix;
use crate::part::Part;
use crate::project::{MaterialDef, Project};
use crate::types::{JointLimits, JointType};

pub use geometry::{
    GeometryContext, create_part_from_mesh, process_collision_geometry, process_geometry,
    process_visual_geometry, resolve_mesh_path,
};
pub use options::ImportOptions;

/// Result of processing URDF links: (parts, links, link_name_to_id mapping)
type ProcessedLinks = (
    HashMap<Uuid, Part>,
    HashMap<Uuid, Link>,
    HashMap<String, Uuid>,
);

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
    let robot = urdf_rs::read_file(urdf_path).map_err(|e| ImportError::UrdfParse(e.to_string()))?;

    if robot.links.is_empty() {
        return Err(ImportError::EmptyUrdf);
    }

    let base_dir = resolve_base_dir(urdf_path, options);
    let (materials, material_colors) = collect_materials(&robot.materials, options);

    let ctx = GeometryContext {
        base_dir: &base_dir,
        options,
        material_colors: &material_colors,
        package_paths: &options.package_paths,
    };

    let (mut parts, links, link_name_to_id) = process_urdf_links(&robot.links, &ctx)?;

    let mut assembly = Assembly::new(&robot.name);
    assembly.links = links;
    assembly.rebuild_indices();

    process_urdf_joints(&robot.joints, &link_name_to_id, &mut assembly)?;

    assembly.rebuild_indices();
    assembly.update_world_transforms();

    apply_world_transforms_to_parts(&assembly, &mut parts);

    Ok(Project::with_parts(robot.name, parts, assembly, materials))
}

/// Resolve the base directory for mesh path resolution
fn resolve_base_dir(urdf_path: &Path, options: &ImportOptions) -> PathBuf {
    if options.base_dir.as_os_str() == "." {
        urdf_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        options.base_dir.clone()
    }
}

/// Collect materials from URDF and create lookup map
fn collect_materials(
    urdf_materials: &[urdf_rs::Material],
    options: &ImportOptions,
) -> (Vec<MaterialDef>, HashMap<String, [f32; 4]>) {
    let materials: Vec<MaterialDef> = urdf_materials
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

    let material_colors: HashMap<String, [f32; 4]> = materials
        .iter()
        .map(|m| (m.name.clone(), m.color))
        .collect();

    (materials, material_colors)
}

/// Process URDF links and create Parts and Links
fn process_urdf_links(
    urdf_links: &[urdf_rs::Link],
    ctx: &GeometryContext,
) -> Result<ProcessedLinks, ImportError> {
    let mut parts: HashMap<Uuid, Part> = HashMap::new();
    let mut links: HashMap<Uuid, Link> = HashMap::new();
    let mut link_name_to_id: HashMap<String, Uuid> = HashMap::new();

    for urdf_link in urdf_links {
        let link_id = Uuid::new_v4();
        link_name_to_id.insert(urdf_link.name.clone(), link_id);

        let (part_opt, visuals) = process_visual_geometry(&urdf_link.visual, &urdf_link.name, ctx)?;

        let inertial_props = InertialProperties {
            origin: Pose::from(&urdf_link.inertial.origin),
            mass: urdf_link.inertial.mass.value as f32,
            inertia: InertiaMatrix::from(&urdf_link.inertial.inertia),
        };

        let collisions = process_collision_geometry(&urdf_link.collision);

        let part_id = if let Some(mut part) = part_opt {
            part.mass = inertial_props.mass;
            part.inertia = inertial_props.inertia;
            let id = part.id;
            parts.insert(id, part);
            Some(id)
        } else {
            None
        };

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

    Ok((parts, links, link_name_to_id))
}

/// Process URDF joints and add them to the assembly
fn process_urdf_joints(
    urdf_joints: &[urdf_rs::Joint],
    link_name_to_id: &HashMap<String, Uuid>,
    assembly: &mut Assembly,
) -> Result<(), ImportError> {
    let mut joint_name_to_id: HashMap<String, Uuid> = HashMap::new();

    // First pass: create joints without mimic
    for urdf_joint in urdf_joints {
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
            mimic: None,
        };

        let joint_id = joint.id;
        joint_name_to_id.insert(urdf_joint.name.clone(), joint_id);
        assembly.joints.insert(joint_id, joint);

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
    for urdf_joint in urdf_joints {
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

    Ok(())
}

/// Apply link world transforms to parts
fn apply_world_transforms_to_parts(assembly: &Assembly, parts: &mut HashMap<Uuid, Part>) {
    for link in assembly.links.values() {
        if let Some(part_id) = link.part_id
            && let Some(part) = parts.get_mut(&part_id)
        {
            part.origin_transform = link.world_transform * part.origin_transform;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::Pose;
    use crate::types::JointType;

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
        let result = resolve_mesh_path(
            "package://robot/meshes/link.stl",
            std::path::Path::new("."),
            &packages,
        );
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

        let result = resolve_mesh_path(
            "package://robot/meshes/link.stl",
            std::path::Path::new("."),
            &packages,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), stl_path);
    }

    #[test]
    fn test_resolve_mesh_path_unsupported_format() {
        let packages = HashMap::new();
        // Use a truly unsupported format (DAE is now supported)
        let result = resolve_mesh_path("mesh.xyz", std::path::Path::new("."), &packages);
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
