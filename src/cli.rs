use std::path::PathBuf;

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(author, version, about = "A command-line tool for recording notes")]
#[command(propagate_version = true)]
pub struct Cli {
    /// Directory to store notes
    #[arg(short = 'd', long = "notes-dir", global = true)]
    pub notes_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Title of the note (if no subcommand is provided)
    #[arg(short = 't', long = "title", global = true)]
    pub title: Option<String>,

    /// File to read note content from (if no subcommand is provided)
    #[arg(short = 'f', long = "file", global = true)]
    pub file: Option<PathBuf>,

    /// Note content (if no subcommand is provided, defaults to 'add')
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new note
    Add(AddArgs),
    /// MCP command (not yet implemented)
    Mcp(McpArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// Title of the note
    #[arg(short = 't', long = "title")]
    pub title: Option<String>,

    /// File to read note content from
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Note content
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// Arguments for the mcp command
#[derive(Args)]
pub struct McpArgs {
    // We need to capture global options to check if they were provided

    /// Title of the note (should not be used with mcp)
    #[arg(short = 't', long = "title", hide = true)]
    pub title: Option<String>,

    /// File to read note content from (should not be used with mcp)
    #[arg(short = 'f', long = "file", hide = true)]
    pub file: Option<PathBuf>,

    /// Arguments (should not be used with mcp)
    #[arg(trailing_var_arg = true, hide = true)]
    pub args: Vec<String>,
}
