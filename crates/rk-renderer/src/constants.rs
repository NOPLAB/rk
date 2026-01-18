//! Rendering constants and configuration
//!
//! This module centralizes all magic numbers and configuration constants
//! used across the renderer to improve maintainability.

/// Grid rendering constants
pub mod grid {
    /// Default grid extent (half-size in each direction)
    pub const DEFAULT_SIZE: f32 = 10.0;
    /// Default grid line spacing
    pub const DEFAULT_SPACING: f32 = 1.0;
    /// Grid line color (gray)
    pub const LINE_COLOR: [f32; 3] = [0.3, 0.3, 0.3];
    /// X-axis color (red)
    pub const X_AXIS_COLOR: [f32; 3] = [0.8, 0.2, 0.2];
    /// Y-axis color (green)
    pub const Y_AXIS_COLOR: [f32; 3] = [0.2, 0.8, 0.2];
}

/// Gizmo rendering constants
pub mod gizmo {
    /// Arrow shaft radius
    pub const SHAFT_RADIUS: f32 = 0.02;
    /// Arrow head radius
    pub const HEAD_RADIUS: f32 = 0.06;
    /// Arrow head length
    pub const HEAD_LENGTH: f32 = 0.15;
    /// Total arrow length
    pub const ARROW_LENGTH: f32 = 1.0;
    /// Number of segments for cylindrical geometry
    pub const SEGMENTS: u32 = 8;
    /// Hit test cylinder radius multiplier
    pub const HIT_RADIUS_MULTIPLIER: f32 = 0.08;

    /// Rotation gizmo ring radius
    pub const RING_RADIUS: f32 = 0.8;
    /// Rotation gizmo ring tube thickness
    pub const RING_THICKNESS: f32 = 0.03;
    /// Number of segments for ring (around the circle)
    pub const RING_SEGMENTS: u32 = 32;
    /// Number of segments for ring tube (cross-section)
    pub const RING_TUBE_SEGMENTS: u32 = 8;
    /// Hit test ring thickness multiplier
    pub const RING_HIT_THICKNESS: f32 = 0.1;

    /// Scale gizmo cube size (half-extent)
    pub const SCALE_CUBE_SIZE: f32 = 0.06;
    /// Scale gizmo axis length (distance to cube center)
    pub const SCALE_AXIS_LENGTH: f32 = 0.9;
    /// Scale gizmo line radius
    pub const SCALE_LINE_RADIUS: f32 = 0.015;
    /// Scale gizmo hit test cube size
    pub const SCALE_HIT_SIZE: f32 = 0.12;

    /// Axis colors for gizmo
    pub mod colors {
        /// X-axis color (red)
        pub const X_AXIS: [f32; 4] = [1.0, 0.2, 0.2, 1.0];
        /// Y-axis color (green)
        pub const Y_AXIS: [f32; 4] = [0.2, 1.0, 0.2, 1.0];
        /// Z-axis color (blue)
        pub const Z_AXIS: [f32; 4] = [0.2, 0.2, 1.0, 1.0];
    }
}

/// Marker (sphere) rendering constants
pub mod marker {
    /// Number of horizontal segments for sphere
    pub const SEGMENTS: u32 = 16;
    /// Number of vertical rings for sphere
    pub const RINGS: u32 = 12;
}

/// Instance buffer limits
pub mod instances {
    /// Maximum number of axis instances
    pub const MAX_AXES: u32 = 64;
    /// Maximum number of marker instances
    pub const MAX_MARKERS: u32 = 256;
    /// Maximum number of collision instances
    pub const MAX_COLLISIONS: u32 = 128;
}

/// Collision visualization constants
pub mod collision {
    /// Number of segments for cylinder/capsule geometry
    pub const SEGMENTS: u32 = 16;
    /// Number of rings for sphere geometry
    pub const RINGS: u32 = 12;
    /// Default collision color (semi-transparent cyan)
    pub const DEFAULT_COLOR: [f32; 4] = [0.0, 0.8, 0.8, 0.3];
    /// Selected collision color (semi-transparent yellow)
    pub const SELECTED_COLOR: [f32; 4] = [1.0, 0.8, 0.0, 0.5];
}

/// Camera default parameters
pub mod camera {
    /// Default field of view in degrees
    pub const DEFAULT_FOV_DEGREES: f32 = 40.0;
    /// Default near clipping plane
    pub const DEFAULT_NEAR: f32 = 0.1;
    /// Default far clipping plane
    pub const DEFAULT_FAR: f32 = 100000.0;
    /// Default orbit distance
    pub const DEFAULT_DISTANCE: f32 = 5.0;
    /// Default yaw angle in degrees
    pub const DEFAULT_YAW_DEGREES: f32 = 45.0;
    /// Default pitch angle in degrees
    pub const DEFAULT_PITCH_DEGREES: f32 = 30.0;
    /// Minimum pitch angle in degrees
    pub const MIN_PITCH_DEGREES: f32 = -89.0;
    /// Maximum pitch angle in degrees
    pub const MAX_PITCH_DEGREES: f32 = 89.0;
    /// Pan sensitivity multiplier
    pub const PAN_SCALE: f32 = 0.002;
    /// Zoom sensitivity multiplier
    pub const ZOOM_SCALE: f32 = 0.1;
    /// Minimum orbit distance
    pub const MIN_DISTANCE: f32 = 0.1;
    /// Maximum orbit distance
    pub const MAX_DISTANCE: f32 = 10000.0;
    /// Fit-all radius multiplier
    pub const FIT_ALL_MULTIPLIER: f32 = 2.5;
}

/// Viewport rendering constants
pub mod viewport {
    /// Background clear color
    pub const CLEAR_COLOR: wgpu::Color = wgpu::Color {
        r: 0.15,
        g: 0.15,
        b: 0.18,
        a: 1.0,
    };

    /// MSAA sample count for anti-aliasing (1 = disabled, 4 = 4x MSAA)
    pub const SAMPLE_COUNT: u32 = 4;
}

/// Shadow mapping constants
pub mod shadow {
    /// Shadow map resolution (width and height in pixels)
    pub const SHADOW_MAP_SIZE: u32 = 2048;
    /// Shadow map depth texture format
    pub const SHADOW_MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    /// Default shadow depth bias to prevent shadow acne
    pub const DEFAULT_BIAS: f32 = 0.005;
    /// Default normal-based shadow bias for grazing angles
    pub const DEFAULT_NORMAL_BIAS: f32 = 0.01;
}
