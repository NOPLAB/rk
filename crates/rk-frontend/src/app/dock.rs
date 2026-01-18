//! Dock layout and tab viewer

use egui_dock::{DockState, NodeIndex, TabViewer};

use crate::config::SharedConfig;
use crate::panels::{
    FeatureTreePanel, JointListPanel, Panel, PartListPanel, PropertiesPanel, ViewportPanel,
};
use crate::state::{SharedAppState, SharedViewportState};

/// Panel types for the dock system
pub enum PanelType {
    Viewport(ViewportPanel),
    PartList(PartListPanel),
    JointList(JointListPanel),
    FeatureTree(FeatureTreePanel),
    Properties(PropertiesPanel),
}

impl PanelType {
    pub fn name(&self) -> &str {
        match self {
            PanelType::Viewport(p) => p.name(),
            PanelType::PartList(p) => p.name(),
            PanelType::JointList(p) => p.name(),
            PanelType::FeatureTree(p) => p.name(),
            PanelType::Properties(p) => p.name(),
        }
    }
}

/// Tab viewer for dock area
pub struct UrdfTabViewer<'a> {
    pub app_state: &'a SharedAppState,
    pub render_state: Option<&'a egui_wgpu::RenderState>,
    pub viewport_state: &'a Option<SharedViewportState>,
    pub config: &'a SharedConfig,
}

impl TabViewer for UrdfTabViewer<'_> {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelType::Viewport(panel) => {
                if let (Some(render_state), Some(viewport_state)) =
                    (self.render_state, self.viewport_state)
                {
                    panel.ui_with_render_context(
                        ui,
                        self.app_state,
                        render_state,
                        viewport_state,
                        self.config,
                    );
                } else {
                    panel.ui(ui, self.app_state);
                }
            }
            PanelType::PartList(panel) => panel.ui(ui, self.app_state),
            PanelType::JointList(panel) => panel.ui(ui, self.app_state),
            PanelType::FeatureTree(panel) => panel.ui(ui, self.app_state),
            PanelType::Properties(panel) => {
                if let (Some(render_state), Some(viewport_state)) =
                    (self.render_state, self.viewport_state)
                {
                    panel.ui_with_render_context(
                        ui,
                        self.app_state,
                        render_state,
                        viewport_state,
                        self.config,
                    );
                } else {
                    panel.ui(ui, self.app_state);
                }
            }
        }
    }
}

/// Create the default dock layout
pub fn create_dock_layout() -> DockState<PanelType> {
    let mut dock_state = DockState::new(vec![PanelType::Viewport(ViewportPanel::new())]);

    // Get the main surface
    let surface = dock_state.main_surface_mut();

    // 1. Split right for properties (Viewport 75%, Properties 25%)
    let [viewport_area, _properties] = surface.split_right(
        NodeIndex::root(),
        0.75,
        vec![PanelType::Properties(PropertiesPanel::new())],
    );

    // 2. Split left from viewport area for parts list
    let [left, _viewport] = surface.split_left(
        viewport_area,
        0.25, // Left panel gets 25% of the viewport area
        vec![PanelType::PartList(PartListPanel::new())],
    );

    // 3. Split left panel vertically to add joints and features below parts
    let [_parts, _joints] = surface.split_below(
        left,
        0.6, // Parts gets 60%, bottom tabs get 40%
        vec![
            PanelType::JointList(JointListPanel::new()),
            PanelType::FeatureTree(FeatureTreePanel::new()),
        ],
    );

    dock_state
}
