//! MCP server handler and tool definitions.
//!
//! Exposes four tools over MCP:
//! - `apply`: mutate the document with engine commands (JSON)
//! - `describe_scene`: structured JSON snapshot of the document
//! - `screenshot`: headless render of the current scene as PNG
//! - `command_reference`: documentation for every command
//!
//! stdout carries the JSON-RPC stream — all logging must go to stderr.

use std::sync::Arc;

use base64::Engine as _;
use parking_lot::Mutex;
use rk_engine::{Command, Engine, SharedEngine};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};

use crate::describe::describe_scene;
use crate::headless::{HeadlessRenderer, ScreenshotOptions, ViewPreset};

/// Command reference served by the `command_reference` tool.
/// Kept honest by `tests/reference_examples.rs`, which deserializes
/// every ```json block in it as a `Command`.
pub const COMMAND_REFERENCE: &str = include_str!("commands_reference.md");

const INSTRUCTIONS: &str = "RK is a parametric CAD editor for robots. This server exposes its \
headless engine. Use `describe_scene` to inspect the document, `apply` to mutate it with \
commands (call `command_reference` once for the full command catalog with JSON examples), and \
`screenshot` to see the 3D scene. Units are meters and radians, the world is Z-up. All changes \
are undoable via the `undo`/`redo` commands.";

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyRequest {
    /// Commands in the engine's JSON form, e.g.
    /// {"type":"create_primitive","id":null,"primitive":{"shape":"box","size":[0.1,0.1,0.1]},"name":"base"}.
    /// All commands are validated first, then applied in order; if one
    /// fails, the earlier ones stay applied (use {"type":"undo"} to revert).
    pub commands: Vec<serde_json::Value>,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenshotRequest {
    /// Image width in pixels (default 1024, max 4096)
    pub width: Option<u32>,
    /// Image height in pixels (default 768, max 4096)
    pub height: Option<u32>,
    /// Camera preset: iso (default), front, top or side
    pub view: Option<ViewPreset>,
    /// Camera yaw in degrees (overrides the preset)
    pub yaw_deg: Option<f32>,
    /// Camera pitch in degrees (overrides the preset)
    pub pitch_deg: Option<f32>,
    /// Camera distance from target in meters (default: auto-fit)
    pub distance: Option<f32>,
    /// Camera target point in meters (default: scene center)
    pub target: Option<[f32; 3]>,
    /// Whether to draw the ground grid (default true)
    pub show_grid: Option<bool>,
}

#[derive(Clone)]
pub struct RkMcpServer {
    engine: SharedEngine,
    /// GPU device is created lazily on the first screenshot so the
    /// server works on GPU-less machines until an image is requested
    headless: Arc<Mutex<Option<HeadlessRenderer>>>,
    tool_router: ToolRouter<Self>,
}

impl Default for RkMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl RkMcpServer {
    pub fn new() -> Self {
        let engine = Engine::new(Arc::from(rk_cad::default_kernel()));
        Self::with_engine(Arc::new(Mutex::new(engine)))
    }

    pub fn with_engine(engine: SharedEngine) -> Self {
        Self {
            engine,
            headless: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    pub fn engine(&self) -> &SharedEngine {
        &self.engine
    }

    #[tool(
        description = "Apply one or more engine commands to the CAD document (create parts, \
        joints, sketches, features; save/load; undo/redo). Call `command_reference` for the \
        full catalog with JSON examples. Commands are validated first, then applied in order; \
        on failure the earlier commands of the batch stay applied. Returns the emitted events."
    )]
    pub async fn apply(
        &self,
        Parameters(req): Parameters<ApplyRequest>,
    ) -> Result<CallToolResult, McpError> {
        if req.commands.is_empty() {
            return Err(McpError::invalid_params("commands must not be empty", None));
        }
        let commands: Vec<Command> = req
            .commands
            .iter()
            .enumerate()
            .map(|(i, raw)| {
                serde_json::from_value(raw.clone()).map_err(|e| {
                    McpError::invalid_params(
                        format!("commands[{i}] is not a valid command: {e}"),
                        Some(raw.clone()),
                    )
                })
            })
            .collect::<Result<_, _>>()?;

        let mut applied = Vec::new();
        let mut failure: Option<serde_json::Value> = None;
        {
            let mut engine = self.engine.lock();
            for (i, cmd) in commands.into_iter().enumerate() {
                let description = cmd.description();
                match engine.apply(cmd) {
                    Ok(events) => applied.push(serde_json::json!({
                        "command": description,
                        "events": events,
                    })),
                    Err(e) => {
                        failure = Some(serde_json::json!({
                            "index": i,
                            "command": description,
                            "message": e.to_string(),
                        }));
                        break;
                    }
                }
            }
        }

        let is_error = failure.is_some();
        let body = serde_json::json!({
            "applied": applied.len(),
            "results": applied,
            "error": failure,
        });
        let content = vec![Content::text(body.to_string())];
        Ok(if is_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        })
    }

    #[tool(
        description = "Get a structured JSON snapshot of the whole document: parts (with world \
        poses), links, joints, sketches (with entities and constraints), features, bodies, and \
        undo/redo state. Meshes are not included — use `screenshot` to see the scene."
    )]
    pub async fn describe_scene(&self) -> Result<CallToolResult, McpError> {
        let description = describe_scene(&self.engine.lock());
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&description)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    #[tool(
        description = "Render the current 3D scene headlessly and return a PNG image. Camera \
        auto-fits the scene; use view presets (iso/front/top/side) or explicit yaw/pitch/\
        distance/target to control it. Requires a GPU (the device is created on first use)."
    )]
    pub async fn screenshot(
        &self,
        Parameters(req): Parameters<ScreenshotRequest>,
    ) -> Result<CallToolResult, McpError> {
        let opts = ScreenshotOptions {
            width: req.width.unwrap_or(1024),
            height: req.height.unwrap_or(768),
            view: req.view.unwrap_or_default(),
            yaw_deg: req.yaw_deg,
            pitch_deg: req.pitch_deg,
            distance: req.distance,
            target: req.target,
            show_grid: req.show_grid.unwrap_or(true),
        };

        // GPU work is synchronous (readback blocks); keep it off the
        // async reactor
        let png = tokio::task::block_in_place(|| {
            let mut headless = self.headless.lock();
            if headless.is_none() {
                *headless = Some(HeadlessRenderer::new()?);
            }
            let renderer = headless.as_ref().expect("initialized above");
            renderer.screenshot(&mut self.engine.lock(), &opts)
        })
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
        Ok(CallToolResult::success(vec![Content::image(
            encoded,
            "image/png",
        )]))
    }

    #[tool(
        description = "Full catalog of engine commands accepted by `apply`, with JSON examples \
        and data-type conventions. Call this once before composing commands."
    )]
    pub async fn command_reference(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            COMMAND_REFERENCE,
        )]))
    }
}

#[tool_handler]
impl ServerHandler for RkMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(INSTRUCTIONS.to_string()),
        }
    }
}
