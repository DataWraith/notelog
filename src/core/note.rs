//! Note implementation for notelog

use std::str::FromStr;

use crate::core::frontmatter::Frontmatter;
use crate::error::{NotelogError, Result};

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
        format!("{}\n\n{}\n\n", self.frontmatter, self.content)
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
        assert_eq!(note.frontmatter().tags().len(), 1); // Default tag
        assert_eq!(note.frontmatter().tags()[0].as_str(), "log");
        assert_eq!(note.content(), content);

        // Empty frontmatter
        let content = "---\n---\nContent";
        let note = Note::from_str(content).unwrap();
        assert_eq!(note.frontmatter().tags().len(), 1); // Default tag
        assert_eq!(note.frontmatter().tags()[0].as_str(), "log");
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
        assert!(result.contains("tags:\n  - log"));
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
}
