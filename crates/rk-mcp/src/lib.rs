//! MCP server exposing the RK headless engine to AI agents.
//!
//! Tools: apply commands, describe the scene, take screenshots
//! (headless rendering) and read the command reference.

pub mod describe;
pub mod headless;
pub mod server;

pub use server::RkMcpServer;
