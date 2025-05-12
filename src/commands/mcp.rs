use std::path::Path;

use crate::cli::McpArgs;
use crate::error::{NotelogError, Result};
use crate::mcp;

/// Handle the mcp command
pub fn mcp_command(notes_dir: &Path, args: McpArgs) -> Result<()> {
    // Check if any options were provided that are not allowed
    if args.title.is_some() || args.file.is_some() || !args.args.is_empty() {
        return Err(NotelogError::InvalidMcpOptions);
    }

    // Run the MCP server with database initialization
    // This uses a single Tokio runtime for both database initialization and the MCP server
    match mcp::run_mcp_server_with_db(notes_dir) {
        Ok(_) => Ok(()),
        Err(e) => Err(NotelogError::McpServerError(e.to_string())),
    }
}
