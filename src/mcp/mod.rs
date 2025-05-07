//! MCP (Model Context Protocol) implementation for notelog

mod calculator;

pub use calculator::Calculator;

use rmcp::ServerHandler;
use tokio::runtime::Runtime;

/// Creates a new tokio runtime for MCP operations
pub fn create_runtime() -> Result<Runtime, std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}

/// Runs the MCP server with the given handler
pub fn run_mcp_server<H>(handler: H) -> Result<(), Box<dyn std::error::Error>>
where
    H: ServerHandler + 'static,
{
    let rt = create_runtime()?;

    rt.block_on(async {
        use rmcp::ServiceExt;
        use tokio::io::{stdin, stdout};

        // Set up the transport using stdin and stdout
        let stdin = stdin();
        let stdout = stdout();
        let transport = (stdin, stdout);

        // Create and run the server with the provided handler
        // This will block until STDIN is closed
        let server = handler.serve(transport).await?;

        // Wait for the server to complete (this will block until STDIN is closed)
        let _quit_reason = server.waiting().await?;

        Ok(())
    })
}
