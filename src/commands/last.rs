use std::collections::BinaryHeap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::cli::LastArgs;
use crate::core::note::Note;
use crate::error::{NotelogError, Result};
use crate::utils::{is_valid_note_file, open_editor, read_file_content};

/// Find and open the newest note
pub fn last_note(notes_dir: &Path, args: LastArgs) -> Result<()> {
    // Check if any options were provided that are not allowed
    if args.title.is_some() || args.file.is_some() || !args.args.is_empty() {
        return Err(NotelogError::InvalidLastOptions);
    }

    // Find the newest note.
    let newest_note_path = find_newest_note(notes_dir)?;

    // Either print the note or open it in the editor
    if args.print {
        // Read and print the note content
        let content = read_file_content(&newest_note_path)?;
        println!("{}", content);
    } else {
        // Read the note content
        let content = read_file_content(&newest_note_path)?;

        // Parse the note to validate it
        let _note = Note::from_str(&content)?;

        // Open the note in the editor
        let new_content = open_editor(Some(&content))?;

        // If the content has changed, save it back to the file
        if new_content != content {
            fs::write(&newest_note_path, new_content)?;
            println!("Note updated: {}", newest_note_path.display());
        }
    }

    Ok(())
}

/// Find the newest note in the notes directory
///
/// Searches for the last year in the notes directory, then the last month in
/// that directory, and then the last note in that directory. Usually it should
/// return quickly, because it does not descend deeper into the tree than
/// necessary.
fn find_newest_note(notes_dir: &Path) -> Result<PathBuf> {
    let mut year_dirs = get_year_dirs(notes_dir)?;

    while let Some(year_dir) = year_dirs.pop() {
        let mut month_dirs = get_month_dirs(&year_dir)?;

        while let Some(month_dir) = month_dirs.pop() {
            let note_file = get_last_note_file(&month_dir)?;

            if note_file.is_some() {
                return Ok(note_file.unwrap());
            }
        }
    }

    // If we get here, no valid note was found
    Err(NotelogError::NoValidNoteFound)
}

/// Get all year directories sorted by name
fn get_year_dirs(notes_dir: &Path) -> Result<BinaryHeap<PathBuf>> {
    let mut year_dirs = BinaryHeap::new();

    // Read the notes directory
    let entries = fs::read_dir(notes_dir)?;

    // Filter for year directories (4-digit numbers)
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                // Check if the name is a 4-digit year
                if name_str.len() == 4 && name_str.chars().all(|c| c.is_ascii_digit()) {
                    year_dirs.push(path);
                }
            }
        }
    }

    Ok(year_dirs)
}

/// Get all month directories sorted by name
fn get_month_dirs(year_dir: &Path) -> Result<BinaryHeap<PathBuf>> {
    let mut month_dirs = BinaryHeap::new();

    // Read the year directory
    let entries = fs::read_dir(year_dir)?;

    // Filter for month directories (starting with 01-12)
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                // Check if the name starts with a valid month number (01-12)
                if name_str.len() >= 2 {
                    let month_prefix = &name_str[..2];
                    if let Ok(month_num) = month_prefix.parse::<u32>() {
                        if (1..=12).contains(&month_num) {
                            month_dirs.push(path);
                        }
                    }
                }
            }
        }
    }

    Ok(month_dirs)
}

/// Get the note file with the largest path (which should be the newest one,
/// since note paths include the date and time)
fn get_last_note_file(month_dir: &Path) -> Result<Option<PathBuf>> {
    let mut note_file = None;

    // Read the month directory
    let entries = fs::read_dir(month_dir)?;

    // Filter for valid note files
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            // Use the utility function to check if it's a valid note file
            if is_valid_note_file(&path)? {
                note_file = note_file.max(Some(path));
            }
        }
    }

    Ok(note_file)
}
