mod cli;
mod commands;
mod core;
mod error;
mod mcp;
mod utils;

use std::io::{self, Read};

use atty;
use clap::Parser;

use cli::{AddArgs, Cli, Commands};
use error::Result;
use utils::{ensure_notes_dir_exists, get_notes_dir};

fn main() {
    if let Err(e) = run() {
        // Print the full error message to stderr
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Determine the notes directory
    let notes_dir = get_notes_dir(cli.notes_dir)?;

    // Ensure the notes directory exists and is writable
    ensure_notes_dir_exists(&notes_dir)?;

    // Handle the command (or default to 'add')
    match cli.command {
        Some(Commands::Add(args)) => {
            // Only check stdin for the add command
            let stdin_content = if atty::isnt(atty::Stream::Stdin) {
                let mut buffer = Vec::new();
                io::stdin().read_to_end(&mut buffer)?;
                buffer
            } else {
                Vec::new()
            };
            commands::add_note(&notes_dir, args, stdin_content).map(|_| ())
        }
        Some(Commands::Mcp(args)) => commands::mcp_command(&notes_dir, args),
        None => {
            // If no subcommand is provided, treat trailing args as 'add' command
            let add_args = AddArgs {
                title: cli.title,
                file: cli.file,
                args: cli.args,
            };

            // Only check stdin for the default add command
            let stdin_content = if atty::isnt(atty::Stream::Stdin) {
                let mut buffer = Vec::new();
                io::stdin().read_to_end(&mut buffer)?;
                buffer
            } else {
                Vec::new()
            };
            commands::add_note(&notes_dir, add_args, stdin_content).map(|_| ())
        }
    }
}
