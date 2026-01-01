//! Inertia tensor calculations

use serde::{Deserialize, Serialize};

/// Inertia tensor (symmetric 3x3 matrix)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InertiaMatrix {
    pub ixx: f32,
    pub ixy: f32,
    pub ixz: f32,
    pub iyy: f32,
    pub iyz: f32,
    pub izz: f32,
}

impl Default for InertiaMatrix {
    fn default() -> Self {
        // Default to a small sphere-like inertia
        Self {
            ixx: 0.001,
            ixy: 0.0,
            ixz: 0.0,
            iyy: 0.001,
            iyz: 0.0,
            izz: 0.001,
        }
    }
}

impl InertiaMatrix {
    /// Create an inertia matrix for a solid box
    pub fn box_inertia(mass: f32, width: f32, height: f32, depth: f32) -> Self {
        let w2 = width * width;
        let h2 = height * height;
        let d2 = depth * depth;
        let k = mass / 12.0;
        Self {
            ixx: k * (h2 + d2),
            ixy: 0.0,
            ixz: 0.0,
            iyy: k * (w2 + d2),
            iyz: 0.0,
            izz: k * (w2 + h2),
        }
    }

    /// Create an inertia matrix for a solid cylinder (along Z axis)
    pub fn cylinder_inertia(mass: f32, radius: f32, length: f32) -> Self {
        let r2 = radius * radius;
        let l2 = length * length;
        Self {
            ixx: mass * (3.0 * r2 + l2) / 12.0,
            ixy: 0.0,
            ixz: 0.0,
            iyy: mass * (3.0 * r2 + l2) / 12.0,
            iyz: 0.0,
            izz: mass * r2 / 2.0,
        }
    }

    /// Create an inertia matrix for a solid sphere
    pub fn sphere_inertia(mass: f32, radius: f32) -> Self {
        let i = 2.0 * mass * radius * radius / 5.0;
        Self {
            ixx: i,
            ixy: 0.0,
            ixz: 0.0,
            iyy: i,
            iyz: 0.0,
            izz: i,
        }
    }

    /// Calculate approximate inertia from mesh bounding box
    pub fn from_bounding_box(mass: f32, bbox_min: [f32; 3], bbox_max: [f32; 3]) -> Self {
        let width = bbox_max[0] - bbox_min[0];
        let height = bbox_max[1] - bbox_min[1];
        let depth = bbox_max[2] - bbox_min[2];
        Self::box_inertia(mass, width, height, depth)
    }

    /// Check if the inertia matrix is physically valid
    pub fn is_valid(&self) -> bool {
        // Diagonal elements must be positive
        if self.ixx <= 0.0 || self.iyy <= 0.0 || self.izz <= 0.0 {
            return false;
        }

        // Triangle inequality: each diagonal must be <= sum of other two
        let ixx = self.ixx;
        let iyy = self.iyy;
        let izz = self.izz;

        ixx <= iyy + izz && iyy <= ixx + izz && izz <= ixx + iyy
    }

    /// Get as array for URDF export [ixx, ixy, ixz, iyy, iyz, izz]
    pub fn to_array(&self) -> [f64; 6] {
        [
            self.ixx as f64,
            self.ixy as f64,
            self.ixz as f64,
            self.iyy as f64,
            self.iyz as f64,
            self.izz as f64,
        ]
    }
}

/// Calculate volume of a mesh using signed tetrahedron method
pub fn calculate_mesh_volume(vertices: &[[f32; 3]], indices: &[u32]) -> f32 {
    let mut volume = 0.0;

    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }
        let v0 = vertices[triangle[0] as usize];
        let v1 = vertices[triangle[1] as usize];
        let v2 = vertices[triangle[2] as usize];

        // Signed volume of tetrahedron formed with origin
        volume += signed_tetrahedron_volume(v0, v1, v2);
    }

    volume.abs()
}

fn signed_tetrahedron_volume(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> f32 {
    // V = (1/6) * |v0 . (v1 x v2)|
    let cross = [
        v1[1] * v2[2] - v1[2] * v2[1],
        v1[2] * v2[0] - v1[0] * v2[2],
        v1[0] * v2[1] - v1[1] * v2[0],
    ];
    (v0[0] * cross[0] + v0[1] * cross[1] + v0[2] * cross[2]) / 6.0
}

/// Estimate mass from volume and density
pub fn mass_from_volume(volume: f32, density: f32) -> f32 {
    volume * density
}

/// Default density for common materials (kg/m^3)
pub mod density {
    pub const PLASTIC_ABS: f32 = 1050.0;
    pub const PLASTIC_PLA: f32 = 1240.0;
    pub const ALUMINUM: f32 = 2700.0;
    pub const STEEL: f32 = 7850.0;
    pub const TITANIUM: f32 = 4500.0;
}
