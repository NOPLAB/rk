//! URDF Editor Frontend
//!
//! egui-based application for editing URDF files.

pub mod actions;
pub mod app;
pub mod config;
pub mod fonts;
pub mod panels;
pub mod state;
pub mod update;

// Re-exports for convenience
pub use app::UrdfEditorApp;
pub use config::{AppConfig, ConfigManager, SharedConfig};
pub use state::{AppAction, AppState, SharedAppState};
