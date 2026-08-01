//! Display options for controlling visibility of rendering elements.

/// Display options for controlling visibility of rendering elements.
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    /// Whether the grid is visible.
    pub show_grid: bool,
    /// Whether axes are visible.
    pub show_axes: bool,
    /// Whether markers are visible.
    pub show_markers: bool,
    /// Whether the gizmo rendering is enabled.
    pub show_gizmo: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_axes: true,
            show_markers: true,
            show_gizmo: true,
        }
    }
}

impl DisplayOptions {
    /// Create new display options with all elements visible.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get whether the grid is visible.
    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    /// Set whether the grid is visible.
    pub fn set_show_grid(&mut self, show: bool) {
        self.show_grid = show;
    }

    /// Get whether axes are visible.
    pub fn show_axes(&self) -> bool {
        self.show_axes
    }

    /// Set whether axes are visible.
    pub fn set_show_axes(&mut self, show: bool) {
        self.show_axes = show;
    }

    /// Get whether markers are visible.
    pub fn show_markers(&self) -> bool {
        self.show_markers
    }

    /// Set whether markers are visible.
    pub fn set_show_markers(&mut self, show: bool) {
        self.show_markers = show;
    }

    /// Get whether the gizmo rendering is enabled.
    pub fn is_gizmo_enabled(&self) -> bool {
        self.show_gizmo
    }

    /// Set whether the gizmo rendering is enabled.
    pub fn set_gizmo_enabled(&mut self, enabled: bool) {
        self.show_gizmo = enabled;
    }
}
