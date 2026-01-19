//! Viewport rendering state

mod gizmo;
mod picking;
mod render_state;

pub use gizmo::{GizmoInteraction, GizmoTransform};
pub use picking::{PickablePartData, pick_object};
pub use render_state::ViewportState;

use parking_lot::Mutex;
use std::sync::Arc;

pub type SharedViewportState = Arc<Mutex<ViewportState>>;
