//! Application configuration module
//!
//! This module handles application-wide configuration including renderer settings,
//! editor preferences, and UI settings.

mod manager;

pub use manager::{ConfigError, ConfigManager, SharedConfig, create_shared_config};

use rk_core::StlUnit;
use rk_renderer::config::RendererConfig;
use serde::{Deserialize, Serialize};

use crate::state::AngleDisplayMode;

/// Editor preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorConfig {
    /// Show axes on selected part
    pub show_part_axes: bool,
    /// Show joint point markers
    pub show_joint_markers: bool,
    /// Angle display mode for joint sliders
    pub angle_display_mode: AngleDisplayMode,
    /// Default unit for STL import
    pub stl_import_unit: StlUnit,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_part_axes: true,
            show_joint_markers: true,
            angle_display_mode: AngleDisplayMode::Degrees,
            stl_import_unit: StlUnit::Millimeters,
        }
    }
}

/// UI theme
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UiTheme {
    #[default]
    Dark,
    Light,
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    /// UI theme
    pub theme: UiTheme,
    /// Font size multiplier
    pub font_size: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: UiTheme::Dark,
            font_size: 1.0,
        }
    }
}

/// Complete application configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppConfig {
    /// Configuration format version
    #[serde(default)]
    pub version: u32,
    /// Renderer settings
    #[serde(default)]
    pub renderer: RendererConfig,
    /// Editor settings
    #[serde(default)]
    pub editor: EditorConfig,
    /// UI settings
    #[serde(default)]
    pub ui: UiConfig,
}

impl AppConfig {
    /// Current configuration version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            ..Default::default()
        }
    }
}
