//! NoteBuilder implementation for notelog

use chrono::{DateTime, Local};
use std::str::FromStr;

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::Tag;
use crate::error::Result;
use crate::utils::validate_content;

/// A builder for creating Note objects with various options
#[derive(Debug, Clone)]
pub struct NoteBuilder {
    /// The frontmatter of the note
    frontmatter: Option<Frontmatter>,
    /// The content of the note
    content: String,
    /// Tags to add to the note
    tags: Vec<Tag>,
    /// The creation timestamp
    created: Option<DateTime<Local>>,
    /// Whether to validate the content
    validate: bool,
    /// Title override for saving
    title_override: Option<String>,
}

impl NoteBuilder {
    /// Create a new NoteBuilder with empty content
    pub fn new() -> Self {
        Self {
            frontmatter: None,
            content: String::new(),
            tags: Vec::new(),
            created: None,
            validate: true,
            title_override: None,
        }
    }

    /// Set the creation timestamp
    #[cfg(test)]
    pub fn created(mut self, created: DateTime<Local>) -> Self {
        self.created = Some(created);
        self
    }

    /// Set the frontmatter directly
    #[cfg(test)]
    pub fn frontmatter(mut self, frontmatter: Frontmatter) -> Self {
        self.frontmatter = Some(frontmatter);
        self
    }

    /// Set the content of the note
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.content = content.into();
        self
    }

    /// Add a single tag to the note
    pub fn tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add multiple tags to the note
    pub fn tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = Tag>,
    {
        self.tags.extend(tags);
        self
    }

    /// Set whether to validate the content
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Set a title override for saving
    pub fn title_override<S: Into<String>>(mut self, title: S) -> Self {
        self.title_override = Some(title.into());
        self
    }

    /// Build the Note object
    pub fn build(self) -> Result<Note> {
        // Validate the content if requested
        if self.validate {
            validate_content(self.content.as_bytes())?;
        }

        // Create the frontmatter
        let frontmatter = match self.frontmatter {
            Some(mut fm) => {
                // If we have frontmatter but also have tags, add them to the frontmatter
                if !self.tags.is_empty() {
                    for tag in self.tags {
                        fm.add_tag(tag);
                    }
                }

                fm
            }
            None => {
                // Create new frontmatter with the provided tags and timestamp
                let created = self.created.unwrap_or_else(Local::now);
                Frontmatter::new(created, self.tags)
            }
        };

        Ok(Note::new(frontmatter, self.content))
    }

    /// Try to parse content as a note, falling back to creating a new note if parsing fails
    pub fn parse_or_create(self) -> Result<Note> {
        match Note::from_str(&self.content) {
            Ok(mut note) => {
                // If we have tags, add them to the note
                for tag in self.tags {
                    note.frontmatter_mut().add_tag(tag);
                }
                Ok(note)
            }
            Err(_) => {
                // If parsing fails, create a new note with the content
                // TODO: Is this really the best way to handle this?
                self.build()
            }
        }
    }
}

impl Default for NoteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_builder_basic() {
        let content = "# Test Note\nThis is a test note.";
        let note = NoteBuilder::new().content(content).build().unwrap();

        assert_eq!(note.content(), content);
        assert_eq!(note.frontmatter().tags().len(), 0);
    }

    #[test]
    fn test_note_builder_with_tags() {
        let content = "# Test Note\nThis is a test note.";
        let tag1 = Tag::new("+test").unwrap();
        let tag2 = Tag::new("+example").unwrap();

        let note = NoteBuilder::new()
            .content(content)
            .tag(tag1)
            .tag(tag2)
            .build()
            .unwrap();

        assert_eq!(note.content(), content);
        assert_eq!(note.frontmatter().tags().len(), 2);
        assert_eq!(note.frontmatter().tags()[0].as_str(), "test");
        assert_eq!(note.frontmatter().tags()[1].as_str(), "example");
    }

    #[test]
    fn test_note_builder_with_created_timestamp() {
        let content = "# Test Note\nThis is a test note.";
        let created = Local::now();

        let note = NoteBuilder::new()
            .content(content)
            .created(created)
            .build()
            .unwrap();

        assert_eq!(note.content(), content);
        assert_eq!(note.frontmatter().created(), &created);
    }

    #[test]
    fn test_note_builder_with_frontmatter() {
        let content = "# Test Note\nThis is a test note.";
        let frontmatter = Frontmatter::default();

        let note = NoteBuilder::new()
            .content(content)
            .frontmatter(frontmatter)
            .build()
            .unwrap();

        assert_eq!(note.content(), content);
        assert!(note.frontmatter().id().is_some());
    }

    #[test]
    fn test_note_builder_parse_or_create() {
        // Content with frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---\n\n# Content";
        let note = NoteBuilder::new()
            .content(content)
            .parse_or_create()
            .unwrap();

        assert_eq!(note.frontmatter().tags().len(), 1);
        assert_eq!(note.frontmatter().tags()[0].as_str(), "test");
        assert_eq!(note.content(), "# Content");

        // Content without frontmatter
        let content = "# Just content\nNo frontmatter here";
        let tag = Tag::new("+example").unwrap();
        let note = NoteBuilder::new()
            .content(content)
            .tag(tag)
            .parse_or_create()
            .unwrap();

        assert_eq!(note.frontmatter().tags().len(), 1);
        assert_eq!(note.frontmatter().tags()[0].as_str(), "example");
        assert_eq!(note.content(), content);
    }

    #[test]
    fn test_note_builder_parse_or_create_with_additional_tags() {
        // Content with frontmatter and tags
        let content =
            "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - existing\n---\n\n# Content";
        let tag = Tag::new("+new").unwrap();

        let note = NoteBuilder::new()
            .content(content)
            .tag(tag)
            .parse_or_create()
            .unwrap();

        // Should have both the existing tag and the new tag
        assert_eq!(note.frontmatter().tags().len(), 2);
        assert!(
            note.frontmatter()
                .tags()
                .iter()
                .any(|t| t.as_str() == "existing")
        );
        assert!(
            note.frontmatter()
                .tags()
                .iter()
                .any(|t| t.as_str() == "new")
        );
    }

    #[test]
    fn test_note_builder_validation() {
        // Test with invalid content (empty)
        let result = NoteBuilder::new().content("").validate(true).build();

        assert!(result.is_err());

        // Test with validation disabled
        let result = NoteBuilder::new().content("").validate(false).build();

        assert!(result.is_ok());
    }
}
