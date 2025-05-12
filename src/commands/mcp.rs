use std::path::Path;

use crate::cli::McpArgs;
use crate::db::Database;
use crate::error::{NotelogError, Result};
use crate::mcp::{self, AddNote};

/// Handle the mcp command
pub fn mcp_command(notes_dir: &Path, args: McpArgs) -> Result<()> {
    // Check if any options were provided that are not allowed
    if args.title.is_some() || args.file.is_some() || !args.args.is_empty() {
        return Err(NotelogError::InvalidMcpOptions);
    }

    // Create a new tokio runtime for database initialization
    let rt = mcp::create_runtime()?;

    // Initialize the database
    let db = rt.block_on(async {
        let db = Database::initialize(notes_dir).await?;

        // Start the background task to index notes
        db.start_indexing_task().await?;

        Ok::<_, NotelogError>(db)
    })?;

    // Create a new AddNote handler with the notes directory and database
    let handler = AddNote::with_db(notes_dir, db);

    // Run the MCP server with the handler
    match mcp::run_mcp_server(handler) {
        Ok(_) => Ok(()),
        Err(e) => Err(NotelogError::McpServerError(e.to_string())),
    }
}
