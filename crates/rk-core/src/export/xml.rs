//! XML generation utilities for URDF export

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::assembly::{Assembly, Joint, Link, Pose};
use crate::part::Part;
use crate::types::JointType;

use super::ExportError;

/// Internal function to generate URDF XML string
pub fn generate_urdf_string(
    assembly: &Assembly,
    parts: &HashMap<Uuid, Part>,
    mesh_paths: &HashMap<Uuid, String>,
    robot_name: &str,
) -> Result<String, ExportError> {
    let roots = assembly.get_root_links();
    let root_id = roots.first().copied().ok_or(ExportError::NoRootLink)?;
    if roots.len() > 1 {
        eprintln!(
            "Warning: Multiple root links found ({}), using first one for URDF export",
            roots.len()
        );
    }

    // Build URDF string
    let mut urdf = String::new();
    urdf.push_str(&format!(
        "<?xml version=\"1.0\"?>\n<robot name=\"{}\">\n\n",
        xml_escape(robot_name)
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
        mesh_paths,
        root_id,
        &mut HashSet::new(),
    )?;

    urdf.push_str("</robot>\n");

    Ok(urdf)
}

pub fn write_link_recursive(
    urdf: &mut String,
    assembly: &Assembly,
    parts: &HashMap<Uuid, Part>,
    mesh_paths: &HashMap<Uuid, String>,
    link_id: Uuid,
    visited: &mut HashSet<Uuid>,
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

                write_joint(urdf, joint, &link.name, &child_link.name, assembly);
                write_link_recursive(urdf, assembly, parts, mesh_paths, *child_id, visited)?;
            }
        }
    }

    Ok(())
}

pub fn write_link(urdf: &mut String, link: &Link, part: Option<&Part>, mesh_uri: Option<&str>) {
    urdf.push_str(&format!("  <link name=\"{}\">\n", xml_escape(&link.name)));

    // Only write full link content if we have a part/mesh
    if let Some(_part) = part {
        // Inertial
        urdf.push_str("    <inertial>\n");
        write_origin(urdf, &link.inertial.origin, 6);
        urdf.push_str(&format!("      <mass value=\"{}\"/>\n", link.inertial.mass));
        let inertia = &link.inertial.inertia;
        urdf.push_str(&format!(
            "      <inertia ixx=\"{}\" ixy=\"{}\" ixz=\"{}\" iyy=\"{}\" iyz=\"{}\" izz=\"{}\"/>\n",
            inertia.ixx, inertia.ixy, inertia.ixz, inertia.iyy, inertia.iyz, inertia.izz
        ));
        urdf.push_str("    </inertial>\n");

        // Visual elements
        for elem in &link.visuals {
            let geom_str = elem.geometry.to_urdf_xml(mesh_uri);
            write_visual_element(
                urdf,
                elem.name.as_deref(),
                &elem.origin,
                elem.material_name.as_deref(),
                &elem.color,
                elem.texture.as_deref(),
                &geom_str,
            );
        }

        // Collision elements
        for elem in &link.collisions {
            let geom_str = elem.geometry.to_urdf_xml(mesh_uri);
            write_collision_element(urdf, elem.name.as_deref(), &elem.origin, &geom_str);
        }
    }
    // Empty links have no visual/collision/inertial

    urdf.push_str("  </link>\n\n");
}

pub fn write_origin(urdf: &mut String, origin: &Pose, indent: usize) {
    let indent_str = " ".repeat(indent);
    urdf.push_str(&format!(
        "{}<origin xyz=\"{} {} {}\" rpy=\"{} {} {}\"/>\n",
        indent_str,
        origin.xyz[0],
        origin.xyz[1],
        origin.xyz[2],
        origin.rpy[0],
        origin.rpy[1],
        origin.rpy[2]
    ));
}

pub fn write_visual_element(
    urdf: &mut String,
    name: Option<&str>,
    origin: &Pose,
    material_name: Option<&str>,
    color: &[f32; 4],
    texture: Option<&str>,
    geometry_xml: &str,
) {
    if let Some(n) = name {
        urdf.push_str(&format!("    <visual name=\"{}\">\n", xml_escape(n)));
    } else {
        urdf.push_str("    <visual>\n");
    }

    write_origin(urdf, origin, 6);
    urdf.push_str(&format!(
        "      <geometry>\n        {}\n      </geometry>\n",
        geometry_xml
    ));

    if let Some(mat_name) = material_name {
        urdf.push_str(&format!(
            "      <material name=\"{}\"/>\n",
            xml_escape(mat_name)
        ));
    } else {
        urdf.push_str("      <material name=\"\">\n");
        urdf.push_str(&format!(
            "        <color rgba=\"{} {} {} {}\"/>\n",
            color[0], color[1], color[2], color[3]
        ));
        if let Some(tex) = texture {
            urdf.push_str(&format!(
                "        <texture filename=\"{}\"/>\n",
                xml_escape(tex)
            ));
        }
        urdf.push_str("      </material>\n");
    }
    urdf.push_str("    </visual>\n");
}

pub fn write_collision_element(
    urdf: &mut String,
    name: Option<&str>,
    origin: &Pose,
    geometry_xml: &str,
) {
    if let Some(n) = name {
        urdf.push_str(&format!("    <collision name=\"{}\">\n", xml_escape(n)));
    } else {
        urdf.push_str("    <collision>\n");
    }

    write_origin(urdf, origin, 6);
    urdf.push_str(&format!(
        "      <geometry>\n        {}\n      </geometry>\n",
        geometry_xml
    ));
    urdf.push_str("    </collision>\n");
}

pub fn write_joint(
    urdf: &mut String,
    joint: &Joint,
    parent_name: &str,
    child_name: &str,
    assembly: &Assembly,
) {
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
        // Resolve joint ID to name for URDF export
        if let Some(mimic_joint) = assembly.joints.get(&mimic.joint_id) {
            urdf.push_str(&format!(
                "    <mimic joint=\"{}\" multiplier=\"{}\" offset=\"{}\"/>\n",
                xml_escape(&mimic_joint.name),
                mimic.multiplier,
                mimic.offset
            ));
        }
    }

    urdf.push_str("  </joint>\n\n");
}

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn sanitize_filename(name: &str) -> String {
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
