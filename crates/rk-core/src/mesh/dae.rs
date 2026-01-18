//! DAE (COLLADA) mesh file loading

use std::path::Path;

use dae_parser::{Document, Geometry, Primitive, Semantic, Source};

use crate::part::Part;

use super::MeshError;
use super::normals::{calculate_face_normals, calculate_triangle_normal};
use super::stl::StlUnit;

/// Load a DAE (COLLADA) file and create a Part
pub fn load_dae(path: impl AsRef<Path>) -> Result<Part, MeshError> {
    load_dae_with_unit(path, StlUnit::Meters)
}

/// Load a DAE (COLLADA) file with specified unit
pub fn load_dae_with_unit(path: impl AsRef<Path>, unit: StlUnit) -> Result<Part, MeshError> {
    let path = path.as_ref();

    let document = Document::from_file(path)
        .map_err(|e| MeshError::Parse(format!("DAE parse error: {:?}", e)))?;

    let scale = unit.scale_factor();

    let mut all_vertices: Vec<[f32; 3]> = Vec::new();
    let mut all_normals: Vec<[f32; 3]> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    // Get the local map to access geometry elements
    let geom_map = document
        .local_map::<Geometry>()
        .map_err(|_| MeshError::EmptyMesh)?;

    let source_map = document
        .local_map::<Source>()
        .map_err(|_| MeshError::EmptyMesh)?;

    // Process each geometry
    for geometry in geom_map.0.values() {
        let mesh = match &geometry.element {
            dae_parser::GeometryElement::Mesh(m) => m,
            _ => continue,
        };

        // Find position source from vertices
        let vertices = mesh.vertices.as_ref().ok_or(MeshError::EmptyMesh)?;

        let position_input = vertices
            .inputs
            .iter()
            .find(|i| i.semantic == Semantic::Position)
            .ok_or(MeshError::EmptyMesh)?;

        // Get position source array
        let position_source = source_map
            .get(position_input.source_as_source())
            .ok_or_else(|| MeshError::Parse("Position source not found".to_string()))?;

        let positions: Vec<[f32; 3]> = extract_vec3_from_source(position_source, scale)?;

        // Find normal source if exists
        let normal_input = vertices
            .inputs
            .iter()
            .find(|i| i.semantic == Semantic::Normal);
        let normals_from_vertices: Option<Vec<[f32; 3]>> = if let Some(ni) = normal_input {
            source_map
                .get(ni.source_as_source())
                .and_then(|s| extract_vec3_from_source(s, 1.0).ok())
        } else {
            None
        };

        // Process primitives (triangles, polylist, etc.)
        for primitive in &mesh.elements {
            let vertex_offset = all_vertices.len() as u32;

            match primitive {
                Primitive::Triangles(tris) => {
                    // Get the stride and vertex offset from inputs
                    let stride = tris.inputs.stride;
                    let vtx_off = tris
                        .inputs
                        .iter()
                        .find(|i| i.semantic == Semantic::Vertex)
                        .map(|i| i.offset as usize)
                        .unwrap_or(0);

                    // Get normal offset if exists
                    let normal_off = tris
                        .inputs
                        .iter()
                        .find(|i| i.semantic == Semantic::Normal)
                        .map(|i| i.offset as usize);

                    let normal_source = if normal_off.is_some() {
                        tris.inputs
                            .iter()
                            .find(|i| i.semantic == Semantic::Normal)
                            .and_then(|i| source_map.get(i.source_as_source()))
                            .and_then(|s| extract_vec3_from_source(s, 1.0).ok())
                    } else {
                        None
                    };

                    if let Some(ref prim_data) = tris.data.prim {
                        process_triangles(
                            prim_data,
                            stride,
                            vtx_off,
                            normal_off,
                            &positions,
                            normal_source.as_ref().or(normals_from_vertices.as_ref()),
                            &mut all_vertices,
                            &mut all_normals,
                            &mut all_indices,
                            vertex_offset,
                        );
                    }
                }
                Primitive::PolyList(polylist) => {
                    let stride = polylist.inputs.stride;
                    let vtx_off = polylist
                        .inputs
                        .iter()
                        .find(|i| i.semantic == Semantic::Vertex)
                        .map(|i| i.offset as usize)
                        .unwrap_or(0);

                    let normal_off = polylist
                        .inputs
                        .iter()
                        .find(|i| i.semantic == Semantic::Normal)
                        .map(|i| i.offset as usize);

                    let normal_source = if normal_off.is_some() {
                        polylist
                            .inputs
                            .iter()
                            .find(|i| i.semantic == Semantic::Normal)
                            .and_then(|i| source_map.get(i.source_as_source()))
                            .and_then(|s| extract_vec3_from_source(s, 1.0).ok())
                    } else {
                        None
                    };

                    process_polylist(
                        &polylist.data.prim,
                        &polylist.data.vcount,
                        stride,
                        vtx_off,
                        normal_off,
                        &positions,
                        normal_source.as_ref().or(normals_from_vertices.as_ref()),
                        &mut all_vertices,
                        &mut all_normals,
                        &mut all_indices,
                        vertex_offset,
                    );
                }
                _ => {
                    // Skip other primitive types (lines, etc.)
                }
            }
        }
    }

    if all_vertices.is_empty() {
        return Err(MeshError::EmptyMesh);
    }

    // Calculate face normals if not provided
    if all_normals.is_empty() {
        all_normals = calculate_face_normals(&all_vertices, &all_indices);
    }

    let (name, mesh_path) = super::extract_name_and_path(path);
    let mut part = Part::new(name);
    super::finalize_part(
        &mut part,
        mesh_path,
        super::RawMeshData {
            vertices: all_vertices,
            normals: all_normals,
            indices: all_indices,
        },
    );

    Ok(part)
}

/// Extract Vec3 data from a COLLADA source
fn extract_vec3_from_source(source: &Source, scale: f32) -> Result<Vec<[f32; 3]>, MeshError> {
    let accessor = &source.accessor;

    // Get float array from the array element
    let float_array = match &source.array {
        Some(dae_parser::ArrayElement::Float(arr)) => arr,
        _ => return Err(MeshError::Parse("No float array in source".to_string())),
    };

    let stride = if accessor.stride > 0 {
        accessor.stride
    } else {
        3
    };
    let count = accessor.count;

    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * stride;
        if base + 2 < float_array.len() {
            result.push([
                float_array[base] * scale,
                float_array[base + 1] * scale,
                float_array[base + 2] * scale,
            ]);
        }
    }
    Ok(result)
}

/// Process triangle primitives
#[allow(clippy::too_many_arguments)]
fn process_triangles(
    prim_data: &[u32],
    stride: usize,
    vtx_off: usize,
    normal_off: Option<usize>,
    positions: &[[f32; 3]],
    normals: Option<&Vec<[f32; 3]>>,
    all_vertices: &mut Vec<[f32; 3]>,
    all_normals: &mut Vec<[f32; 3]>,
    all_indices: &mut Vec<u32>,
    vertex_offset: u32,
) {
    let triangle_count = prim_data.len() / (stride * 3);

    for tri_idx in 0..triangle_count {
        let base = tri_idx * stride * 3;

        for vert in 0..3 {
            let idx_base = base + vert * stride;
            let pos_idx = prim_data[idx_base + vtx_off] as usize;

            if pos_idx < positions.len() {
                all_vertices.push(positions[pos_idx]);
                all_indices.push(vertex_offset + (tri_idx * 3 + vert) as u32);

                // Handle normals
                if let (Some(n_off), Some(norms)) = (normal_off, normals) {
                    let norm_idx = prim_data[idx_base + n_off] as usize;
                    if norm_idx < norms.len() {
                        all_normals.push(norms[norm_idx]);
                    }
                }
            }
        }

        // Add face normal if per-vertex normals weren't added
        if (normal_off.is_none() || normals.is_none()) && all_normals.len() < all_vertices.len() {
            // Calculate face normal for this triangle
            let v_base = all_vertices.len() - 3;
            if v_base + 2 < all_vertices.len() {
                let v0 = all_vertices[v_base];
                let v1 = all_vertices[v_base + 1];
                let v2 = all_vertices[v_base + 2];
                let normal = calculate_triangle_normal(v0, v1, v2);
                all_normals.push(normal);
            }
        }
    }
}

/// Process polylist primitives
#[allow(clippy::too_many_arguments)]
fn process_polylist(
    prim_data: &[u32],
    vcount: &[u32],
    stride: usize,
    vtx_off: usize,
    normal_off: Option<usize>,
    positions: &[[f32; 3]],
    normals: Option<&Vec<[f32; 3]>>,
    all_vertices: &mut Vec<[f32; 3]>,
    all_normals: &mut Vec<[f32; 3]>,
    all_indices: &mut Vec<u32>,
    vertex_offset: u32,
) {
    let mut prim_offset = 0;

    for &vc in vcount {
        let vert_count = vc as usize;

        // Triangulate polygon (fan triangulation)
        if vert_count >= 3 {
            for i in 1..vert_count - 1 {
                let indices = [0, i, i + 1];

                for &vi in &indices {
                    let idx_base = prim_offset + vi * stride;
                    let pos_idx = prim_data[idx_base + vtx_off] as usize;

                    if pos_idx < positions.len() {
                        all_vertices.push(positions[pos_idx]);
                        all_indices.push(vertex_offset + all_vertices.len() as u32 - 1);

                        if let (Some(n_off), Some(norms)) = (normal_off, normals) {
                            let norm_idx = prim_data[idx_base + n_off] as usize;
                            if norm_idx < norms.len() {
                                all_normals.push(norms[norm_idx]);
                            }
                        }
                    }
                }

                // Add face normal if per-vertex normals weren't added
                if (normal_off.is_none() || normals.is_none())
                    && all_normals.len() < all_vertices.len()
                {
                    let v_base = all_vertices.len() - 3;
                    if v_base + 2 < all_vertices.len() {
                        let v0 = all_vertices[v_base];
                        let v1 = all_vertices[v_base + 1];
                        let v2 = all_vertices[v_base + 2];
                        let normal = calculate_triangle_normal(v0, v1, v2);
                        all_normals.push(normal);
                    }
                }
            }
        }

        prim_offset += vert_count * stride;
    }
}
