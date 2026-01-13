//! Renderer configuration structures
//!
//! This module provides configurable settings for the renderer that can be
//! serialized and loaded from configuration files.

use serde::{Deserialize, Serialize};

/// Grid rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridConfig {
    /// Whether the grid is enabled
    pub enabled: bool,
    /// Grid extent (half-size in each direction)
    pub size: f32,
    /// Grid line spacing
    pub spacing: f32,
    /// Grid line color (RGB)
    pub line_color: [f32; 3],
    /// X-axis color (RGB)
    pub x_axis_color: [f32; 3],
    /// Y-axis color (RGB)
    pub y_axis_color: [f32; 3],
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 10.0,
            spacing: 1.0,
            line_color: [0.3, 0.3, 0.3],
            x_axis_color: [0.8, 0.2, 0.2],
            y_axis_color: [0.2, 0.8, 0.2],
        }
    }
}

/// Viewport rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewportConfig {
    /// Background clear color (RGBA)
    pub background_color: [f32; 4],
    /// MSAA sample count (1 = disabled, 2, 4, 8)
    pub msaa_sample_count: u32,
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            background_color: [0.15, 0.15, 0.18, 1.0],
            msaa_sample_count: 4,
        }
    }
}

/// Shadow mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowConfig {
    /// Whether shadows are enabled
    pub enabled: bool,
    /// Shadow map resolution (512, 1024, 2048, 4096)
    pub map_size: u32,
    /// Shadow depth bias to prevent shadow acne
    pub bias: f32,
    /// Normal-based shadow bias for grazing angles
    pub normal_bias: f32,
    /// Shadow softness (PCF filter size)
    pub softness: f32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            map_size: 2048,
            bias: 0.005,
            normal_bias: 0.01,
            softness: 1.0,
        }
    }
}

/// Lighting configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightingConfig {
    /// Light direction (normalized)
    pub direction: [f32; 3],
    /// Light color (RGB)
    pub color: [f32; 3],
    /// Light intensity multiplier
    pub intensity: f32,
    /// Ambient light color (RGB)
    pub ambient_color: [f32; 3],
    /// Ambient light strength
    pub ambient_strength: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            direction: [0.5, 0.5, 1.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            ambient_color: [1.0, 1.0, 1.0],
            ambient_strength: 0.3,
        }
    }
}

/// Camera default configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraConfig {
    /// Field of view in degrees
    pub fov_degrees: f32,
    /// Near clipping plane distance
    pub near_plane: f32,
    /// Far clipping plane distance
    pub far_plane: f32,
    /// Pan sensitivity multiplier
    pub pan_sensitivity: f32,
    /// Zoom sensitivity multiplier
    pub zoom_sensitivity: f32,
    /// Orbit sensitivity multiplier
    pub orbit_sensitivity: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            fov_degrees: 40.0,
            near_plane: 0.1,
            far_plane: 100000.0,
            pan_sensitivity: 0.002,
            zoom_sensitivity: 0.1,
            orbit_sensitivity: 0.005,
        }
    }
}

/// Gizmo configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GizmoConfig {
    /// Whether gizmos are enabled
    pub enabled: bool,
    /// Gizmo scale multiplier
    pub scale: f32,
    /// X-axis color (RGBA)
    pub x_axis_color: [f32; 4],
    /// Y-axis color (RGBA)
    pub y_axis_color: [f32; 4],
    /// Z-axis color (RGBA)
    pub z_axis_color: [f32; 4],
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scale: 1.0,
            x_axis_color: [1.0, 0.2, 0.2, 1.0],
            y_axis_color: [0.2, 1.0, 0.2, 1.0],
            z_axis_color: [0.2, 0.2, 1.0, 1.0],
        }
    }
}

/// Complete renderer configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RendererConfig {
    /// Grid settings
    #[serde(default)]
    pub grid: GridConfig,
    /// Viewport settings
    #[serde(default)]
    pub viewport: ViewportConfig,
    /// Shadow settings
    #[serde(default)]
    pub shadow: ShadowConfig,
    /// Lighting settings
    #[serde(default)]
    pub lighting: LightingConfig,
    /// Camera settings
    #[serde(default)]
    pub camera: CameraConfig,
    /// Gizmo settings
    #[serde(default)]
    pub gizmo: GizmoConfig,
}

impl RendererConfig {
    /// Create a new renderer configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
}
