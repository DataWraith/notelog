use std::path::Path;
use std::str::FromStr;

use crate::cli::AddArgs;
use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::extract_tags_from_args;
use crate::error::{NotelogError, Result};
use crate::utils::{
    open_editor, read_file_content, validate_content, wait_for_user_input,
};

/// Create a note from various input sources and save it
///
/// Returns the path to the created note file on success
pub fn add_note(notes_dir: &Path, args: AddArgs, stdin_content: Vec<u8>) -> Result<String> {
    // Create a note from the input
    let (note, title_override) = create_note_from_input(args, stdin_content)?;

    // Save the note to disk
    let note_path = note.save(notes_dir, title_override.as_deref())?;

    // Print success message
    println!("Note saved to: {}", note_path);

    // Return the path
    Ok(note_path)
}

/// Create a Note object from various input sources
///
/// Returns a tuple of (Note, Option<String>) where the second element is an optional title override
pub fn create_note_from_input(args: AddArgs, stdin_content: Vec<u8>) -> Result<(Note, Option<String>)> {
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
        String::from_utf8(stdin_content).map_err(|_| NotelogError::InvalidUtf8Content)?
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
                let base_content = args
                    .title
                    .as_ref()
                    .map(|t| format!("# {}", t))
                    .unwrap_or_default();
                // When opening the editor, use default tag if no tags provided
                // This makes it easier for users to add tags
                let frontmatter = Frontmatter::default();
                frontmatter.apply_to_content(&base_content)
            };

            content = open_editor(Some(&editor_content))?;

            // Check if the content is completely blank
            if content.trim().is_empty() {
                println!("Note is empty. Exiting without saving.");
                return Err(NotelogError::EmptyContent);
            }

            // Check if content has frontmatter and validate it
            match Note::from_str(&content) {
                Ok(_) => {
                    // Note is valid (either has valid frontmatter or no frontmatter)
                    break;
                }
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
                            return Err(NotelogError::UserCancelled);
                        }
                    }
                }
            }
        }

        content
    };

    validate_content(content.as_bytes())?;

    // Get the title override if provided
    let title_override = args.title.clone();

    // Create the note object
    let note = match Note::from_str(&content) {
        Ok(note) => {
            if note.frontmatter().tags().is_empty() && !tags.is_empty() {
                // Note has no tags but we have tags from command line
                let frontmatter = Frontmatter::with_tags(tags);
                Note::new(frontmatter, note.content().to_string())
            } else {
                // Note already has valid frontmatter or no tags specified
                note
            }
        }
        _ => {
            // Invalid frontmatter, add a new one
            let frontmatter = Frontmatter::with_tags(tags);
            Note::new(frontmatter, content)
        }
    };

    Ok((note, title_override))
}
