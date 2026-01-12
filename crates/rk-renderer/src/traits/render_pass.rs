//! Render pass types and abstractions.

/// Type of render pass for multi-pass rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassType {
    /// Main opaque geometry pass (depth tested, no blending)
    Opaque,
    /// Transparent geometry pass (back-to-front sorted, alpha blending)
    Transparent,
    /// Overlay pass (gizmos, UI elements - rendered on top)
    Overlay,
    /// Shadow map generation pass
    Shadow,
    /// Post-processing pass
    PostProcess,
}

impl PassType {
    /// Returns true if this pass should use depth testing.
    pub fn uses_depth_test(&self) -> bool {
        matches!(
            self,
            PassType::Opaque | PassType::Transparent | PassType::Shadow
        )
    }

    /// Returns true if this pass should write to the depth buffer.
    pub fn writes_depth(&self) -> bool {
        matches!(self, PassType::Opaque | PassType::Shadow)
    }
}
