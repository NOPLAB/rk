//! rk-mcp: MCP server for the RK CAD engine (stdio transport).
//!
//! stdout is the JSON-RPC channel; logs go to stderr (RUST_LOG works).

use rk_mcp::RkMcpServer;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("rk-mcp starting (kernel: {})", {
        let kernel = rk_cad::default_kernel();
        kernel.name().to_string()
    });

    let service = RkMcpServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
