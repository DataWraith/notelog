use std::path::Path;
use std::str::FromStr;

use crate::cli::AddArgs;
use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::extract_tags_from_args;
use crate::error::{NotelogError, Result};
use crate::utils::{open_editor, read_file_content, validate_content, wait_for_user_input};

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
pub fn create_note_from_input(
    args: AddArgs,
    stdin_content: Vec<u8>,
) -> Result<(Note, Option<String>)> {
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
        let content = non_tag_args.join(" ");

        // If a title is provided and the content doesn't start with a markdown header,
        // add a markdown header with the title
        if let Some(title) = &args.title {
            // Check if the content already has a markdown header
            if !content.trim_start().starts_with('#') {
                return Ok((
                    Note::new(
                        Frontmatter::with_tags(tags),
                        format!("# {}\n\n{}", title, content),
                    ),
                    Some(title.clone()),
                ));
            }
        }

        content
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::AddArgs;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_note_from_stdin() {
        // Test with content from stdin
        let args = AddArgs {
            args: vec![],
            file: None,
            title: None,
        };
        let stdin_content = "This is a test note from stdin".as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, title_override) = result;

        assert_eq!(note.content(), "This is a test note from stdin");
        assert!(title_override.is_none());
        assert!(note.frontmatter().tags().is_empty());
    }

    #[test]
    fn test_create_note_from_stdin_with_tags() {
        // Test with content from stdin and tags in args
        let args = AddArgs {
            args: vec!["+test".to_string(), "+tag2".to_string()],
            file: None,
            title: None,
        };
        let stdin_content = "This is a test note with tags".as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, _) = result;

        // Check that the content is preserved
        assert_eq!(note.content(), "This is a test note with tags");

        // Check that the tags from args were applied
        let tags = note.frontmatter().tags();
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t.as_str() == "test"));
        assert!(tags.iter().any(|t| t.as_str() == "tag2"));
    }

    #[test]
    fn test_create_note_from_stdin_with_file() {
        // Test with content from stdin and file (should error)
        let args = AddArgs {
            args: vec![],
            file: Some(PathBuf::from("test.txt")),
            title: None,
        };
        let stdin_content = "This is a test note".as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NotelogError::ConflictingInputMethods));
    }

    #[test]
    fn test_create_note_from_file() -> Result<()> {
        // Create a temporary file with test content
        let mut temp_file = NamedTempFile::new()?;
        use std::io::Write;
        writeln!(temp_file, "This is a test note from a file")?;

        let args = AddArgs {
            args: vec![],
            file: Some(temp_file.path().to_path_buf()),
            title: None,
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content)?;
        let (note, title_override) = result;

        assert!(note.content().contains("This is a test note from a file"));
        assert!(title_override.is_none());
        assert!(note.frontmatter().tags().is_empty());

        Ok(())
    }

    #[test]
    fn test_create_note_from_file_with_args() {
        // Test with content from file and non-tag args (should error)
        let args = AddArgs {
            args: vec!["some".to_string(), "args".to_string()],
            file: Some(PathBuf::from("test.txt")),
            title: None,
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NotelogError::ConflictingInputMethods));
    }

    #[test]
    fn test_create_note_from_args() {
        // Test with content from command line arguments
        let args = AddArgs {
            args: vec!["This".to_string(), "is".to_string(), "a".to_string(), "test".to_string(), "note".to_string()],
            file: None,
            title: None,
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, title_override) = result;

        assert_eq!(note.content(), "This is a test note");
        assert!(title_override.is_none());
        assert!(note.frontmatter().tags().is_empty());
    }

    #[test]
    fn test_create_note_from_args_with_tags() {
        // Test with content and tags from command line arguments
        let args = AddArgs {
            args: vec![
                "This".to_string(),
                "is".to_string(),
                "a".to_string(),
                "+test".to_string(),
                "note".to_string(),
                "+tag2".to_string()
            ],
            file: None,
            title: None,
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, title_override) = result;

        assert_eq!(note.content(), "This is a note");
        assert!(title_override.is_none());

        // Check that tags were extracted correctly
        let tags = note.frontmatter().tags();
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().any(|t| t.as_str() == "test"));
        assert!(tags.iter().any(|t| t.as_str() == "tag2"));
    }

    #[test]
    fn test_create_note_with_title_override() {
        // Test with title override
        let args = AddArgs {
            args: vec!["This".to_string(), "is".to_string(), "a".to_string(), "test".to_string()],
            file: None,
            title: Some("Custom Title".to_string()),
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, title_override) = result;

        // Content should now include a markdown header with the title
        assert_eq!(note.content(), "# Custom Title\n\nThis is a test");
        assert_eq!(title_override, Some("Custom Title".to_string()));
    }

    #[test]
    fn test_create_note_with_title_override_existing_header() {
        // Test with title override when content already has a header
        let args = AddArgs {
            args: vec!["#".to_string(), "Existing".to_string(), "Header".to_string(), "content".to_string()],
            file: None,
            title: Some("Custom Title".to_string()),
        };
        let stdin_content = vec![];

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, title_override) = result;

        // Content should remain unchanged since it already has a header
        assert_eq!(note.content(), "# Existing Header content");
        assert_eq!(title_override, Some("Custom Title".to_string()));
    }

    #[test]
    fn test_create_note_with_frontmatter_in_content() {
        // Test with content that already has frontmatter
        let content = r#"---
created: 2025-04-01T12:00:00+00:00
tags:
  - existing
---

# Note with existing frontmatter"#;

        let args = AddArgs {
            args: vec![],
            file: None,
            title: None,
        };
        let stdin_content = content.as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, _) = result;

        assert_eq!(note.content(), "# Note with existing frontmatter");

        // Check that the existing frontmatter was preserved
        let tags = note.frontmatter().tags();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "existing");
    }

    #[test]
    fn test_create_note_with_frontmatter_and_command_line_tags() {
        // Test with content that has frontmatter and additional tags from command line
        let content = r#"---
created: 2025-04-01T12:00:00+00:00
tags:
  - existing
---

# Note with existing frontmatter"#;

        let args = AddArgs {
            args: vec!["+cli-tag".to_string()],
            file: None,
            title: None,
        };
        let stdin_content = content.as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, _) = result;

        // Check that the content is preserved
        assert_eq!(note.content(), "# Note with existing frontmatter");

        // Check that the existing frontmatter tags are preserved (not replaced by command line tags)
        // since the note already has tags
        let tags = note.frontmatter().tags();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "existing");
    }

    #[test]
    fn test_create_note_with_empty_frontmatter_and_command_line_tags() {
        // Test with content that has empty frontmatter and tags from command line
        let content = r#"---
created: 2025-04-01T12:00:00+00:00
tags: []
---

# Note with empty tags"#;

        let args = AddArgs {
            args: vec!["+cli-tag".to_string()],
            file: None,
            title: None,
        };
        let stdin_content = content.as_bytes().to_vec();

        let result = create_note_from_input(args, stdin_content).unwrap();
        let (note, _) = result;

        // Check that the content is preserved
        assert_eq!(note.content(), "# Note with empty tags");

        // Check that the command line tags were applied since the note has empty tags
        let tags = note.frontmatter().tags();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "cli-tag");
    }
}
