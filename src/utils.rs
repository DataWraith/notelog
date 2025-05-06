use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Datelike, Local};
use dirs::home_dir;
use serde::Deserialize;
use tempfile::NamedTempFile;
use yaml_front_matter::YamlFrontMatter;

use crate::error::{NotelogError, Result};

/// Determine the notes directory from the provided path, environment variable, or default
pub fn get_notes_dir(notes_dir: Option<PathBuf>) -> Result<PathBuf> {
    notes_dir
        .or_else(|| env::var("NOTELOG_DIR").map(PathBuf::from).ok())
        .or_else(|| home_dir().map(|p| p.join("NoteLog")))
        .ok_or_else(|| NotelogError::NotesDirectoryNotFound("Could not determine home directory".to_string()))
}

/// Ensure the notes directory exists and is writable
pub fn ensure_notes_dir_exists(notes_dir: &Path) -> Result<()> {
    if !notes_dir.exists() {
        return Err(NotelogError::NotesDirectoryNotFound(
            format!("Directory does not exist: {}", notes_dir.display())
        ));
    } else if !notes_dir.is_dir() {
        return Err(NotelogError::NotesDirectoryNotFound(
            format!("{} is not a directory", notes_dir.display())
        ));
    }

    // Check if the directory is writable by attempting to create a temporary file
    let temp_file_path = notes_dir.join(".notelog_write_test");
    match File::create(&temp_file_path) {
        Ok(_) => {
            // Clean up the test file
            let _ = fs::remove_file(temp_file_path);
            Ok(())
        },
        Err(e) => Err(NotelogError::NotesDirectoryNotWritable(
            format!("{}: {}", notes_dir.display(), e)
        )),
    }
}

/// Create the year and month directories for the note
pub fn create_date_directories(notes_dir: &Path, date: &DateTime<Local>) -> Result<PathBuf> {
    let year = date.year();
    let month = date.month();
    let month_name = match month {
        1 => "01 January",
        2 => "02 February",
        3 => "03 March",
        4 => "04 April",
        5 => "05 May",
        6 => "06 June",
        7 => "07 July",
        8 => "08 August",
        9 => "09 September",
        10 => "10 October",
        11 => "11 November",
        12 => "12 December",
        _ => unreachable!(),
    };

    let year_dir = notes_dir.join(year.to_string());
    let month_dir = year_dir.join(month_name);

    fs::create_dir_all(&month_dir)?;

    Ok(month_dir)
}

/// Generate a valid filename from a title
pub fn generate_filename(date: &DateTime<Local>, title: &str, counter: Option<usize>) -> String {
    let date_str = date.format("%Y-%m-%d").to_string();

    // Sanitize the title for use in a filename
    let sanitized_title = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>();

    // Add counter if provided
    if let Some(counter) = counter {
        format!("{} ({}) {}.md", date_str, counter, sanitized_title)
    } else {
        format!("{} {}.md", date_str, sanitized_title)
    }
}

/// Extract title from note content
pub fn extract_title(content: &str) -> String {
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

/// Check if a string is a valid tag
pub fn validate_tag(tag: &str) -> Result<String> {
    // Remove the '+' prefix if present
    let tag = tag.strip_prefix('+').unwrap_or(tag).to_lowercase();

    // Check if tag is empty
    if tag.is_empty() {
        return Err(NotelogError::InvalidTag("Tag cannot be empty".to_string()));
    }

    // Check if tag starts or ends with a dash
    if tag.starts_with('-') || tag.ends_with('-') {
        return Err(NotelogError::InvalidTag(
            format!("Tag '{}' cannot start or end with a dash", tag)
        ));
    }

    // Check if tag contains only valid characters (a-z, 0-9, -)
    if !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(NotelogError::InvalidTag(
            format!("Tag '{}' can only contain lowercase letters, numbers, and dashes", tag)
        ));
    }

    Ok(tag)
}

/// Extract tags from command line arguments
pub fn extract_tags_from_args(args: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    let mut tags = Vec::new();
    let mut non_tag_args = Vec::new();

    for arg in args {
        if arg.starts_with('+') {
            match validate_tag(arg) {
                Ok(tag) => tags.push(tag),
                Err(e) => return Err(e),
            }
        } else {
            non_tag_args.push(arg.clone());
        }
    }

    Ok((tags, non_tag_args))
}

/// Check if content is valid
pub fn validate_content(content: &[u8]) -> Result<()> {
    // Check if content is too large (> 50KiB)
    if content.len() > 50 * 1024 {
        return Err(NotelogError::ContentTooLarge);
    }

    // Check if content is empty
    if content.is_empty() || content.iter().all(|&b| b.is_ascii_whitespace()) {
        return Err(NotelogError::EmptyContent);
    }

    // Check if content contains null bytes
    if content.contains(&0) {
        return Err(NotelogError::ContentContainsNullBytes);
    }

    Ok(())
}

/// Open an editor for the user to write a note
pub fn open_editor(initial_content: Option<&str>) -> Result<String> {
    // Create a temporary file with .md extension
    let mut temp_file = NamedTempFile::with_suffix(".md")?;
    let temp_path = temp_file.path().to_path_buf();

    // Write initial content if provided
    if let Some(content) = initial_content {
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
    }

    // Get the editor command
    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "nano".to_string());

    // Launch the editor
    let status = Command::new(&editor)
        .arg(&temp_path)
        .status()
        .map_err(|e| NotelogError::EditorLaunchFailed(format!("{}: {}", editor, e)))?;

    if !status.success() {
        return Err(NotelogError::EditorLaunchFailed(
            format!("{} exited with status {}", editor, status)
        ));
    }

    // Read the content back from the file.
    // Uses the path directly instead of reopening the temporary file,
    // because the editor may replace the file instead of modifying it in place
    let mut content = String::new();
    File::open(&temp_path)?.read_to_string(&mut content)?;

    // The temporary file will be automatically deleted when temp_file goes out of scope

    Ok(content)
}

/// Read content from a file
pub fn read_file_content(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    validate_content(&content)?;

    String::from_utf8(content)
        .map_err(|_| NotelogError::InvalidUtf8Content)
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

/// Wait for user to press Enter or Ctrl+C
pub fn wait_for_user_input() -> Result<bool> {
    println!("Press Enter to continue or Ctrl+C to exit...");

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => Ok(true),  // User pressed Enter
        Err(e) => Err(NotelogError::Io(e)),  // Error reading input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use crate::error::NotelogError;

    #[test]
    fn test_extract_title_from_plain_text() {
        let content = "This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");
    }

    #[test]
    fn test_extract_title_from_markdown() {
        let content = "# This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");
    }

    #[test]
    fn test_extract_title_with_multiple_hashes() {
        let content = "### This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");
    }

    #[test]
    fn test_extract_title_with_empty_lines() {
        let content = "\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");
    }

    #[test]
    fn test_extract_title_truncates_long_titles() {
        let long_title = "A".repeat(150);
        let content = format!("# {}\nThis is the content", long_title);
        let extracted = extract_title(&content);
        assert_eq!(extracted.len(), 100);
        assert_eq!(extracted, "A".repeat(100));
    }

    #[test]
    fn test_extract_title_with_frontmatter() {
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");
    }

    #[test]
    fn test_extract_title_with_frontmatter_no_title() {
        let content = "---\ncreated: 2025-04-01T12:00:00+00:00\ntags:\n  - tag1\n---\n\nThis is the content without a title";
        assert_eq!(extract_title(content), "This is the content without a title");
    }

    #[test]
    fn test_generate_filename() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let title = "Test Title";
        assert_eq!(
            generate_filename(&date, title, None),
            "2025-04-01 Test Title.md"
        );
    }

    #[test]
    fn test_generate_filename_with_counter() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let title = "Test Title";
        assert_eq!(
            generate_filename(&date, title, Some(1)),
            "2025-04-01 (1) Test Title.md"
        );
    }

    #[test]
    fn test_generate_filename_sanitizes_title() {
        let date = Local.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        let title = "Test/Title:With*Invalid?Chars";
        assert_eq!(
            generate_filename(&date, title, None),
            "2025-04-01 Test-Title-With-Invalid-Chars.md"
        );
    }

    #[test]
    fn test_validate_content_empty() {
        let content = b"";
        assert!(matches!(validate_content(content), Err(NotelogError::EmptyContent)));

        let content = b"   \n   ";
        assert!(matches!(validate_content(content), Err(NotelogError::EmptyContent)));
    }

    #[test]
    fn test_validate_content_too_large() {
        let content = vec![b'a'; 51 * 1024]; // 51KiB
        assert!(matches!(validate_content(&content), Err(NotelogError::ContentTooLarge)));
    }

    #[test]
    fn test_validate_content_null_bytes() {
        let content = b"Test\0Content";
        assert!(matches!(validate_content(content), Err(NotelogError::ContentContainsNullBytes)));
    }

    #[test]
    fn test_validate_content_valid() {
        let content = b"This is valid content";
        assert!(validate_content(content).is_ok());
    }

    #[test]
    fn test_invalid_utf8_conversion() {
        // Create invalid UTF-8 sequence
        let invalid_utf8 = vec![0xFF, 0xFF, 0xFF, 0xFF];

        // Test with String::from_utf8 conversion
        let result = String::from_utf8(invalid_utf8)
            .map_err(|_| NotelogError::InvalidUtf8Content);

        assert!(matches!(result, Err(NotelogError::InvalidUtf8Content)));
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
    fn test_validate_tag() {
        // Valid tags
        assert_eq!(validate_tag("+foo").unwrap(), "foo");
        assert_eq!(validate_tag("+foo-bar").unwrap(), "foo-bar");
        assert_eq!(validate_tag("+123").unwrap(), "123");
        assert_eq!(validate_tag("+foo123").unwrap(), "foo123");
        assert_eq!(validate_tag("+FOO").unwrap(), "foo");

        // Invalid tags
        assert!(validate_tag("+").is_err());
        assert!(validate_tag("+-foo").is_err());
        assert!(validate_tag("+foo-").is_err());
        assert!(validate_tag("+foo_bar").is_err());
        assert!(validate_tag("+foo bar").is_err());
    }

    #[test]
    fn test_extract_tags_from_args() {
        // Test with no tags
        let args = vec!["foo".to_string(), "bar".to_string()];
        let (tags, non_tags) = extract_tags_from_args(&args).unwrap();
        assert!(tags.is_empty());
        assert_eq!(non_tags, args);

        // Test with tags
        let args = vec!["+foo".to_string(), "bar".to_string(), "+baz".to_string()];
        let (tags, non_tags) = extract_tags_from_args(&args).unwrap();
        assert_eq!(tags, vec!["foo", "baz"]);
        assert_eq!(non_tags, vec!["bar"]);

        // Test with invalid tag
        let args = vec!["+foo".to_string(), "+foo-".to_string()];
        assert!(extract_tags_from_args(&args).is_err());
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
    fn test_extract_title_with_empty_frontmatter() {
        let content = "---\n---\n\n# This is a title\nThis is the content";
        assert_eq!(extract_title(content), "This is a title");

        let content = "---\n---\nThis is the content without a title";
        assert_eq!(extract_title(content), "This is the content without a title");
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
