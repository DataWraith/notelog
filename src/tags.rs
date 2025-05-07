use std::fmt;
use crate::error::{NotelogError, Result, TagError};

/// An opaque wrapper type that represents a valid tag
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag from a string, validating it in the process
    pub fn new(input: &str) -> Result<Self> {
        // Remove the '+' prefix if present
        let tag = input.strip_prefix('+').unwrap_or(input).to_lowercase();

        // Check if tag is empty
        if tag.is_empty() {
            return Err(NotelogError::TagError(TagError::Empty));
        }

        // Check if tag starts or ends with a dash
        if tag.starts_with('-') || tag.ends_with('-') {
            return Err(NotelogError::TagError(TagError::InvalidDashPosition(tag)));
        }

        // Check if tag contains only valid characters (a-z, 0-9, -)
        if !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(NotelogError::TagError(TagError::InvalidCharacters(tag)));
        }

        Ok(Tag(tag))
    }

    /// Get the tag as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Extract tags from command line arguments
pub fn extract_tags_from_args(args: &[String]) -> Result<(Vec<Tag>, Vec<String>)> {
    let mut tags = Vec::new();
    let mut non_tag_args = Vec::new();

    for arg in args {
        if arg.starts_with('+') {
            match Tag::new(arg) {
                Ok(tag) => tags.push(tag),
                Err(e) => return Err(e),
            }
        } else {
            non_tag_args.push(arg.clone());
        }
    }

    Ok((tags, non_tag_args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        // Valid tags
        assert_eq!(Tag::new("+foo").unwrap().as_str(), "foo");
        assert_eq!(Tag::new("+foo-bar").unwrap().as_str(), "foo-bar");
        assert_eq!(Tag::new("+123").unwrap().as_str(), "123");
        assert_eq!(Tag::new("+foo123").unwrap().as_str(), "foo123");
        assert_eq!(Tag::new("+FOO").unwrap().as_str(), "foo");

        // Invalid tags
        assert!(matches!(Tag::new("+").unwrap_err(), NotelogError::TagError(TagError::Empty)));
        assert!(matches!(Tag::new("+-foo").unwrap_err(), NotelogError::TagError(TagError::InvalidDashPosition(_))));
        assert!(matches!(Tag::new("+foo-").unwrap_err(), NotelogError::TagError(TagError::InvalidDashPosition(_))));
        assert!(matches!(Tag::new("+foo_bar").unwrap_err(), NotelogError::TagError(TagError::InvalidCharacters(_))));
        assert!(matches!(Tag::new("+foo bar").unwrap_err(), NotelogError::TagError(TagError::InvalidCharacters(_))));
    }

    #[test]
    fn test_tag_display() {
        let tag = Tag::new("+foo").unwrap();
        assert_eq!(format!("{}", tag), "foo");
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
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), "foo");
        assert_eq!(tags[1].as_str(), "baz");
        assert_eq!(non_tags, vec!["bar"]);

        // Test with invalid tag
        let args = vec!["+foo".to_string(), "+foo-".to_string()];
        assert!(extract_tags_from_args(&args).is_err());
    }
}
