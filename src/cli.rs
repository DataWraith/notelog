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
    
    /// Note content (if no subcommand is provided, defaults to 'add')
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new note
    Add(AddArgs),
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
