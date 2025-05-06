use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Datelike, Local};
use dirs::home_dir;
use tempfile::NamedTempFile;

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
    let mut title = content
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
pub fn generate_frontmatter(content: &str, created: &DateTime<Local>) -> String {
    // Format with one-second precision (no fractional seconds)
    let created_iso = created.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    // For now, use placeholder tags
    format!(
        "---\ncreated: {}\ntags: \n  - tag1\n  - tag2\n  - tag3\n---\n\n{}\n\n",
        created_iso, content
    )
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
        .arg(temp_file.path())
        .status()
        .map_err(|e| NotelogError::EditorLaunchFailed(format!("{}: {}", editor, e)))?;

    if !status.success() {
        return Err(NotelogError::EditorLaunchFailed(
            format!("{} exited with status {}", editor, status)
        ));
    }

    // Read the content back from the file
    let mut content = String::new();
    temp_file.reopen()?.read_to_string(&mut content)?;

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
        let frontmatter = generate_frontmatter(content, &date);

        // Check for the timestamp format with one-second precision
        // (the timezone part will vary, so we don't check the exact format)
        assert!(frontmatter.starts_with("---\ncreated: 2025-04-01T12:00:00"));
        assert!(frontmatter.contains("tags: \n  - tag1\n  - tag2\n  - tag3"));
        assert!(frontmatter.contains("---\n\n# Test Title\nThis is the content\n\n"));
    }
}
