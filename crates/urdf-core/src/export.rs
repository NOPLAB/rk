//! URDF export functionality

use std::collections::HashMap;

use crate::assembly::{Assembly, GeometryType, Joint, Link, Pose};
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

    // Handle links with or without parts
    if let Some(part_id) = link.part_id {
        let part = parts
            .get(&part_id)
            .ok_or(ExportError::PartNotFound(part_id))?;

        let mesh_uri = mesh_paths
            .get(&part_id)
            .ok_or(ExportError::MeshNotFound(part_id))?;

        // Write link with mesh
        write_link(urdf, link, Some(part), Some(mesh_uri));
    } else {
        // Write empty link (no geometry)
        write_link(urdf, link, None, None);
    }

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

fn write_link(urdf: &mut String, link: &Link, part: Option<&Part>, mesh_uri: Option<&str>) {
    urdf.push_str(&format!("  <link name=\"{}\">\n", xml_escape(&link.name)));

    // Only write full link content if we have a part/mesh
    if let (Some(part), Some(mesh_uri)) = (part, mesh_uri) {
        // Inertial
        urdf.push_str("    <inertial>\n");
        write_origin(urdf, &link.inertial.origin, 6);
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

        // Primary Visual
        write_visual_element(urdf, None, &link.visual.origin, mesh_uri,
            part.material_name.as_deref(), &part.color, None);

        // Additional Visual elements
        for elem in &link.visual.elements {
            let geom_str = elem.geometry.as_ref().map(|g| geometry_to_string(g));
            write_visual_element(
                urdf,
                elem.name.as_deref(),
                &elem.origin,
                mesh_uri,  // Use same mesh for now (could be extended)
                elem.material_name.as_deref(),
                &elem.color,
                geom_str.as_deref(),
            );
        }

        // Primary Collision
        write_collision_element(urdf, None, &link.collision.origin, mesh_uri, None);

        // Additional Collision elements
        for elem in &link.collision.elements {
            let geom_str = elem.geometry.as_ref().map(|g| geometry_to_string(g));
            write_collision_element(
                urdf,
                elem.name.as_deref(),
                &elem.origin,
                mesh_uri,  // Use same mesh for now (could be extended)
                geom_str.as_deref(),
            );
        }
    }
    // Empty links have no visual/collision/inertial

    urdf.push_str("  </link>\n\n");
}

fn write_origin(urdf: &mut String, origin: &Pose, indent: usize) {
    let indent_str = " ".repeat(indent);
    urdf.push_str(&format!(
        "{}<origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        indent_str,
        origin.xyz[0], origin.xyz[1], origin.xyz[2],
        origin.rpy[0], origin.rpy[1], origin.rpy[2]
    ));
}

fn write_visual_element(
    urdf: &mut String,
    name: Option<&str>,
    origin: &Pose,
    mesh_uri: &str,
    material_name: Option<&str>,
    color: &[f32; 4],
    geometry_override: Option<&str>,
) {
    if let Some(n) = name {
        urdf.push_str(&format!("    <visual name=\"{}\">\n", xml_escape(n)));
    } else {
        urdf.push_str("    <visual>\n");
    }

    write_origin(urdf, origin, 6);

    if let Some(geom) = geometry_override {
        urdf.push_str(&format!("      <geometry>\n        {}\n      </geometry>\n", geom));
    } else {
        urdf.push_str(&format!(
            "      <geometry>\n        <mesh filename=\"{}\"/>\n      </geometry>\n",
            xml_escape(mesh_uri)
        ));
    }

    if let Some(mat_name) = material_name {
        urdf.push_str(&format!(
            "      <material name=\"{}\"/>\n",
            xml_escape(mat_name)
        ));
    } else {
        urdf.push_str(&format!(
            "      <material name=\"\">\n        <color rgba=\"{} {} {} {}\"/>\n      </material>\n",
            color[0], color[1], color[2], color[3]
        ));
    }
    urdf.push_str("    </visual>\n");
}

fn write_collision_element(
    urdf: &mut String,
    name: Option<&str>,
    origin: &Pose,
    mesh_uri: &str,
    geometry_override: Option<&str>,
) {
    if let Some(n) = name {
        urdf.push_str(&format!("    <collision name=\"{}\">\n", xml_escape(n)));
    } else {
        urdf.push_str("    <collision>\n");
    }

    write_origin(urdf, origin, 6);

    if let Some(geom) = geometry_override {
        urdf.push_str(&format!("      <geometry>\n        {}\n      </geometry>\n", geom));
    } else {
        urdf.push_str(&format!(
            "      <geometry>\n        <mesh filename=\"{}\"/>\n      </geometry>\n",
            xml_escape(mesh_uri)
        ));
    }
    urdf.push_str("    </collision>\n");
}

fn geometry_to_string(geometry: &GeometryType) -> String {
    match geometry {
        GeometryType::Mesh => "<!-- mesh -->".to_string(),
        GeometryType::Box { size } => {
            format!("<box size=\"{} {} {}\"/>", size[0], size[1], size[2])
        }
        GeometryType::Cylinder { radius, length } => {
            format!("<cylinder radius=\"{}\" length=\"{}\"/>", radius, length)
        }
        GeometryType::Sphere { radius } => {
            format!("<sphere radius=\"{}\"/>", radius)
        }
        GeometryType::Capsule { radius, length } => {
            // Capsule is not standard URDF, approximate as cylinder
            format!("<cylinder radius=\"{}\" length=\"{}\"/>", radius, length)
        }
    }
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

    if let Some(ref mimic) = joint.mimic {
        urdf.push_str(&format!(
            "    <mimic joint=\"{}\" multiplier=\"{}\" offset=\"{}\"/>\n",
            xml_escape(&mimic.joint), mimic.multiplier, mimic.offset
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
