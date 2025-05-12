//! MCP (Model Context Protocol) implementation for notelog

mod add_note;

pub use add_note::AddNote;

use tokio::runtime::Runtime;

/// Creates a new tokio runtime for MCP operations
pub fn create_runtime() -> Result<Runtime, std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}

/// Runs the MCP server with database initialization
///
/// This function creates a single Tokio runtime that handles both database initialization
/// and running the MCP server.
pub fn run_mcp_server_with_db<P: AsRef<std::path::Path>>(
    notes_dir: P,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::db::Database;
    use crate::mcp::AddNote;

    let rt = create_runtime()?;

    rt.block_on(async {
        // Initialize the database
        let db = Database::initialize(notes_dir.as_ref()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // Start the background task to index notes
        db.start_indexing_task().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // Create the AddNote handler with the database
        let handler = AddNote::with_db(notes_dir, db);

        use rmcp::ServiceExt;
        use tokio::io::{stdin, stdout};

        // Set up the transport using stdin and stdout
        let stdin = stdin();
        let stdout = stdout();
        let transport = (stdin, stdout);

        // Create and run the server with the provided handler
        let server = handler.serve(transport).await?;

        // Wait for the server to complete (this will block until STDIN is closed)
        let _quit_reason = server.waiting().await?;

        Ok(())
    })
}
