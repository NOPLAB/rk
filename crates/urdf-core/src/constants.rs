//! Global constants for urdf-core

/// STL vertex comparison precision (multiply by this, then round to int)
pub const STL_VERTEX_PRECISION: f32 = 10000.0;

/// Default number of segments for cylinder mesh generation
pub const CYLINDER_SEGMENTS: u32 = 32;

/// Default number of latitude segments for sphere mesh generation
pub const SPHERE_LAT_SEGMENTS: u32 = 16;

/// Default number of longitude segments for sphere mesh generation
pub const SPHERE_LON_SEGMENTS: u32 = 32;

/// Default color for parts and visuals (gray, RGBA)
pub const DEFAULT_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
