//! Node graph panel for assembly using egui-snarl

use std::collections::HashMap;

use egui_snarl::ui::{PinInfo, SnarlStyle, SnarlViewer};
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::panels::Panel;

/// Node types in the graph
#[derive(Debug, Clone)]
pub enum GraphNode {
    /// Base link (root of the robot)
    BaseLink { name: String },
    /// Part node
    Part {
        part_id: Uuid,
        name: String,
        /// Joint point indices used for connections
        input_point: Option<usize>,
        output_points: Vec<usize>,
    },
}

impl GraphNode {
    pub fn name(&self) -> &str {
        match self {
            GraphNode::BaseLink { name } => name,
            GraphNode::Part { name, .. } => name,
        }
    }
}

/// Graph panel using egui-snarl
pub struct GraphPanel {
    snarl: Snarl<GraphNode>,
    style: SnarlStyle,
    /// Mapping from part ID to node ID
    part_to_node: HashMap<Uuid, NodeId>,
}

impl GraphPanel {
    pub fn new() -> Self {
        let mut snarl = Snarl::new();

        // Add base link node
        snarl.insert_node(
            egui::pos2(100.0, 200.0),
            GraphNode::BaseLink {
                name: "base_link".to_string(),
            },
        );

        Self {
            snarl,
            style: SnarlStyle::default(),
            part_to_node: HashMap::new(),
        }
    }

    /// Add a part node to the graph
    pub fn add_part_node(&mut self, part_id: Uuid, name: String) -> NodeId {
        let node_id = self.snarl.insert_node(
            egui::pos2(300.0, 100.0 + self.part_to_node.len() as f32 * 100.0),
            GraphNode::Part {
                part_id,
                name,
                input_point: None,
                output_points: Vec::new(),
            },
        );
        self.part_to_node.insert(part_id, node_id);
        node_id
    }

    /// Remove a part node
    pub fn remove_part_node(&mut self, part_id: Uuid) {
        if let Some(node_id) = self.part_to_node.remove(&part_id) {
            self.snarl.remove_node(node_id);
        }
    }

    /// Sync graph with app state
    pub fn sync_with_state(&mut self, app_state: &SharedAppState) {
        let state = app_state.lock();

        // Add nodes for new parts
        for (id, part) in &state.parts {
            if !self.part_to_node.contains_key(id) {
                self.add_part_node(*id, part.name.clone());
            }
        }

        // Remove nodes for deleted parts
        let part_ids: Vec<Uuid> = state.parts.keys().cloned().collect();
        drop(state);

        let to_remove: Vec<Uuid> = self
            .part_to_node
            .keys()
            .filter(|id| !part_ids.contains(id))
            .cloned()
            .collect();

        for id in to_remove {
            self.remove_part_node(id);
        }
    }
}

impl Default for GraphPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Viewer implementation for the graph
struct GraphViewer<'a> {
    app_state: &'a SharedAppState,
}

impl<'a> SnarlViewer<GraphNode> for GraphViewer<'a> {
    fn title(&mut self, node: &GraphNode) -> String {
        node.name().to_string()
    }

    fn outputs(&mut self, node: &GraphNode) -> usize {
        match node {
            GraphNode::BaseLink { .. } => 1, // One output for connecting child
            GraphNode::Part { output_points, .. } => output_points.len().max(1), // At least one output
        }
    }

    fn inputs(&mut self, node: &GraphNode) -> usize {
        match node {
            GraphNode::BaseLink { .. } => 0, // Base link has no parent
            GraphNode::Part { .. } => 1,     // One input for parent connection
        }
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<GraphNode>,
    ) -> PinInfo {
        let node = &snarl[pin.id.node];

        match node {
            GraphNode::BaseLink { .. } => PinInfo::circle().with_fill(egui::Color32::GRAY),
            GraphNode::Part { .. } => {
                ui.label("Parent");
                PinInfo::circle().with_fill(egui::Color32::from_rgb(100, 200, 100))
            }
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<GraphNode>,
    ) -> PinInfo {
        let node = &snarl[pin.id.node];

        match node {
            GraphNode::BaseLink { .. } => {
                ui.label("Child");
                PinInfo::circle().with_fill(egui::Color32::from_rgb(100, 100, 200))
            }
            GraphNode::Part { .. } => {
                ui.label(format!("Out {}", pin.id.output));
                PinInfo::circle().with_fill(egui::Color32::from_rgb(100, 100, 200))
            }
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<GraphNode>) {
        // Prevent self-connections
        if from.id.node == to.id.node {
            return;
        }

        // Connect the nodes
        snarl.connect(from.id, to.id);

        tracing::info!("Connected {:?} -> {:?}", from.id, to.id);
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<GraphNode>) {
        snarl.disconnect(from.id, to.id);
        tracing::info!("Disconnected {:?} -> {:?}", from.id, to.id);
    }

    fn has_body(&mut self, _node: &GraphNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<GraphNode>,
    ) {
        let graph_node = &snarl[node];

        match graph_node {
            GraphNode::BaseLink { name } => {
                ui.label(format!("Root: {}", name));
            }
            GraphNode::Part { part_id, .. } => {
                let state = self.app_state.lock();
                if let Some(part) = state.parts.get(part_id) {
                    ui.label(format!("Vertices: {}", part.vertices.len()));
                    ui.label(format!("Joint Points: {}", part.joint_points.len()));
                }
            }
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<GraphNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<GraphNode>,
    ) {
        ui.label("Add Node");
        ui.separator();

        // List available parts
        let state = self.app_state.lock();
        let parts: Vec<(Uuid, String)> = state
            .parts
            .iter()
            .map(|(id, p)| (*id, p.name.clone()))
            .collect();
        drop(state);

        for (id, name) in parts {
            if ui.button(&name).clicked() {
                snarl.insert_node(
                    pos,
                    GraphNode::Part {
                        part_id: id,
                        name,
                        input_point: None,
                        output_points: Vec::new(),
                    },
                );
                ui.close_menu();
            }
        }
    }

    fn has_node_menu(&mut self, _node: &GraphNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<GraphNode>,
    ) {
        let graph_node = &snarl[node];

        match graph_node {
            GraphNode::BaseLink { .. } => {
                ui.label("Base Link (cannot delete)");
            }
            GraphNode::Part { .. } => {
                if ui.button("Delete").clicked() {
                    snarl.remove_node(node);
                    ui.close_menu();
                }
            }
        }
    }
}

impl Panel for GraphPanel {
    fn name(&self) -> &str {
        "Node Graph"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState) {
        // Sync nodes with parts
        self.sync_with_state(app_state);

        // Instructions
        ui.horizontal(|ui| {
            ui.label("Drag to pan, scroll to zoom. Right-click for menu.");
        });

        ui.separator();

        // Render the node graph
        let mut viewer = GraphViewer { app_state };

        self.snarl.show(&mut viewer, &self.style, "urdf_graph", ui);
    }
}
