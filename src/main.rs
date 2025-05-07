mod cli;
mod commands;
mod error;
mod frontmatter;
mod utils;

use std::io::{self, Read};

use atty;
use clap::Parser;

use cli::{AddArgs, Cli, Commands};
use error::Result;
use utils::{ensure_notes_dir_exists, get_notes_dir};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine the notes directory
    let notes_dir = get_notes_dir(cli.notes_dir)?;

    // Ensure the notes directory exists and is writable
    ensure_notes_dir_exists(&notes_dir)?;

    // Check if we have data on stdin
    let stdin_content = if atty::isnt(atty::Stream::Stdin) {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    } else {
        Vec::new()
    };

    // Handle the command (or default to 'add')
    match cli.command {
        Some(Commands::Add(args)) => commands::add_note(&notes_dir, args, stdin_content),
        None => {
            // If no subcommand is provided, treat trailing args as 'add' command
            let add_args = AddArgs {
                title: cli.title,
                file: cli.file,
                args: cli.args,
            };
            commands::add_note(&notes_dir, add_args, stdin_content)
        }
    }
}
