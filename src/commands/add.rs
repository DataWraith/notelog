use std::fs;
use std::path::Path;

use chrono::Local;

use crate::cli::AddArgs;
use crate::error::{NotelogError, Result};
use crate::utils::{
    create_date_directories, extract_title, generate_filename, generate_frontmatter,
    has_frontmatter, open_editor, read_file_content, validate_content,
};

/// Add a new note
pub fn add_note(notes_dir: &Path, args: AddArgs, stdin_content: Vec<u8>) -> Result<()> {
    // Get the current date and time
    let now = Local::now();

    // Create the year and month directories
    let month_dir = create_date_directories(notes_dir, &now)?;

    // Determine the note content
    let content = if !stdin_content.is_empty() {
        // Content from stdin
        if !args.args.is_empty() {
            return Err(NotelogError::ConflictingStdinAndArgs);
        }
        if args.file.is_some() {
            return Err(NotelogError::ConflictingInputMethods);
        }

        validate_content(&stdin_content)?;
        String::from_utf8(stdin_content)
            .map_err(|_| NotelogError::InvalidUtf8Content)?
    } else if let Some(file_path) = &args.file {
        // Content from file
        if !args.args.is_empty() {
            return Err(NotelogError::ConflictingInputMethods);
        }

        read_file_content(file_path)?
    } else if !args.args.is_empty() {
        // Content from command line arguments
        args.args.join(" ")
    } else {
        // Open an editor with frontmatter
        let now = Local::now();
        let base_content = args.title.as_ref().map(|t| format!("# {}", t)).unwrap_or_default();
        let content_with_frontmatter = generate_frontmatter(&base_content, &now);
        open_editor(Some(&content_with_frontmatter))?
    };

    validate_content(content.as_bytes())?;

    // Determine the title
    let title = match &args.title {
        Some(title) => title.clone(),
        None => extract_title(&content),
    };

    if title.is_empty() {
        return Err(NotelogError::EmptyContent);
    }

    // Generate the filename
    let mut filename = generate_filename(&now, &title, None);
    let mut counter = 1;

    // Check for filename collisions
    while month_dir.join(&filename).exists() {
        filename = generate_filename(&now, &title, Some(counter));
        counter += 1;
    }

    // Add frontmatter to the content if it doesn't already have it
    let final_content = if has_frontmatter(&content) {
        // Content already has frontmatter (from editor)
        content
    } else {
        // Add frontmatter
        generate_frontmatter(&content, &now)
    };

    // Write the note to the file
    let note_path = month_dir.join(filename);
    fs::write(&note_path, final_content)?;

    println!("Note saved to: {}", note_path.display());

    Ok(())
}
