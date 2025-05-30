//! Note implementation for notelog

use chrono::Local;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::core::frontmatter::Frontmatter;
use crate::core::tags::Tag;
use crate::error::{NotelogError, Result};
use crate::utils::{create_date_directories, generate_filename, validate_content};

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

    /// Get a mutable reference to the frontmatter of the note
    pub fn frontmatter_mut(&mut self) -> &mut Frontmatter {
        &mut self.frontmatter
    }

    /// Get the content of the note
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the formatted content with frontmatter
    ///
    /// This returns the complete note content with frontmatter and content properly formatted
    pub fn formatted_content(&self) -> String {
        format!("{}\n\n{}\n\n", self.frontmatter, self.content.trim_end())
    }

    /// Save the note to disk in the appropriate directory
    ///
    /// Returns the path to the saved note file, relative to the notes_dir
    pub fn save(&self, notes_dir: &Path, title_override: Option<&str>) -> Result<PathBuf> {
        let now = Local::now();

        // Create the year and month directories
        let month_dir = create_date_directories(notes_dir, &now)?;

        // Determine the title to use for the filename
        let title = match title_override {
            Some(title) => title.to_string(),
            None => self.extract_title(),
        };

        if title.is_empty() {
            // If the title is empty, there can be no content either.
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
        let final_content = self.formatted_content();

        // Write the note to the file
        let absolute_note_path = month_dir.join(&filename);
        fs::write(&absolute_note_path, final_content)?;

        // Convert the absolute path to a path relative to notes_dir
        let relative_path = absolute_note_path
            .strip_prefix(notes_dir)
            .map_err(|e| NotelogError::PathError(format!("Failed to create relative path: {}", e)))?
            .to_path_buf();

        // Return the relative path
        Ok(relative_path)
    }

    /// Extract tags as strings from the note
    pub fn tags_as_strings(&self) -> Vec<String> {
        self.frontmatter
            .tags()
            .iter()
            .map(|tag| tag.as_str().to_string())
            .collect()
    }

    /// Update the note's tags by adding and removing specified tags
    pub fn update_tags<I, J>(&mut self, tags_to_add: I, tags_to_remove: J)
    where
        I: IntoIterator<Item = Tag>,
        J: IntoIterator<Item = Tag>,
    {
        self.frontmatter_mut()
            .update_tags(tags_to_add, tags_to_remove);
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

        // Remove leading '#' characters (indicating a Markdown header) from the
        // title. If the note starts with a Markdown list indicated by "- " or
        // "* ", remove that as well.
        if title.starts_with('#') {
            title = title.trim_start_matches('#').trim().to_string();
        } else if title.starts_with("- ") {
            title = title
                .strip_prefix("- ")
                .unwrap_or(&title)
                .trim()
                .to_string();
        } else if title.starts_with("* ") {
            title = title
                .strip_prefix("* ")
                .unwrap_or(&title)
                .trim()
                .to_string();
        }

        // Truncate to 100 characters maximum
        if title.len() > 100 {
            title.truncate(100);
        }

        // Remove any trailing periods (so we don't end up with "Title..md")
        while title.ends_with('.') {
            title.pop();
        }

        title
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted_content())
    }
}

impl FromStr for Note {
    type Err = NotelogError;

    fn from_str(s: &str) -> Result<Self> {
        // First validate the content
        validate_content(s.as_bytes())?;

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
        // Id should appear first in the YAML
        assert!(result.starts_with("---\nid:"));
        assert!(result.contains("created:"));
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

        // Single dash prefix
        let content = "- This is a title\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This is a title");

        // Single asterisk prefix
        let content = "* This is a title\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This is a title");

        // Long title truncation
        let long_title = "A".repeat(150);
        let content = format!("# {}\nThis is the content", long_title);
        let note = Note::new(frontmatter.clone(), content);
        let extracted = note.extract_title();
        assert_eq!(extracted.len(), 100);
        assert_eq!(extracted, "A".repeat(100));

        // Single trailing period
        let content = "This title has a period.\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This title has a period");

        // Multiple trailing periods
        let content = "This title has multiple periods...\nThis is the content";
        let note = Note::new(frontmatter.clone(), content.to_string());
        assert_eq!(note.extract_title(), "This title has multiple periods");

        // Trailing period with markdown header
        let content = "# This is a header with period.\nThis is the content";
        let note = Note::new(frontmatter, content.to_string());
        assert_eq!(note.extract_title(), "This is a header with period");
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
        let relative_path = result.unwrap();
        let absolute_path = notes_dir.join(&relative_path);
        assert!(absolute_path.exists());

        // Verify the path is relative (doesn't start with the temp_dir path)
        assert!(!relative_path.is_absolute());

        // Verify the content
        let saved_content = fs::read_to_string(absolute_path).unwrap();
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

        // Get the relative path
        let relative_path = result.unwrap();

        // Verify the path is relative (doesn't start with the temp_dir path)
        assert!(!relative_path.is_absolute());

        // Verify the file was created with the custom title in the filename
        let path_str = relative_path.to_string_lossy();
        assert!(path_str.contains("Custom Title"));

        // Verify the content still has the original title
        let absolute_path = notes_dir.join(&relative_path);
        let saved_content = fs::read_to_string(absolute_path).unwrap();
        assert!(saved_content.contains("# Original Title"));
    }

    #[test]
    fn test_tags_as_strings() {
        // Create a note with tags
        let tag1 = Tag::new("+test").unwrap();
        let tag2 = Tag::new("+example").unwrap();
        let frontmatter = Frontmatter::with_tags(vec![tag1, tag2]);
        let content = "# Test Note\nThis is a test note.";
        let note = Note::new(frontmatter, content.to_string());

        // Get tags as strings
        let tags = note.tags_as_strings();

        // Verify the tags
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"test".to_string()));
        assert!(tags.contains(&"example".to_string()));
    }

    #[test]
    fn test_update_tags() {
        // Create a note with tags
        let tag1 = Tag::new("+test").unwrap();
        let tag2 = Tag::new("+example").unwrap();
        let frontmatter = Frontmatter::with_tags(vec![tag1.clone(), tag2.clone()]);
        let content = "# Test Note\nThis is a test note.";
        let mut note = Note::new(frontmatter, content.to_string());

        // Add and remove tags
        let tag3 = Tag::new("+new").unwrap();
        note.update_tags(vec![tag3.clone()], vec![tag1.clone()]);

        // Verify the tags
        let tags = note.tags_as_strings();
        assert_eq!(tags.len(), 2);
        assert!(!tags.contains(&"test".to_string()));
        assert!(tags.contains(&"example".to_string()));
        assert!(tags.contains(&"new".to_string()));
    }
}
