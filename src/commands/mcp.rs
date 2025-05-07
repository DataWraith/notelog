use std::path::Path;

use crate::cli::McpArgs;
use crate::error::{NotelogError, Result};
use crate::mcp::{self, Calculator};

/// Handle the mcp command
pub fn mcp_command(_notes_dir: &Path, args: McpArgs) -> Result<()> {
    // Check if any options were provided that are not allowed
    if args.title.is_some() || args.file.is_some() || !args.args.is_empty() {
        return Err(NotelogError::InvalidMcpOptions);
    }

    // Create a new Calculator handler
    let handler = Calculator;

    // Run the MCP server with the handler
    match mcp::run_mcp_server(handler) {
        Ok(_) => Ok(()),
        Err(e) => Err(NotelogError::McpServerError(e.to_string())),
    }
}
