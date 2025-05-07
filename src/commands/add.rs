use std::fs;
use std::path::Path;

use chrono::Local;

use crate::cli::AddArgs;
use crate::error::{NotelogError, Result};
use crate::utils::{
    create_date_directories, extract_tags_from_args, extract_title, generate_filename, generate_frontmatter,
    has_empty_frontmatter, has_frontmatter, open_editor, read_file_content, remove_empty_frontmatter,
    validate_content, validate_frontmatter, wait_for_user_input,
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
            let now = Local::now();

            // For the first iteration, use the default initial content
            // For subsequent iterations, use the user's content (even if it has invalid YAML)
            let editor_content = if let Some(ref user_content) = initial_content {
                user_content.clone()
            } else {
                let base_content = args.title.as_ref().map(|t| format!("# {}", t)).unwrap_or_default();
                generate_frontmatter(&base_content, &now, Some(&tags))
            };

            content = open_editor(Some(&editor_content))?;

            // Check if the content is completely blank
            if content.trim().is_empty() {
                println!("Note is empty. Exiting without saving.");
                return Ok(());
            }

            // Check if content has frontmatter
            if !has_frontmatter(&content) {
                // No frontmatter or empty frontmatter, we'll add it later
                break;
            }

            // Validate the frontmatter
            match validate_frontmatter(&content) {
                Ok(_) => break,  // Frontmatter is valid
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
    let final_content = if has_frontmatter(&content) {
        // Content already has frontmatter (from editor)
        // We've already validated it above, so we can use it as is
        content
    } else if has_empty_frontmatter(&content) {
        // Empty frontmatter, remove it and add proper frontmatter
        let content_without_frontmatter = remove_empty_frontmatter(&content);
        generate_frontmatter(&content_without_frontmatter, &now, Some(&tags))
    } else {
        // No frontmatter, add it
        generate_frontmatter(&content, &now, Some(&tags))
    };

    // Write the note to the file
    let note_path = month_dir.join(filename);
    fs::write(&note_path, final_content)?;

    println!("Note saved to: {}", note_path.display());

    Ok(())
}
