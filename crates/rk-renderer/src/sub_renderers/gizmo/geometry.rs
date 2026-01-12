//! Gizmo geometry generation
//!
//! This module contains the geometry generation functions for the transform gizmo.
//! It generates arrow meshes for translation and ring meshes for rotation.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::constants::gizmo::{
    self, SCALE_AXIS_LENGTH, SCALE_CUBE_SIZE, SCALE_LINE_RADIUS, colors,
};

/// Gizmo vertex data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GizmoVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub axis_id: u32,
}

/// Generate translation gizmo geometry (3 arrows along X, Y, Z axes).
///
/// Returns (vertices, indices) for indexed triangle rendering.
pub fn generate_translation_gizmo() -> (Vec<GizmoVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let arrow_length = gizmo::ARROW_LENGTH;
    let shaft_radius = gizmo::SHAFT_RADIUS;
    let head_radius = gizmo::HEAD_RADIUS;
    let head_length = gizmo::HEAD_LENGTH;
    let segments = gizmo::SEGMENTS;

    // Generate arrow for each axis
    for (axis_id, (color, direction)) in [
        (colors::X_AXIS, Vec3::X), // X = Red
        (colors::Y_AXIS, Vec3::Y), // Y = Green
        (colors::Z_AXIS, Vec3::Z), // Z = Blue
    ]
    .iter()
    .enumerate()
    {
        let base_index = vertices.len() as u32;

        // Create rotation to align arrow with axis direction
        let rotation = if *direction == Vec3::X {
            Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2)
        } else if *direction == Vec3::Z {
            Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
        } else {
            Mat4::IDENTITY
        };

        // Shaft cylinder
        let shaft_end = arrow_length - head_length;
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * shaft_radius;
            let z = angle.sin() * shaft_radius;

            // Bottom of shaft
            let pos_bottom = rotation.transform_point3(Vec3::new(x, 0.0, z));
            vertices.push(GizmoVertex {
                position: pos_bottom.into(),
                color: *color,
                axis_id: axis_id as u32,
            });

            // Top of shaft
            let pos_top = rotation.transform_point3(Vec3::new(x, shaft_end, z));
            vertices.push(GizmoVertex {
                position: pos_top.into(),
                color: *color,
                axis_id: axis_id as u32,
            });
        }

        // Shaft indices
        for i in 0..segments {
            let i0 = base_index + i * 2;
            let i1 = base_index + i * 2 + 1;
            let i2 = base_index + (i + 1) * 2;
            let i3 = base_index + (i + 1) * 2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }

        // Cone head
        let cone_base_index = vertices.len() as u32;

        // Cone tip
        let tip_pos = rotation.transform_point3(Vec3::new(0.0, arrow_length, 0.0));
        vertices.push(GizmoVertex {
            position: tip_pos.into(),
            color: *color,
            axis_id: axis_id as u32,
        });

        // Cone base vertices
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * head_radius;
            let z = angle.sin() * head_radius;
            let pos = rotation.transform_point3(Vec3::new(x, shaft_end, z));
            vertices.push(GizmoVertex {
                position: pos.into(),
                color: *color,
                axis_id: axis_id as u32,
            });
        }

        // Cone side indices
        let tip_index = cone_base_index;
        for i in 0..segments {
            let i0 = cone_base_index + 1 + i;
            let i1 = cone_base_index + 1 + (i + 1);
            indices.extend_from_slice(&[tip_index, i1, i0]);
        }

        // Cone base cap (center point + ring)
        let center_index = vertices.len() as u32;
        let center_pos = rotation.transform_point3(Vec3::new(0.0, shaft_end, 0.0));
        vertices.push(GizmoVertex {
            position: center_pos.into(),
            color: *color,
            axis_id: axis_id as u32,
        });

        for i in 0..segments {
            let i0 = cone_base_index + 1 + i;
            let i1 = cone_base_index + 1 + (i + 1);
            indices.extend_from_slice(&[center_index, i0, i1]);
        }
    }

    (vertices, indices)
}

/// Generate rotation gizmo geometry (3 rings for X, Y, Z axes).
///
/// Each ring is a torus centered at origin, perpendicular to its axis.
/// Returns (vertices, indices) for indexed triangle rendering.
pub fn generate_rotation_gizmo() -> (Vec<GizmoVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let ring_radius = gizmo::RING_RADIUS;
    let tube_radius = gizmo::RING_THICKNESS;
    let ring_segments = gizmo::RING_SEGMENTS;
    let tube_segments = gizmo::RING_TUBE_SEGMENTS;

    // Generate ring for each axis
    for (axis_id, (color, rotation)) in [
        // X-axis: ring in YZ plane (rotate around X)
        (colors::X_AXIS, Mat4::from_rotation_z(-FRAC_PI_2)),
        // Y-axis: ring in XZ plane (no rotation needed)
        (colors::Y_AXIS, Mat4::IDENTITY),
        // Z-axis: ring in XY plane (rotate around Z then X)
        (colors::Z_AXIS, Mat4::from_rotation_x(FRAC_PI_2)),
    ]
    .iter()
    .enumerate()
    {
        let base_index = vertices.len() as u32;

        // Generate torus vertices
        for i in 0..=ring_segments {
            let ring_angle = (i as f32 / ring_segments as f32) * TAU;
            let ring_cos = ring_angle.cos();
            let ring_sin = ring_angle.sin();

            // Center of tube at this ring position
            let tube_center = Vec3::new(ring_cos * ring_radius, 0.0, ring_sin * ring_radius);

            for j in 0..=tube_segments {
                let tube_angle = (j as f32 / tube_segments as f32) * TAU;
                let tube_cos = tube_angle.cos();
                let tube_sin = tube_angle.sin();

                // Position on the tube surface
                let local_pos = Vec3::new(
                    tube_center.x + ring_cos * tube_cos * tube_radius,
                    tube_sin * tube_radius,
                    tube_center.z + ring_sin * tube_cos * tube_radius,
                );

                let pos = rotation.transform_point3(local_pos);
                vertices.push(GizmoVertex {
                    position: pos.into(),
                    color: *color,
                    axis_id: axis_id as u32,
                });
            }
        }

        // Generate torus indices
        let tube_verts = tube_segments + 1;
        for i in 0..ring_segments {
            for j in 0..tube_segments {
                let i0 = base_index + i * tube_verts + j;
                let i1 = base_index + i * tube_verts + (j + 1);
                let i2 = base_index + (i + 1) * tube_verts + j;
                let i3 = base_index + (i + 1) * tube_verts + (j + 1);

                indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }
    }

    (vertices, indices)
}

/// Generate scale gizmo geometry (3 axes with cubes at the end).
///
/// Each axis has a thin line from origin to a cube handle.
/// Returns (vertices, indices) for indexed triangle rendering.
pub fn generate_scale_gizmo() -> (Vec<GizmoVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let axis_length = SCALE_AXIS_LENGTH;
    let line_radius = SCALE_LINE_RADIUS;
    let cube_size = SCALE_CUBE_SIZE;
    let segments = gizmo::SEGMENTS;

    // Generate for each axis
    for (axis_id, (color, direction)) in [
        (colors::X_AXIS, Vec3::X),
        (colors::Y_AXIS, Vec3::Y),
        (colors::Z_AXIS, Vec3::Z),
    ]
    .iter()
    .enumerate()
    {
        let base_index = vertices.len() as u32;

        // Create rotation to align with axis direction
        let rotation = if *direction == Vec3::X {
            Mat4::from_rotation_z(-FRAC_PI_2)
        } else if *direction == Vec3::Z {
            Mat4::from_rotation_x(FRAC_PI_2)
        } else {
            Mat4::IDENTITY
        };

        // Line (cylinder) from origin to cube
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * TAU;
            let x = angle.cos() * line_radius;
            let z = angle.sin() * line_radius;

            // Bottom of line
            let pos_bottom = rotation.transform_point3(Vec3::new(x, 0.0, z));
            vertices.push(GizmoVertex {
                position: pos_bottom.into(),
                color: *color,
                axis_id: axis_id as u32,
            });

            // Top of line (just before cube)
            let pos_top = rotation.transform_point3(Vec3::new(x, axis_length - cube_size, z));
            vertices.push(GizmoVertex {
                position: pos_top.into(),
                color: *color,
                axis_id: axis_id as u32,
            });
        }

        // Line indices
        for i in 0..segments {
            let i0 = base_index + i * 2;
            let i1 = base_index + i * 2 + 1;
            let i2 = base_index + (i + 1) * 2;
            let i3 = base_index + (i + 1) * 2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }

        // Cube at the end
        let cube_center = rotation.transform_point3(Vec3::new(0.0, axis_length, 0.0));
        let cube_base_index = vertices.len() as u32;

        // Cube vertices (8 corners)
        let cube_offsets = [
            Vec3::new(-cube_size, -cube_size, -cube_size),
            Vec3::new(cube_size, -cube_size, -cube_size),
            Vec3::new(cube_size, cube_size, -cube_size),
            Vec3::new(-cube_size, cube_size, -cube_size),
            Vec3::new(-cube_size, -cube_size, cube_size),
            Vec3::new(cube_size, -cube_size, cube_size),
            Vec3::new(cube_size, cube_size, cube_size),
            Vec3::new(-cube_size, cube_size, cube_size),
        ];

        for offset in &cube_offsets {
            vertices.push(GizmoVertex {
                position: (cube_center + *offset).into(),
                color: *color,
                axis_id: axis_id as u32,
            });
        }

        // Cube faces (6 faces, 2 triangles each)
        #[rustfmt::skip]
        let cube_indices: [u32; 36] = [
            // Front face
            0, 1, 2, 0, 2, 3,
            // Back face
            4, 6, 5, 4, 7, 6,
            // Top face
            3, 2, 6, 3, 6, 7,
            // Bottom face
            0, 5, 1, 0, 4, 5,
            // Right face
            1, 5, 6, 1, 6, 2,
            // Left face
            0, 3, 7, 0, 7, 4,
        ];

        for idx in &cube_indices {
            indices.push(cube_base_index + *idx);
        }
    }

    (vertices, indices)
}
