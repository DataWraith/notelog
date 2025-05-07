use std::fs;
use std::path::Path;
use std::str::FromStr;

use chrono::Local;

use crate::cli::AddArgs;
use crate::error::{NotelogError, Result};
use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::extract_tags_from_args;
use crate::utils::{
    create_date_directories, extract_title, generate_filename, open_editor,
    read_file_content, validate_content, wait_for_user_input,
};

/// Add a new note
pub fn add_note(notes_dir: &Path, args: AddArgs, stdin_content: Vec<u8>) -> Result<()> {
    // Get the current date and time
    let now = Local::now();

    // Create the year and month directories
    let month_dir = create_date_directories(notes_dir, &now)?;

    // Extract tags from command line arguments
    let (tags, non_tag_args) = extract_tags_from_args(&args.args)?;

    // Determine the note content
    let content = if !stdin_content.is_empty() {
        // Content from stdin
        if !non_tag_args.is_empty() {
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
        if !non_tag_args.is_empty() {
            return Err(NotelogError::ConflictingInputMethods);
        }

        read_file_content(file_path)?
    } else if !non_tag_args.is_empty() {
        // Content from command line arguments
        non_tag_args.join(" ")
    } else {
        // Open an editor with frontmatter
        let mut content;
        let mut initial_content: Option<String> = None;

        loop {
            // For the first iteration, use the default initial content
            // For subsequent iterations, use the user's content (even if it has invalid YAML)
            let editor_content = if let Some(ref user_content) = initial_content {
                user_content.clone()
            } else {
                let base_content = args.title.as_ref().map(|t| format!("# {}", t)).unwrap_or_default();
                // When opening the editor, use default tag if no tags provided
                // This makes it easier for users to add tags
                let frontmatter = Frontmatter::default();
                frontmatter.apply_to_content(&base_content)
            };

            content = open_editor(Some(&editor_content))?;

            // Check if the content is completely blank
            if content.trim().is_empty() {
                println!("Note is empty. Exiting without saving.");
                return Ok(());
            }

            // Check if content has frontmatter and validate it
            match Note::from_str(&content) {
                Ok(_) => {
                    // Note is valid (either has valid frontmatter or no frontmatter)
                    break;
                },
                Err(e) => {
                    eprintln!("Error in YAML frontmatter: {}", e);

                    // Save the user's content for the next iteration
                    initial_content = Some(content.clone());

                    // Wait for user to press Enter or Ctrl+C
                    match wait_for_user_input() {
                        Ok(true) => {
                            // User pressed Enter, continue the loop to reopen the editor
                            println!("Reopening editor to fix frontmatter...");
                            continue;
                        }
                        _ => {
                            // User pressed Ctrl+C or there was an error
                            println!("Exiting without saving.");
                            return Ok(());
                        }
                    }
                }
            }
        }

        content
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
    let mut counter = 2;

    // Check for filename collisions
    while month_dir.join(&filename).exists() {
        filename = generate_filename(&now, &title, Some(counter));
        counter += 1;
    }

    // Add or regenerate frontmatter as needed
    let final_content = match Note::from_str(&content) {
        Ok(note) => {
            if note.frontmatter().tags().is_empty() && !tags.is_empty() {
                // Note has no tags but we have tags from command line
                let frontmatter = Frontmatter::with_tags(tags.clone());
                let note = Note::new(frontmatter, note.content().to_string());
                note.to_string()
            } else {
                // Note already has valid frontmatter or no tags specified
                content
            }
        },
        _ => {
            // Invalid frontmatter, add a new one
            let frontmatter = Frontmatter::with_tags(tags.clone());
            let note = Note::new(frontmatter, content);
            note.to_string()
        }
    };

    // Write the note to the file
    let note_path = month_dir.join(filename);
    fs::write(&note_path, final_content)?;

    println!("Note saved to: {}", note_path.display());

    Ok(())
}
