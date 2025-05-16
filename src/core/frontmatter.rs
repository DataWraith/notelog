//! Frontmatter implementation for notelog

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::core::id::Id;
use crate::core::tags::Tag;
use crate::error::{FrontmatterError, NotelogError, Result};

/// Represents the frontmatter of a note
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// The unique identifier for the note (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    /// The creation timestamp
    created: DateTime<Local>,
    /// The tags associated with the note
    tags: Vec<Tag>,
}

impl Frontmatter {
    /// Create a new frontmatter with the given creation timestamp and tags
    /// A random Id will be generated automatically
    pub fn new(created: DateTime<Local>, tags: Vec<Tag>) -> Self {
        Self {
            created,
            tags,
            id: Some(Id::default()),
        }
    }

    /// Create a new frontmatter with the current timestamp and given tags
    /// A random Id will be generated automatically
    pub fn with_tags(tags: Vec<Tag>) -> Self {
        Self::new(Local::now(), tags)
    }

    /// Create a new frontmatter with the current timestamp and no tags
    /// A random Id will be generated automatically
    pub fn default() -> Self {
        Self::with_tags(vec![])
    }

    /// Get the creation timestamp
    #[allow(dead_code)]
    pub fn created(&self) -> &DateTime<Local> {
        &self.created
    }

    /// Get the tags
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// Get the id if present
    pub fn id(&self) -> Option<&Id> {
        self.id.as_ref()
    }

    /// Add a tag to the frontmatter
    pub fn add_tag(&mut self, tag: Tag) {
        if self.tags.contains(&tag) {
            return;
        }

        self.tags.push(tag);
    }

    /// Apply frontmatter to content
    pub fn apply_to_content(&self, content: &str) -> String {
        format!("{}\n\n{}\n\n", self.to_yaml(), content)
    }

    /// Extract frontmatter from content if present
    pub fn extract_from_content(content: &str) -> Result<(Option<Self>, String)> {
        // Extract YAML and content
        match Self::extract_yaml_and_content(content) {
            Ok((Some(yaml), content_without_frontmatter)) => {
                // Parse the YAML
                match Self::from_str(&yaml) {
                    Ok(frontmatter) => Ok((Some(frontmatter), content_without_frontmatter)),
                    Err(e) => Err(e),
                }
            }
            Ok((None, content_without_frontmatter)) => {
                // No frontmatter or empty frontmatter
                Ok((None, content_without_frontmatter))
            }
            Err(e) => Err(e),
        }
    }

    /// Format the frontmatter as a YAML string
    pub fn to_yaml(&self) -> String {
        // Add id
        let id_yaml = if let Some(id) = &self.id {
            format!("id: {}\n", id)
        } else {
            String::new()
        };

        // Format with one-second precision (no fractional seconds)
        let created_yaml = self.created.format("created: %Y-%m-%dT%H:%M:%S%:z\n");

        // Format tags for YAML, omitting the tags array if it's empty
        let tags_yaml = if !self.tags.is_empty() {
            let mut yaml = String::from("\ntags:");
            for tag in &self.tags {
                yaml.push_str(&format!("\n  - {}", tag));
            }
            yaml
        } else {
            String::new()
        };

        format!("---\n{}{}{}\n---", id_yaml, created_yaml, tags_yaml)
    }

    /// Helper function to extract YAML frontmatter and content from a document
    fn extract_yaml_and_content(content: &str) -> Result<(Option<String>, String)> {
        // Check if the content starts with frontmatter
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return Ok((None, content.to_string()));
        }

        // Check if there's a closing frontmatter delimiter
        if let Some(rest) = trimmed.strip_prefix("---") {
            if let Some(end_index) = rest.find("\n---") {
                // Check if the frontmatter block is empty
                let frontmatter_content = &rest[..end_index];
                if frontmatter_content.trim().is_empty() {
                    // Empty frontmatter, return content after it
                    let after_frontmatter = &rest[end_index + 4..]; // +4 to skip "\n---"
                    return Ok((None, after_frontmatter.trim_start().to_string()));
                }

                // Extract the frontmatter and content
                let yaml = frontmatter_content.trim().to_string();
                let after_frontmatter = &rest[end_index + 4..]; // +4 to skip "\n---"
                return Ok((Some(yaml), after_frontmatter.trim_start().to_string()));
            } else {
                // No closing delimiter, not valid frontmatter
                return Ok((None, content.to_string()));
            }
        }

        // Should not reach here, but just in case
        Ok((None, content.to_string()))
    }
}

impl fmt::Display for Frontmatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_yaml())
    }
}

/// Serializable/deserializable frontmatter data structure
#[derive(Serialize, Deserialize, Debug)]
struct FrontmatterData {
    #[serde(default)]
    id: Option<String>,
    created: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl FromStr for Frontmatter {
    type Err = NotelogError;

    fn from_str(yaml: &str) -> Result<Self> {
        // Parse the YAML
        let frontmatter_data: FrontmatterData = match serde_yaml::from_str(yaml) {
            Ok(data) => data,
            Err(e) => return Err(FrontmatterError::InvalidYaml(e.to_string()).into()),
        };

        // Validate and convert the created timestamp
        let created = match chrono::DateTime::parse_from_rfc3339(&frontmatter_data.created) {
            Ok(dt) => dt.with_timezone(&Local),
            Err(e) => return Err(FrontmatterError::InvalidTimestamp(e.to_string()).into()),
        };

        // Convert string tags to Tag objects
        let mut tags = Vec::new();
        for tag_str in &frontmatter_data.tags {
            match Tag::new(tag_str) {
                Ok(tag) => tags.push(tag),
                Err(e) => return Err(e),
            }
        }

        // Parse the id if present
        let id = if let Some(id_str) = frontmatter_data.id {
            match Id::from_str(&id_str) {
                Ok(id) => Some(id),
                Err(e) => return Err(e),
            }
        } else {
            None
        };

        Ok(Self { created, tags, id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_frontmatter_struct_creation() {
        // Test creating a new Frontmatter with specific date and tags
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let tag1 = Tag::new("foo").unwrap();
        let tag2 = Tag::new("bar").unwrap();
        let tags = vec![tag1.clone(), tag2.clone()];

        // Test new constructor
        let frontmatter = Frontmatter::new(date.clone(), tags.clone());

        assert_eq!(frontmatter.created(), &date);
        assert_eq!(frontmatter.tags().len(), 2);
        assert_eq!(frontmatter.tags()[0], tag1);
        assert_eq!(frontmatter.tags()[1], tag2);
        assert!(frontmatter.id().is_some()); // Random Id should be generated

        // Test with_tags constructor
        let frontmatter = Frontmatter::with_tags(tags.clone());

        // We can't directly compare timestamps due to the small time difference
        // between creation and assertion, so we'll just check the tags and id
        assert_eq!(frontmatter.tags().len(), 2);
        assert_eq!(frontmatter.tags()[0], tag1);
        assert_eq!(frontmatter.tags()[1], tag2);
        assert!(frontmatter.id().is_some()); // Random Id should be generated

        // Test default constructor
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.tags().len(), 0);
        assert!(frontmatter.id().is_some()); // Random Id should be generated

        // Test creating with empty tags
        let frontmatter = Frontmatter::with_tags(vec![]);
        assert_eq!(frontmatter.tags().len(), 0);
        assert!(frontmatter.id().is_some()); // Random Id should be generated

        // Test that Ids are unique
        let frontmatter1 = Frontmatter::default();
        let frontmatter2 = Frontmatter::default();
        assert_ne!(frontmatter1.id(), frontmatter2.id()); // Ids should be different
    }

    #[test]
    fn test_frontmatter_add_tag() {
        // Test adding a tag to an empty frontmatter
        let mut frontmatter = Frontmatter::default();
        let tag = Tag::new("test").unwrap();
        frontmatter.add_tag(tag.clone());

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0], tag);

        // Test adding a second tag
        let tag2 = Tag::new("another").unwrap();
        frontmatter.add_tag(tag2.clone());

        assert_eq!(frontmatter.tags().len(), 2);
        assert_eq!(frontmatter.tags()[0], tag);
        assert_eq!(frontmatter.tags()[1], tag2);
    }

    #[test]
    fn test_frontmatter_add_duplicate_tags() {
        // Test adding duplicate tags
        let mut frontmatter = Frontmatter::default();

        // Create tags a, b, a, b, c
        let tag_a1 = Tag::new("a").unwrap();
        let tag_b1 = Tag::new("b").unwrap();
        let tag_a2 = Tag::new("a").unwrap(); // Duplicate of a
        let tag_b2 = Tag::new("b").unwrap(); // Duplicate of b
        let tag_c = Tag::new("c").unwrap();

        // Add all tags
        frontmatter.add_tag(tag_a1.clone());
        frontmatter.add_tag(tag_b1.clone());
        frontmatter.add_tag(tag_a2.clone()); // Should be ignored as duplicate
        frontmatter.add_tag(tag_b2.clone()); // Should be ignored as duplicate
        frontmatter.add_tag(tag_c.clone());

        // Verify we only have 3 unique tags: a, b, c
        assert_eq!(frontmatter.tags().len(), 3);
        assert_eq!(frontmatter.tags()[0].as_str(), "a");
        assert_eq!(frontmatter.tags()[1].as_str(), "b");
        assert_eq!(frontmatter.tags()[2].as_str(), "c");

        // Try adding a duplicate again
        frontmatter.add_tag(tag_a1.clone());

        // Verify count still remains at 3
        assert_eq!(frontmatter.tags().len(), 3);
    }

    #[test]
    fn test_frontmatter_to_yaml() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let tag1 = Tag::new("foo").unwrap();
        let tag2 = Tag::new("bar").unwrap();
        let tags = vec![tag1, tag2];

        // Create a frontmatter with a specific ID for testing
        let id = Id::new("0123456789abcdef").unwrap();
        let frontmatter = Frontmatter {
            created: date.clone(),
            tags: tags.clone(),
            id: Some(id.clone()),
        };

        let yaml = frontmatter.to_yaml();

        // Id should appear first in the YAML
        assert!(yaml.starts_with("---\nid: 0123456789abcdef\n"));
        assert!(yaml.contains("created: 2025-04-01T12:00:00"));
        assert!(yaml.contains("tags:\n  - foo\n  - bar"));
        assert!(yaml.ends_with("---"));

        // Test with no tags
        let frontmatter = Frontmatter {
            created: date.clone(),
            tags: vec![],
            id: Some(id.clone()),
        };
        let yaml = frontmatter.to_yaml();

        assert!(yaml.starts_with("---\nid: 0123456789abcdef\n"));
        assert!(yaml.contains("created: 2025-04-01T12:00:00"));
        assert!(!yaml.contains("tags:"));
        assert!(yaml.ends_with("---"));

        // Test with auto-generated ID
        let frontmatter = Frontmatter::new(date, vec![]);
        let yaml = frontmatter.to_yaml();

        assert!(yaml.contains("\nid: "));
        assert!(yaml.contains("created: 2025-04-01T12:00:00"));
        assert!(!yaml.contains("tags:"));
        assert!(yaml.ends_with("---"));
    }

    #[test]
    fn test_frontmatter_apply_to_content() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let tag = Tag::new("test").unwrap();
        let id = Id::new("0123456789abcdef").unwrap();

        // Create a frontmatter with a specific ID for testing
        let frontmatter = Frontmatter {
            created: date.clone(),
            tags: vec![tag.clone()],
            id: Some(id.clone()),
        };

        let content = "# Test Content\nThis is a test.";
        let result = frontmatter.apply_to_content(content);

        // Id should appear first in the YAML
        assert!(result.contains("---\nid: 0123456789abcdef\n"));
        assert!(result.contains("created: 2025-04-01T12:00:00"));
        assert!(result.contains("tags:\n  - test"));
        assert!(result.contains("---\n\n# Test Content\nThis is a test.\n\n"));
    }

    #[test]
    fn test_frontmatter_extract_from_content() {
        // Valid frontmatter with tags
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---\n\n# Content";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_some());
        let frontmatter = result.0.unwrap();
        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");
        assert!(frontmatter.id().is_none());
        assert_eq!(result.1, "# Content");

        // Valid frontmatter with id
        let content =
            "---\nid: 0123456789abcdef\ncreated: 2025-04-01T12:00:00+00:00\n---\n\n# Content";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_some());
        let frontmatter = result.0.unwrap();
        assert_eq!(frontmatter.tags().len(), 0);
        assert!(frontmatter.id().is_some());
        assert_eq!(frontmatter.id().unwrap().as_str(), "0123456789abcdef");
        assert_eq!(result.1, "# Content");

        // Valid frontmatter with tags and id
        let content = "---\nid: 0123456789abcdef\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---\n\n# Content";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_some());
        let frontmatter = result.0.unwrap();
        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");
        assert!(frontmatter.id().is_some());
        assert_eq!(frontmatter.id().unwrap().as_str(), "0123456789abcdef");
        assert_eq!(result.1, "# Content");

        // No frontmatter
        let content = "# Just content\nNo frontmatter here";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_none());
        assert_eq!(result.1, content);

        // Invalid frontmatter
        let content = "---\ncreated: invalid-date\ntags:\n  - test\n---\n\n# Content";
        assert!(Frontmatter::extract_from_content(content).is_err());

        // Invalid id in frontmatter
        let content = "---\nid: invalid-id\ncreated: 2025-04-01T12:00:00+00:00\n---\n\n# Content";
        assert!(Frontmatter::extract_from_content(content).is_err());
    }

    #[test]
    fn test_frontmatter_from_str() {
        // Valid YAML with tags
        let yaml = "created: 2025-04-01T12:00:00+00:00\ntags:\n  - test";
        let frontmatter = yaml.parse::<Frontmatter>().unwrap();

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");
        assert!(frontmatter.id().is_none());

        // Valid YAML with id
        let yaml = "id: 0123456789abcdef\ncreated: 2025-04-01T12:00:00+00:00";
        let frontmatter = yaml.parse::<Frontmatter>().unwrap();

        assert_eq!(frontmatter.tags().len(), 0);
        assert!(frontmatter.id().is_some());
        assert_eq!(frontmatter.id().unwrap().as_str(), "0123456789abcdef");

        // Valid YAML with tags and id
        let yaml = "id: 0123456789abcdef\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test";
        let frontmatter = yaml.parse::<Frontmatter>().unwrap();

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");
        assert!(frontmatter.id().is_some());
        assert_eq!(frontmatter.id().unwrap().as_str(), "0123456789abcdef");

        // Invalid YAML
        let yaml = "created: invalid-date\ntags:\n  - test";
        assert!(yaml.parse::<Frontmatter>().is_err());

        // Missing required field
        let yaml = "tags:\n  - test";
        assert!(yaml.parse::<Frontmatter>().is_err());

        // Invalid id
        let yaml = "id: invalid-id\ncreated: 2025-04-01T12:00:00+00:00";
        assert!(yaml.parse::<Frontmatter>().is_err());
    }
}
