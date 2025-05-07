use chrono::{DateTime, Local};
use serde::Deserialize;
use yaml_front_matter::YamlFrontMatter;

use crate::error::{NotelogError, Result};

/// Generate YAML frontmatter for a note
pub fn generate_frontmatter(content: &str, created: &DateTime<Local>, tags: Option<&Vec<String>>) -> String {
    // Format with one-second precision (no fractional seconds)
    let created_iso = created.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    // Format tags for YAML
    let tags_yaml = format_tags_for_frontmatter(tags);

    format!(
        "---\ncreated: {}\n{}\n---\n\n{}\n\n",
        created_iso, tags_yaml, content
    )
}

/// Format tags for YAML frontmatter
pub fn format_tags_for_frontmatter(tags: Option<&Vec<String>>) -> String {
    let default_tags = vec!["log".to_string()];
    let tags = tags.filter(|t| !t.is_empty()).unwrap_or(&default_tags);

    let mut tags_yaml = String::from("tags:");
    for tag in tags {
        tags_yaml.push_str(&format!("\n  - {}", tag));
    }

    tags_yaml
}

/// Check if content already has YAML frontmatter
pub fn has_frontmatter(content: &str) -> bool {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return false;
    }

    // Find the end of the frontmatter block
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end_index) = rest.find("\n---") {
            // Make sure there's something in the frontmatter block
            let frontmatter_content = &rest[..end_index];
            return !frontmatter_content.trim().is_empty();
        }
    }

    false
}

/// Check if content has empty frontmatter (---\n---)
pub fn has_empty_frontmatter(content: &str) -> bool {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return false;
    }

    // Find the end of the frontmatter block
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end_index) = rest.find("\n---") {
            // Check if the frontmatter block is empty
            let frontmatter_content = &rest[..end_index];
            return frontmatter_content.trim().is_empty();
        }
    }

    false
}

/// Remove empty frontmatter from content
pub fn remove_empty_frontmatter(content: &str) -> String {
    if !has_empty_frontmatter(content) {
        return content.to_string();
    }

    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end_index) = rest.find("\n---") {
            // Get content after the frontmatter
            let after_frontmatter = &rest[end_index + 4..]; // +4 to skip "\n---"
            return after_frontmatter.trim_start().to_string();
        }
    }

    content.to_string()
}

/// Basic frontmatter structure for validation
#[derive(Deserialize)]
struct FrontMatter {
    #[allow(dead_code)]
    created: String,
    #[allow(dead_code)]
    tags: Vec<String>,
}

/// Validate YAML frontmatter in content
pub fn validate_frontmatter(content: &str) -> Result<()> {
    if !has_frontmatter(content) {
        return Ok(());  // No frontmatter to validate
    }

    // Try to parse the frontmatter
    match YamlFrontMatter::parse::<FrontMatter>(content) {
        Ok(_) => Ok(()),
        Err(e) => Err(NotelogError::YamlParseError(e.to_string())),
    }
}

/// Extract title from note content, handling frontmatter if present
pub fn extract_title_from_content_with_frontmatter(content: &str) -> String {
    // Skip frontmatter if present (both valid and empty frontmatter)
    let content_without_frontmatter = if has_frontmatter(content) || has_empty_frontmatter(content) {
        // Find the end of the frontmatter block
        let trimmed = content.trim_start();
        if let Some(rest) = trimmed.strip_prefix("---") {
            if let Some(end_index) = rest.find("\n---") {
                // Get content after the frontmatter
                let after_frontmatter = &rest[end_index + 4..]; // +4 to skip "\n---"
                after_frontmatter.trim_start()
            } else {
                content
            }
        } else {
            content
        }
    } else {
        content
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
    fn test_generate_frontmatter() {
        let content = "# Test Title\nThis is the content";
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();

        // Test with default tags
        let frontmatter = generate_frontmatter(content, &date, None);
        assert!(frontmatter.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(frontmatter.contains("tags:\n  - log"));
        assert!(frontmatter.contains("---\n\n# Test Title\nThis is the content\n\n"));

        // Test with custom tags
        let tags = vec!["foo".to_string(), "bar".to_string()];
        let frontmatter = generate_frontmatter(content, &date, Some(&tags));
        assert!(frontmatter.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(frontmatter.contains("tags:\n  - foo\n  - bar"));
        assert!(frontmatter.contains("---\n\n# Test Title\nThis is the content\n\n"));
    }

    #[test]
    fn test_has_frontmatter() {
        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nContent";
        assert!(has_frontmatter(content));

        // No frontmatter
        let content = "# Just a title\nNo frontmatter here";
        assert!(!has_frontmatter(content));

        // Starts with --- but no closing ---
        let content = "---\nThis is not valid frontmatter";
        assert!(!has_frontmatter(content));

        // Empty frontmatter
        let content = "---\n---\nContent";
        assert!(!has_frontmatter(content));

        // With whitespace before frontmatter
        let content = "\n\n  ---\ncreated: 2025-04-01T12:00:00+00:00\n---\nContent";
        assert!(has_frontmatter(content));
    }

    #[test]
    fn test_has_empty_frontmatter() {
        // Empty frontmatter
        let content = "---\n---\nContent";
        assert!(has_empty_frontmatter(content));

        // Empty frontmatter with whitespace
        let content = "---\n   \n---\nContent";
        assert!(has_empty_frontmatter(content));

        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags: \n  - tag1\n---\n\nContent";
        assert!(!has_empty_frontmatter(content));

        // No frontmatter
        let content = "# Just a title\nNo frontmatter here";
        assert!(!has_empty_frontmatter(content));
    }

    #[test]
    fn test_remove_empty_frontmatter() {
        // Empty frontmatter
        let content = "---\n---\nContent";
        assert_eq!(remove_empty_frontmatter(content), "Content");

        // Empty frontmatter with whitespace
        let content = "---\n   \n---\nContent";
        assert_eq!(remove_empty_frontmatter(content), "Content");

        // Valid frontmatter (should not be removed)
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags: \n  - tag1\n---\n\nContent";
        assert_eq!(remove_empty_frontmatter(content), content);

        // No frontmatter
        let content = "# Just a title\nNo frontmatter here";
        assert_eq!(remove_empty_frontmatter(content), content);
    }

    #[test]
    fn test_validate_frontmatter() {
        // Valid frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nContent";
        assert!(validate_frontmatter(content).is_ok());

        // No frontmatter (should be ok, as we'll add it later)
        let content = "# Just a title\nNo frontmatter here";
        assert!(validate_frontmatter(content).is_ok());

        // Invalid YAML in frontmatter
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags: invalid yaml\n---\n\nContent";
        assert!(validate_frontmatter(content).is_err());

        // Missing required field
        let content = "---\ntags:\n  - tag1\n---\n\nContent";
        assert!(validate_frontmatter(content).is_err());
    }
}
