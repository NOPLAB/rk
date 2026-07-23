//! Color palette for the UI theme
//!
//! This module defines the color constants used throughout the application.
//! Colors are designed for a modern CAD application, inspired by Blender,
//! Fusion 360, and VS Code Dark+.

use egui::Color32;

// =============================================================================
// Background hierarchy (dark to light)
// =============================================================================

/// Base viewport background
pub const BG_BASE: Color32 = Color32::from_rgb(24, 24, 28);
/// Panel background
pub const BG_PANEL: Color32 = Color32::from_rgb(30, 30, 35);
/// Elevated surfaces (overlays, popups)
pub const BG_ELEVATED: Color32 = Color32::from_rgb(38, 38, 44);
/// Input field background
pub const BG_INPUT: Color32 = Color32::from_rgb(45, 45, 52);
/// Hover state background
pub const BG_HOVER: Color32 = Color32::from_rgb(55, 55, 65);

// =============================================================================
// Borders
// =============================================================================

/// Subtle panel boundary
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(50, 50, 58);
/// Normal divider line
pub const BORDER_NORMAL: Color32 = Color32::from_rgb(65, 65, 75);
/// Strong focus ring
pub const BORDER_STRONG: Color32 = Color32::from_rgb(85, 85, 95);

// =============================================================================
// Text hierarchy
// =============================================================================

/// Primary text color
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 235);
/// Secondary text (labels)
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 170);
/// Disabled text
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(100, 100, 110);

// =============================================================================
// Accent colors (blue)
// =============================================================================

/// Primary accent (selection, active)
pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(66, 150, 250);
/// Accent hover state
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(90, 170, 255);
/// Subtle accent for selection background
pub const ACCENT_SUBTLE: Color32 = Color32::from_rgba_premultiplied(66, 150, 250, 30);

// =============================================================================
// Semantic colors
// =============================================================================

/// Success color
pub const SUCCESS: Color32 = Color32::from_rgb(80, 200, 120);
/// Warning color
pub const WARNING: Color32 = Color32::from_rgb(255, 180, 60);
/// Error color
pub const ERROR: Color32 = Color32::from_rgb(255, 90, 90);

// =============================================================================
// Axis colors (CAD standard: XYZ = RGB)
// =============================================================================

/// X axis color (red)
pub const AXIS_X: Color32 = Color32::from_rgb(255, 75, 75);
/// Y axis color (green)
pub const AXIS_Y: Color32 = Color32::from_rgb(75, 255, 100);
/// Z axis color (blue)
pub const AXIS_Z: Color32 = Color32::from_rgb(75, 150, 255);

// =============================================================================
// Reference plane colors (CAD standard)
// =============================================================================

/// XY plane color (blue - perpendicular to Z)
pub const PLANE_XY: Color32 = Color32::from_rgb(77, 128, 230);
/// XZ plane color (green - perpendicular to Y)
pub const PLANE_XZ: Color32 = Color32::from_rgb(77, 230, 128);
/// YZ plane color (orange - perpendicular to X)
pub const PLANE_YZ: Color32 = Color32::from_rgb(230, 128, 77);

// =============================================================================
// Constraint icon colors
// =============================================================================

/// Geometric constraint color (cyan-ish)
pub const CONSTRAINT_GEOMETRIC: Color32 = Color32::from_rgb(100, 200, 220);
/// Dimensional constraint color (orange)
pub const CONSTRAINT_DIMENSION: Color32 = Color32::from_rgb(255, 180, 80);
/// Fixed constraint color (green)
pub const CONSTRAINT_FIXED: Color32 = Color32::from_rgb(120, 220, 120);

// =============================================================================
// Light theme colors
// =============================================================================

pub mod light {
    use egui::Color32;

    /// Base viewport background
    pub const BG_BASE: Color32 = Color32::from_rgb(245, 245, 248);
    /// Panel background
    pub const BG_PANEL: Color32 = Color32::from_rgb(250, 250, 252);
    /// Elevated surfaces (overlays, popups)
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(255, 255, 255);
    /// Input field background
    pub const BG_INPUT: Color32 = Color32::from_rgb(240, 240, 244);
    /// Hover state background
    pub const BG_HOVER: Color32 = Color32::from_rgb(230, 230, 236);

    /// Subtle panel boundary
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(220, 220, 226);
    /// Normal divider line
    pub const BORDER_NORMAL: Color32 = Color32::from_rgb(200, 200, 210);
    /// Strong focus ring
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(170, 170, 185);

    /// Primary text color
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 30, 35);
    /// Secondary text (labels)
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(90, 90, 100);
    /// Disabled text
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(150, 150, 160);

    /// Primary accent (selection, active)
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(45, 120, 220);
    /// Accent hover state
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(60, 140, 240);
    /// Subtle accent for selection background
    pub const ACCENT_SUBTLE: Color32 = Color32::from_rgba_premultiplied(45, 120, 220, 40);
}

// =============================================================================
// Helper functions
// =============================================================================

/// Create a semi-transparent version of the elevated background for overlays
pub fn overlay_bg(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(BG_ELEVATED.r(), BG_ELEVATED.g(), BG_ELEVATED.b(), alpha)
}

/// Create a semi-transparent version of a color
pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}
