use crate::error::{NotelogError, Result};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
