//! URDF export functionality

use std::collections::HashMap;

use crate::assembly::{Assembly, Joint, Link};
use crate::part::{JointType, Part};
use crate::stl::save_stl;

/// Export options for URDF generation
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Output directory
    pub output_dir: std::path::PathBuf,
    /// Robot name (for URDF root element)
    pub robot_name: String,
    /// Mesh package prefix (e.g., "package://robot_description")
    pub mesh_prefix: String,
    /// Whether to use package:// URIs or relative paths
    pub use_package_uri: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            output_dir: std::path::PathBuf::from("."),
            robot_name: "robot".to_string(),
            mesh_prefix: "meshes".to_string(),
            use_package_uri: false,
        }
    }
}

/// Export assembly to URDF
pub fn export_urdf(
    assembly: &Assembly,
    parts: &HashMap<uuid::Uuid, Part>,
    options: &ExportOptions,
) -> Result<String, ExportError> {
    // Validate assembly
    assembly
        .validate()
        .map_err(|errors| ExportError::Validation(format!("{:?}", errors)))?;

    let root_id = assembly
        .root_link
        .ok_or(ExportError::NoRootLink)?;

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

    // Build URDF string
    let mut urdf = String::new();
    urdf.push_str(&format!(
        "<?xml version=\"1.0\"?>\n<robot name=\"{}\">\n\n",
        xml_escape(&options.robot_name)
    ));

    // Collect unique materials
    let mut materials: HashMap<String, [f32; 4]> = HashMap::new();
    for part in parts.values() {
        if let Some(ref name) = part.material_name {
            materials.insert(name.clone(), part.color);
        }
    }

    // Write material definitions
    for (name, color) in &materials {
        urdf.push_str(&format!(
            "  <material name=\"{}\">\n    <color rgba=\"{} {} {} {}\"/>\n  </material>\n\n",
            xml_escape(name),
            color[0],
            color[1],
            color[2],
            color[3]
        ));
    }

    // Write links and joints recursively
    write_link_recursive(
        &mut urdf,
        assembly,
        parts,
        &mesh_paths,
        root_id,
        &mut std::collections::HashSet::new(),
    )?;

    urdf.push_str("</robot>\n");

    // Write URDF file
    let urdf_path = options.output_dir.join(format!("{}.urdf", options.robot_name));
    std::fs::write(&urdf_path, &urdf).map_err(|e| ExportError::Io(e.to_string()))?;

    Ok(urdf)
}

fn write_link_recursive(
    urdf: &mut String,
    assembly: &Assembly,
    parts: &HashMap<uuid::Uuid, Part>,
    mesh_paths: &HashMap<uuid::Uuid, String>,
    link_id: uuid::Uuid,
    visited: &mut std::collections::HashSet<uuid::Uuid>,
) -> Result<(), ExportError> {
    if !visited.insert(link_id) {
        return Ok(()); // Already visited
    }

    let link = assembly
        .links
        .get(&link_id)
        .ok_or(ExportError::LinkNotFound(link_id))?;

    let part = parts
        .get(&link.part_id)
        .ok_or(ExportError::PartNotFound(link.part_id))?;

    let mesh_uri = mesh_paths
        .get(&link.part_id)
        .ok_or(ExportError::MeshNotFound(link.part_id))?;

    // Write link
    write_link(urdf, link, part, mesh_uri);

    // Write joints and children
    if let Some(children) = assembly.children.get(&link_id) {
        for (joint_id, child_id) in children {
            if let Some(joint) = assembly.joints.get(joint_id) {
                let child_link = assembly
                    .links
                    .get(child_id)
                    .ok_or(ExportError::LinkNotFound(*child_id))?;

                write_joint(urdf, joint, &link.name, &child_link.name);
                write_link_recursive(urdf, assembly, parts, mesh_paths, *child_id, visited)?;
            }
        }
    }

    Ok(())
}

fn write_link(urdf: &mut String, link: &Link, part: &Part, mesh_uri: &str) {
    urdf.push_str(&format!("  <link name=\"{}\">\n", xml_escape(&link.name)));

    // Inertial
    urdf.push_str("    <inertial>\n");
    urdf.push_str(&format!(
        "      <origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        link.inertial.origin.xyz[0],
        link.inertial.origin.xyz[1],
        link.inertial.origin.xyz[2],
        link.inertial.origin.rpy[0],
        link.inertial.origin.rpy[1],
        link.inertial.origin.rpy[2]
    ));
    urdf.push_str(&format!(
        "      <mass value=\"{}\"/>\n",
        link.inertial.mass
    ));
    let inertia = &link.inertial.inertia;
    urdf.push_str(&format!(
        "      <inertia ixx=\"{}\" ixy=\"{}\" ixz=\"{}\" iyy=\"{}\" iyz=\"{}\" izz=\"{}\"/>\n",
        inertia.ixx, inertia.ixy, inertia.ixz, inertia.iyy, inertia.iyz, inertia.izz
    ));
    urdf.push_str("    </inertial>\n");

    // Visual
    urdf.push_str("    <visual>\n");
    urdf.push_str(&format!(
        "      <origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        link.visual.origin.xyz[0],
        link.visual.origin.xyz[1],
        link.visual.origin.xyz[2],
        link.visual.origin.rpy[0],
        link.visual.origin.rpy[1],
        link.visual.origin.rpy[2]
    ));
    urdf.push_str(&format!(
        "      <geometry>\n        <mesh filename=\"{}\"/>\n      </geometry>\n",
        xml_escape(mesh_uri)
    ));
    if let Some(ref material_name) = part.material_name {
        urdf.push_str(&format!(
            "      <material name=\"{}\"/>\n",
            xml_escape(material_name)
        ));
    } else {
        urdf.push_str(&format!(
            "      <material name=\"\">\n        <color rgba=\"{} {} {} {}\"/>\n      </material>\n",
            part.color[0], part.color[1], part.color[2], part.color[3]
        ));
    }
    urdf.push_str("    </visual>\n");

    // Collision
    urdf.push_str("    <collision>\n");
    urdf.push_str(&format!(
        "      <origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        link.collision.origin.xyz[0],
        link.collision.origin.xyz[1],
        link.collision.origin.xyz[2],
        link.collision.origin.rpy[0],
        link.collision.origin.rpy[1],
        link.collision.origin.rpy[2]
    ));
    urdf.push_str(&format!(
        "      <geometry>\n        <mesh filename=\"{}\"/>\n      </geometry>\n",
        xml_escape(mesh_uri)
    ));
    urdf.push_str("    </collision>\n");

    urdf.push_str("  </link>\n\n");
}

fn write_joint(urdf: &mut String, joint: &Joint, parent_name: &str, child_name: &str) {
    let type_str = match joint.joint_type {
        JointType::Fixed => "fixed",
        JointType::Revolute => "revolute",
        JointType::Continuous => "continuous",
        JointType::Prismatic => "prismatic",
        JointType::Floating => "floating",
        JointType::Planar => "planar",
    };

    urdf.push_str(&format!(
        "  <joint name=\"{}\" type=\"{}\">\n",
        xml_escape(&joint.name),
        type_str
    ));
    urdf.push_str(&format!(
        "    <parent link=\"{}\"/>\n",
        xml_escape(parent_name)
    ));
    urdf.push_str(&format!(
        "    <child link=\"{}\"/>\n",
        xml_escape(child_name)
    ));
    urdf.push_str(&format!(
        "    <origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        joint.origin.xyz[0],
        joint.origin.xyz[1],
        joint.origin.xyz[2],
        joint.origin.rpy[0],
        joint.origin.rpy[1],
        joint.origin.rpy[2]
    ));

    if joint.joint_type.has_axis() {
        urdf.push_str(&format!(
            "    <axis xyz=\"{} {} {}\"/>\n",
            joint.axis.x, joint.axis.y, joint.axis.z
        ));
    }

    if let Some(ref limits) = joint.limits {
        urdf.push_str(&format!(
            "    <limit lower=\"{}\" upper=\"{}\" effort=\"{}\" velocity=\"{}\"/>\n",
            limits.lower, limits.upper, limits.effort, limits.velocity
        ));
    }

    if let Some(ref dynamics) = joint.dynamics {
        urdf.push_str(&format!(
            "    <dynamics damping=\"{}\" friction=\"{}\"/>\n",
            dynamics.damping, dynamics.friction
        ));
    }

    urdf.push_str("  </joint>\n\n");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
    LinkNotFound(uuid::Uuid),
    #[error("Part not found: {0}")]
    PartNotFound(uuid::Uuid),
    #[error("Mesh path not found for part: {0}")]
    MeshNotFound(uuid::Uuid),
}
