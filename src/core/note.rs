//! Note implementation for notelog

use chrono::Local;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use crate::core::frontmatter::Frontmatter;
use crate::error::{NotelogError, Result};
use crate::utils::{create_date_directories, generate_filename};

/// Represents a complete note with frontmatter and content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The frontmatter of the note
    frontmatter: Frontmatter,
    /// The content of the note
    content: String,
}

impl Note {
    /// Create a new note with the given frontmatter and content
    pub fn new(frontmatter: Frontmatter, content: String) -> Self {
        Self {
            frontmatter,
            content,
        }
    }

    /// Get the frontmatter of the note
    pub fn frontmatter(&self) -> &Frontmatter {
        &self.frontmatter
    }

    /// Get the content of the note
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Convert the note to a string with frontmatter and content
    pub fn to_string(&self) -> String {
        format!("{}\n\n{}\n\n", self.frontmatter, self.content.trim_end())
    }

    /// Extract title from the note content
    pub fn extract_title(&self) -> String {
        // Find the first non-empty line in the content
        let mut title = self
            .content
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();

        // Remove leading '#' characters and trim
        if title.starts_with('#') {
            title = title.trim_start_matches('#').trim().to_string();
        }

        // Truncate to 100 characters maximum
        if title.len() > 100 {
            title.truncate(100);
        }

        title
    }

    /// Save the note to disk in the appropriate directory
    ///
    /// Returns the path to the saved note file
    pub fn save(&self, notes_dir: &Path, title_override: Option<&str>) -> Result<String> {
        // Get the current date and time
        let now = Local::now();

        // Create the year and month directories
        let month_dir = create_date_directories(notes_dir, &now)?;

        // Determine the title to use for the filename
        let title = match title_override {
            Some(title) => title.to_string(),
            None => self.extract_title(),
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

        // Get the full content with frontmatter
        let final_content = self.to_string();

        // Write the note to the file
        let note_path = month_dir.join(&filename);
        fs::write(&note_path, final_content)?;

        // Return the path as a string
        Ok(note_path.to_string_lossy().to_string())
    }
}

impl FromStr for Note {
    type Err = NotelogError;

    fn from_str(s: &str) -> Result<Self> {
        // Use Frontmatter::extract_from_content to parse the frontmatter
        match Frontmatter::extract_from_content(s) {
            Ok((Some(frontmatter), content)) => {
                // Valid frontmatter found
                Ok(Self {
                    frontmatter,
                    content,
                })
            }
            Ok((None, content)) => {
                // No frontmatter or empty frontmatter, use default
                Ok(Self {
                    frontmatter: Frontmatter::default(),
                    content,
                })
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_note_from_str() {
        // Valid note with frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---\n\n# Content";
        let note = Note::from_str(content).unwrap();
        assert_eq!(note.frontmatter().tags().len(), 1);
        assert_eq!(note.frontmatter().tags()[0].as_str(), "test");
        assert_eq!(note.content(), "# Content");

        // No frontmatter
        let content = "# Just content\nNo frontmatter here";
        let note = Note::from_str(content).unwrap();
        assert_eq!(note.frontmatter().tags().len(), 0); // No default tag
        assert_eq!(note.content(), content);

        // Empty frontmatter
        let content = "---\n---\nContent";
        let note = Note::from_str(content).unwrap();
        assert_eq!(note.frontmatter().tags().len(), 0); // No default tag
        assert_eq!(note.content(), "Content");

        // Invalid YAML in frontmatter
        let content = "---\ncreated: invalid-date\ntags:\n  - test\n---\n\n# Content";
        assert!(Note::from_str(content).is_err());
    }

    #[test]
    fn test_note_to_string() {
        let frontmatter = Frontmatter::default();
        let content = "# Test Content";
        let note = Note::new(frontmatter, content.to_string());

        let result = note.to_string();
        assert!(result.starts_with("---\ncreated:"));
        // Empty tags array should be omitted
        assert!(!result.contains("tags:"));
        assert!(result.contains("---\n\n# Test Content\n\n"));
    }

    #[test]
    fn test_extract_title() {
        // Plain text
        let frontmatter = Frontmatter::default();
        let content = "This is a title\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This is a title");

        // Markdown
        let content = "# This is a title\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This is a title");

        // Multiple hashes
        let content = "### This is a title\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This is a title");

        // Long title truncation
        let long_title = "A".repeat(150);
        let content = format!("# {}\nThis is the content", long_title);
        let note = Note::new(frontmatter, content);
        let extracted = note.extract_title();
        assert_eq!(extracted.len(), 100);
        assert_eq!(extracted, "A".repeat(100));
    }

    #[test]
    fn test_save() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a note
        let frontmatter = Frontmatter::default();
        let content = "# Test Save\nThis is a test of the save method.";
        let note = Note::new(frontmatter, content.to_string());

        // Save the note
        let result = note.save(notes_dir, None);
        assert!(result.is_ok());

        // Verify the file was created
        let path = result.unwrap();
        let path = Path::new(&path);
        assert!(path.exists());

        // Verify the content
        let saved_content = fs::read_to_string(path).unwrap();
        assert!(saved_content.contains("# Test Save"));
        assert!(saved_content.contains("This is a test of the save method."));
        // Empty tags array should be omitted
        assert!(!saved_content.contains("tags:"));
    }

    #[test]
    fn test_save_with_title_override() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a note
        let frontmatter = Frontmatter::default();
        let content = "# Original Title\nThis is a test of the save method with title override.";
        let note = Note::new(frontmatter, content.to_string());

        // Save the note with a title override
        let result = note.save(notes_dir, Some("Custom Title"));
        assert!(result.is_ok());

        // Verify the file was created with the custom title in the filename
        let path = result.unwrap();
        assert!(path.contains("Custom Title"));

        // Verify the content still has the original title
        let path = Path::new(&path);
        let saved_content = fs::read_to_string(path).unwrap();
        assert!(saved_content.contains("# Original Title"));
    }
}
