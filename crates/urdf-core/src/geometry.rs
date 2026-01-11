//! Geometry type definitions and conversions

use serde::{Deserialize, Serialize};

/// Geometry type for visual/collision elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeometryType {
    /// Mesh geometry with optional path
    Mesh {
        path: Option<String>,
    },
    Box {
        size: [f32; 3],
    },
    Cylinder {
        radius: f32,
        length: f32,
    },
    Sphere {
        radius: f32,
    },
    Capsule {
        radius: f32,
        length: f32,
    },
}

impl GeometryType {
    /// Convert to URDF XML string for export
    pub fn to_urdf_xml(&self, mesh_uri: Option<&str>) -> String {
        match self {
            GeometryType::Mesh { path } => {
                let uri = mesh_uri.or(path.as_deref()).unwrap_or("");
                format!("<mesh filename=\"{}\"/>", uri)
            }
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

    /// Check if this is a mesh geometry
    pub fn is_mesh(&self) -> bool {
        matches!(self, GeometryType::Mesh { .. })
    }
}

impl From<&urdf_rs::Geometry> for GeometryType {
    fn from(geometry: &urdf_rs::Geometry) -> Self {
        match geometry {
            urdf_rs::Geometry::Mesh { filename, .. } => GeometryType::Mesh {
                path: Some(filename.clone()),
            },
            urdf_rs::Geometry::Box { size } => GeometryType::Box {
                size: [size.0[0] as f32, size.0[1] as f32, size.0[2] as f32],
            },
            urdf_rs::Geometry::Cylinder { radius, length } => GeometryType::Cylinder {
                radius: *radius as f32,
                length: *length as f32,
            },
            urdf_rs::Geometry::Sphere { radius } => GeometryType::Sphere {
                radius: *radius as f32,
            },
            urdf_rs::Geometry::Capsule { radius, length } => GeometryType::Capsule {
                radius: *radius as f32,
                length: *length as f32,
            },
        }
    }
}
