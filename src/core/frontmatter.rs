//! Frontmatter implementation for notelog

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::core::tags::Tag;
use crate::error::{FrontmatterError, NotelogError, Result};

/// Represents the frontmatter of a note
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter {
    /// The creation timestamp
    created: DateTime<Local>,
    /// The tags associated with the note
    tags: Vec<Tag>,
}

impl Frontmatter {
    /// Create a new frontmatter with the given creation timestamp and tags
    pub fn new(created: DateTime<Local>, tags: Vec<Tag>) -> Self {
        Self { created, tags }
    }

    /// Create a new frontmatter with the current timestamp and given tags
    pub fn with_tags(tags: Vec<Tag>) -> Self {
        Self::new(Local::now(), tags)
    }

    /// Create a new frontmatter with the current timestamp and no tags
    pub fn default() -> Self {
        Self::with_tags(vec![])
    }

    /// Get the creation timestamp
    pub fn created(&self) -> &DateTime<Local> {
        &self.created
    }

    /// Get the tags
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// Format the frontmatter as a YAML string
    pub fn to_yaml(&self) -> String {
        // Format with one-second precision (no fractional seconds)
        let created_iso = self.created.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

        // Format tags for YAML
        let tags_yaml = if self.tags.is_empty() {
            String::from("tags:\n  - edit-me")
        } else {
            let mut yaml = String::from("tags:");
            for tag in &self.tags {
                yaml.push_str(&format!("\n  - {}", tag));
            }
            yaml
        };

        format!("---\ncreated: {}\n{}\n---", created_iso, tags_yaml)
    }

    /// Apply frontmatter to content
    pub fn apply_to_content(&self, content: &str) -> String {
        format!("{}\n\n{}\n\n", self.to_yaml(), content)
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
}

impl fmt::Display for Frontmatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_yaml())
    }
}

/// Serializable/deserializable frontmatter data structure
#[derive(Serialize, Deserialize, Debug)]
struct FrontmatterData {
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

        Ok(Self::new(created, tags))
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

        let frontmatter = Frontmatter::new(date.clone(), tags.clone());

        assert_eq!(frontmatter.created(), &date);
        assert_eq!(frontmatter.tags().len(), 2);
        assert_eq!(frontmatter.tags()[0], tag1);
        assert_eq!(frontmatter.tags()[1], tag2);

        // Test with_tags constructor
        let frontmatter = Frontmatter::with_tags(tags.clone());

        // We can't directly compare timestamps due to the small time difference
        // between creation and assertion, so we'll just check the tags
        assert_eq!(frontmatter.tags().len(), 2);
        assert_eq!(frontmatter.tags()[0], tag1);
        assert_eq!(frontmatter.tags()[1], tag2);

        // Test default constructor
        let frontmatter = Frontmatter::default();
        assert_eq!(frontmatter.tags().len(), 0);

        // Test creating with empty tags
        let frontmatter = Frontmatter::with_tags(vec![]);
        assert_eq!(frontmatter.tags().len(), 0);
    }

    #[test]
    fn test_frontmatter_to_yaml() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let tag1 = Tag::new("foo").unwrap();
        let tag2 = Tag::new("bar").unwrap();
        let tags = vec![tag1, tag2];

        let frontmatter = Frontmatter::new(date.clone(), tags);
        let yaml = frontmatter.to_yaml();

        assert!(yaml.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(yaml.contains("tags:\n  - foo\n  - bar"));
        assert!(yaml.ends_with("---"));

        // Test with empty tags
        let frontmatter = Frontmatter::new(date, vec![]);
        let yaml = frontmatter.to_yaml();

        assert!(yaml.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(yaml.contains("tags:\n  - edit-me"));
        assert!(yaml.ends_with("---"));
    }

    #[test]
    fn test_frontmatter_apply_to_content() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let tag = Tag::new("test").unwrap();
        let frontmatter = Frontmatter::new(date, vec![tag]);

        let content = "# Test Content\nThis is a test.";
        let result = frontmatter.apply_to_content(content);

        assert!(result.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(result.contains("tags:\n  - test"));
        assert!(result.contains("---\n\n# Test Content\nThis is a test.\n\n"));
    }

    #[test]
    fn test_frontmatter_extract_from_content() {
        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---\n\n# Content";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_some());
        let frontmatter = result.0.unwrap();
        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");
        assert_eq!(result.1, "# Content");

        // No frontmatter
        let content = "# Just content\nNo frontmatter here";
        let result = Frontmatter::extract_from_content(content).unwrap();

        assert!(result.0.is_none());
        assert_eq!(result.1, content);

        // Invalid frontmatter
        let content = "---\ncreated: invalid-date\ntags:\n  - test\n---\n\n# Content";
        assert!(Frontmatter::extract_from_content(content).is_err());
    }

    #[test]
    fn test_frontmatter_from_str() {
        // Valid YAML
        let yaml = "created: 2025-04-01T12:00:00+00:00\ntags:\n  - test";
        let frontmatter = yaml.parse::<Frontmatter>().unwrap();

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");

        // Invalid YAML
        let yaml = "created: invalid-date\ntags:\n  - test";
        assert!(yaml.parse::<Frontmatter>().is_err());

        // Missing required field
        let yaml = "tags:\n  - test";
        assert!(yaml.parse::<Frontmatter>().is_err());
    }


}
