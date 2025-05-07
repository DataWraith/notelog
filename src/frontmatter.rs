use std::convert::TryFrom;
use std::fmt;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use yaml_front_matter::YamlFrontMatter;

use crate::error::{FrontmatterError, NotelogError, Result};
use crate::tags::Tag;

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

    /// Create a new frontmatter with the current timestamp and default tags
    pub fn default() -> Self {
        let default_tag = Tag::new("log").expect("Default tag 'log' should be valid");
        Self::with_tags(vec![default_tag])
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
            String::from("tags: []")
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

    /// Extract frontmatter from content if present
    pub fn extract_from_content(content: &str) -> Result<(Option<Self>, String)> {
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
            } else {
                // No closing delimiter, not valid frontmatter
                return Ok((None, content.to_string()));
            }
        }

        // Try to parse the frontmatter
        let result = YamlFrontMatter::parse::<FrontmatterData>(content);
        match result {
            Ok(parsed) => {
                let frontmatter_data = parsed.metadata;
                let content = parsed.content.trim_start().to_string();

                // Convert the parsed data to our Frontmatter struct
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

                Ok((Some(Self::new(created, tags)), content))
            },
            Err(e) => Err(FrontmatterError::InvalidYaml(e.to_string()).into()),
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

impl TryFrom<String> for Frontmatter {
    type Error = NotelogError;

    fn try_from(yaml: String) -> Result<Self> {
        // Add YAML delimiters if not present
        let yaml_with_delimiters = if yaml.trim_start().starts_with("---") {
            yaml
        } else {
            format!("---\n{}\n---", yaml)
        };

        // Try to parse the YAML
        let result = YamlFrontMatter::parse::<FrontmatterData>(&yaml_with_delimiters);
        match result {
            Ok(parsed) => {
                let frontmatter_data = parsed.metadata;

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
            },
            Err(e) => Err(FrontmatterError::InvalidYaml(e.to_string()).into()),
        }
    }
}

/// Extract title from note content, handling frontmatter if present
pub fn extract_title_from_content_with_frontmatter(content: &str) -> String {
    // Skip frontmatter if present
    let content_without_frontmatter = match Frontmatter::extract_from_content(content) {
        Ok((_, content_after_frontmatter)) => content_after_frontmatter,
        Err(_) => content.to_string(), // If there's an error parsing frontmatter, use the original content
    };

    // Find the first non-empty line in the content (after frontmatter if present)
    let mut title = content_without_frontmatter
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::convert::TryFrom;

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
        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "log");

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
        assert!(yaml.contains("tags: []"));
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
    fn test_frontmatter_try_from_string() {
        // Valid YAML
        let yaml = "created: 2025-04-01T12:00:00+00:00\ntags:\n  - test";
        let frontmatter = Frontmatter::try_from(yaml.to_string()).unwrap();

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");

        // Already has delimiters
        let yaml = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - test\n---";
        let frontmatter = Frontmatter::try_from(yaml.to_string()).unwrap();

        assert_eq!(frontmatter.tags().len(), 1);
        assert_eq!(frontmatter.tags()[0].as_str(), "test");

        // Invalid YAML
        let yaml = "created: invalid-date\ntags:\n  - test";
        assert!(Frontmatter::try_from(yaml.to_string()).is_err());

        // Missing required field
        let yaml = "tags:\n  - test";
        assert!(Frontmatter::try_from(yaml.to_string()).is_err());
    }

    #[test]
    fn test_extract_title_from_content_with_frontmatter() {
        // Plain text
        let content = "This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        // Markdown
        let content = "# This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        // Multiple hashes
        let content = "### This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        // Empty lines
        let content = "\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        // Long title truncation
        let long_title = "A".repeat(150);
        let content = format!("# {}\nThis is the content", long_title);
        let extracted = extract_title_from_content_with_frontmatter(&content);
        assert_eq!(extracted.len(), 100);
        assert_eq!(extracted, "A".repeat(100));

        // With frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        // With frontmatter, no title
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nThis is the content without a title";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is the content without a title");

        // With empty frontmatter
        let content = "---\n---\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is a title");

        let content = "---\n---\nThis is the content without a title";
        assert_eq!(extract_title_from_content_with_frontmatter(content), "This is the content without a title");
    }

    #[test]
    fn test_frontmatter_with_different_tags() {
        let content = "# Test Title\nThis is the content";
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();

        // Test with no tags
        let frontmatter = Frontmatter::new(date.clone(), vec![]);
        let result = frontmatter.apply_to_content(content);
        assert!(result.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(result.contains("tags: []"));
        assert!(result.contains("---\n\n# Test Title\nThis is the content\n\n"));

        // Test with custom tags
        let tag1 = Tag::new("foo").unwrap();
        let tag2 = Tag::new("bar").unwrap();
        let tags = vec![tag1, tag2];
        let frontmatter = Frontmatter::new(date.clone(), tags);
        let result = frontmatter.apply_to_content(content);
        assert!(result.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(result.contains("tags:\n  - foo\n  - bar"));
        assert!(result.contains("---\n\n# Test Title\nThis is the content\n\n"));
    }

    #[test]
    fn test_frontmatter_detection() {
        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_some());

        // No frontmatter
        let content = "# Just a title\nNo frontmatter here";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, content);

        // Starts with --- but no closing ---
        let content = "---\nThis is not valid frontmatter";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, content);

        // Empty frontmatter
        let content = "---\n---\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, "Content");

        // With whitespace before frontmatter
        let content = "\n\n  ---\ncreated: 2025-04-01T12:00:00+00:00\n---\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_some());
    }

    #[test]
    fn test_empty_frontmatter_handling() {
        // Empty frontmatter
        let content = "---\n---\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, "Content");

        // Empty frontmatter with whitespace
        let content = "---\n   \n---\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, "Content");

        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags: \n  - tag1\n---\n\nContent";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_some());

        // No frontmatter
        let content = "# Just a title\nNo frontmatter here";
        let result = Frontmatter::extract_from_content(content).unwrap();
        assert!(result.0.is_none());
        assert_eq!(result.1, content);
    }

    #[test]
    fn test_frontmatter_validation() {
        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nContent";
        assert!(Frontmatter::extract_from_content(content).is_ok());

        // No frontmatter (should be ok, as we'll add it later)
        let content = "# Just a title\nNo frontmatter here";
        assert!(Frontmatter::extract_from_content(content).is_ok());

        // Invalid YAML in frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags: invalid yaml\n---\n\nContent";
        assert!(Frontmatter::extract_from_content(content).is_err());

        // Missing required field
        let content = "---\ntags:\n  - tag1\n---\n\nContent";
        assert!(Frontmatter::extract_from_content(content).is_err());
    }
}
