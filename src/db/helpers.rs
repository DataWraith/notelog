//! Helper functions for database operations

use chrono::{DateTime, Local};
use rmcp::serde_json;
use sqlx::{Pool, Sqlite, query_scalar};

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::Tag;
use crate::error::{DatabaseError, Result};

/// Add date conditions to a SQL query string
///
/// Adds WHERE clauses for before and after date conditions if they are provided.
/// The date field is assumed to be stored in the JSON metadata as '$.created'.
///
/// # Parameters
///
/// * `query` - The base SQL query string to modify
/// * `before` - Optional DateTime to filter notes created before this time
/// * `after` - Optional DateTime to filter notes created after this time
/// * `where_clause_exists` - Whether a WHERE clause already exists in the query
///
/// # Returns
///
/// The modified query string with date conditions added
pub fn add_date_conditions(
    mut query: String,
    before: Option<&DateTime<Local>>,
    after: Option<&DateTime<Local>>,
    where_clause_exists: bool,
) -> String {
    if before.is_some() {
        if where_clause_exists {
            query.push_str(" AND json_extract(n.metadata, '$.created') <= ?");
        } else {
            query.push_str(" WHERE json_extract(n.metadata, '$.created') <= ?");
        }
    }

    if after.is_some() {
        if where_clause_exists || before.is_some() {
            query.push_str(" AND json_extract(n.metadata, '$.created') >= ?");
        } else {
            query.push_str(" WHERE json_extract(n.metadata, '$.created') >= ?");
        }
    }

    query
}

/// Check if a date range is valid
///
/// A date range is valid if either:
/// - Both before and after are None
/// - Only one of before or after is Some
/// - Both before and after are Some, and before >= after
///
/// # Parameters
///
/// * `before` - Optional DateTime to filter notes created before this time
/// * `after` - Optional DateTime to filter notes created after this time
///
/// # Returns
///
/// true if the date range is valid, false otherwise
pub fn is_valid_date_range(
    before: Option<&DateTime<Local>>,
    after: Option<&DateTime<Local>>,
) -> bool {
    match (before, after) {
        (Some(before_date), Some(after_date)) => before_date >= after_date,
        _ => true,
    }
}

/// Count notes with an ID prefix
///
/// Counts how many notes have an ID that starts with the provided prefix.
///
/// # Parameters
///
/// * `pool` - The database connection pool
/// * `id_prefix` - The ID prefix to search for
///
/// # Returns
///
/// * `Ok(count)` - The number of notes with the given ID prefix
/// * `Err` - If an error occurs during the database query
pub async fn count_notes_with_id_prefix(pool: &Pool<Sqlite>, id_prefix: &str) -> Result<i64> {
    query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM notes
        WHERE json_extract(metadata, '$.id') LIKE ? || '%'
        "#,
    )
    .bind(id_prefix)
    .fetch_one(pool)
    .await
    .map_err(|e| DatabaseError::Query(e.to_string()).into())
}

/// Check for multiple matches with an ID prefix
///
/// Checks if multiple notes have an ID that starts with the provided prefix.
/// If multiple matches are found, returns an error with the count.
///
/// # Parameters
///
/// * `pool` - The database connection pool
/// * `id_prefix` - The ID prefix to search for
///
/// # Returns
///
/// * `Ok(count)` - The number of notes with the given ID prefix
/// * `Err(DatabaseError::MultipleMatches)` - If multiple notes are found with the given ID prefix
/// * `Err` - If an error occurs during the database query
pub async fn check_multiple_id_matches(pool: &Pool<Sqlite>, id_prefix: &str) -> Result<i64> {
    let count = count_notes_with_id_prefix(pool, id_prefix).await?;

    // If multiple notes match, return an error with the count
    if count > 1 {
        return Err(DatabaseError::MultipleMatches(id_prefix.to_string(), count as usize).into());
    }

    Ok(count)
}

/// Convert JSON metadata and content to a Note
///
/// Parses the frontmatter from the metadata JSON and creates a Note from the frontmatter and content.
///
/// # Parameters
///
/// * `metadata_json` - The JSON string containing the note's metadata
/// * `content` - The content of the note
///
/// # Returns
///
/// * `Ok(Note)` - The parsed Note
/// * `Err` - If an error occurs during parsing
pub fn json_to_note(metadata_json: &str, content: &str) -> Result<Note> {
    // Parse the frontmatter from the metadata JSON
    let frontmatter: Frontmatter = serde_json::from_str(metadata_json)
        .map_err(|e| DatabaseError::Serialization(e.to_string()))?;

    Ok(Note::new(frontmatter, content.to_string()))
}

/// Process a search query to handle tag prefixes (+ signs) and parentheses
///
/// In FTS5, + is a special character that means "required term", so we need to
/// handle it specially when users want to search for tags with the + prefix.
///
/// This function transforms queries with tag prefixes into a format that works with FTS5.
/// It also handles parentheses and quotes properly, ensuring they are balanced.
///
/// # Returns
///
/// * `Ok(String)` - The processed query string
/// * `Err(DatabaseError)` - If the query is invalid (e.g., unbalanced quotes or parentheses)
pub fn process_search_query(query: &str) -> Result<String> {
    // If the query is empty, return an empty string
    if query.trim().is_empty() {
        return Ok(String::new());
    }

    // Check for balanced quotes
    let quote_count = query.chars().filter(|&c| c == '"').count();
    if quote_count % 2 != 0 {
        return Err(DatabaseError::InvalidSearchQuery(
            "Unbalanced quotes in search query".to_string(),
        )
        .into());
    }

    // Check for balanced parentheses
    check_balanced_parentheses(query)?;

    // Split the query into quoted, parenthesized, and unquoted sections
    let mut result = Vec::new();
    let mut in_quotes = false;
    let mut paren_depth = 0;
    let mut section_start = 0;
    let mut escape_next = false;

    for (i, c) in query.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if c == '\\' {
            escape_next = true;
            continue;
        }

        if c == '"' && paren_depth == 0 {
            if !in_quotes {
                // Process any unquoted text before this quote
                if i > section_start {
                    let unquoted_section = &query[section_start..i];
                    process_unquoted_section(unquoted_section, &mut result)?;
                }
                // Start of quoted section
                section_start = i;
            } else {
                // End of quoted section
                let quoted_section = &query[section_start..=i];
                result.push(quoted_section.to_string());
                section_start = i + 1;
            }
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                if paren_depth == 0 {
                    // Process any unquoted text before this parenthesis
                    if i > section_start {
                        let unquoted_section = &query[section_start..i];
                        process_unquoted_section(unquoted_section, &mut result)?;
                    }
                    // Start of parenthesized section
                    section_start = i;
                }
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
                if paren_depth == 0 {
                    // End of parenthesized section
                    let paren_section = &query[section_start..=i];
                    process_parentheses_section(paren_section, &mut result)?;
                    section_start = i + 1;
                }
            }
        }
    }

    // Process any remaining unquoted text
    if section_start < query.len() {
        let unquoted_section = &query[section_start..];
        process_unquoted_section(unquoted_section, &mut result)?;
    }

    // Join the processed sections back together
    Ok(result.join(" "))
}

/// Check if parentheses in a string are balanced and properly ordered
///
/// This function checks if all opening parentheses have matching closing parentheses
/// and that they are in the correct order.
///
/// # Returns
///
/// * `Ok(())` - If parentheses are balanced and properly ordered
/// * `Err(DatabaseError)` - If parentheses are unbalanced or improperly ordered
fn check_balanced_parentheses(s: &str) -> Result<()> {
    let mut stack = Vec::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for c in s.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if c == '\\' {
            escape_next = true;
            continue;
        }

        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                stack.push(c);
            } else if c == ')' && stack.pop().is_none() {
                return Err(DatabaseError::InvalidSearchQuery(
                    "Unbalanced parentheses in search query: too many closing parentheses"
                        .to_string(),
                )
                .into());
            }
        }
    }

    if !stack.is_empty() {
        return Err(DatabaseError::InvalidSearchQuery(
            "Unbalanced parentheses in search query: missing closing parentheses".to_string(),
        )
        .into());
    }

    Ok(())
}

/// Process a parenthesized section of the search query
///
/// This function processes the content inside parentheses, preserving the parentheses
/// themselves but processing the content inside them.
///
/// # Parameters
///
/// * `section` - The parenthesized section to process, including the parentheses
/// * `result` - The vector to append the processed section to
fn process_parentheses_section(section: &str, result: &mut Vec<String>) -> Result<()> {
    // Extract the content inside the parentheses
    let content = &section[1..section.len() - 1];

    // Process the content inside the parentheses
    let processed_content = process_search_query(content)?;

    // Add the processed content back with parentheses
    result.push(format!("({})", processed_content));

    Ok(())
}

/// Process an unquoted section of the search query
///
/// This function splits the unquoted section into words and processes each word
/// according to the rules.
fn process_unquoted_section(section: &str, result: &mut Vec<String>) -> Result<()> {
    // Define boolean operators that should not be wrapped in quotes
    const BOOLEAN_OPERATORS: [&str; 3] = ["AND", "OR", "NOT"];

    // Split the section into words
    for word in section.split_whitespace() {
        if word == "+" {
            // If the word is a verbatim '+', leave it as is
            result.push(word.to_string());
        } else if word.starts_with('+') {
            // If the word is a tag (starts with a '+'), validate it and map it to 'tags:"+<word>"'
            // First, validate the tag
            match Tag::new(word) {
                Ok(_) => {
                    // Format as a column-specific search for tags
                    // Use the SQLite FTS5 column filter syntax without parentheses
                    result.push(format!("tags:\"{}\"", word));
                }
                Err(e) => {
                    return Err(DatabaseError::InvalidSearchQuery(format!(
                        "Invalid tag '{}': {}",
                        word, e
                    ))
                    .into());
                }
            }
        } else if BOOLEAN_OPERATORS.contains(&word) {
            // If the word is a boolean operator, leave it as is
            result.push(word.to_string());
        } else {
            // Otherwise, wrap the word in quotes
            result.push(format!("\"{}\"", word));
        }
    }
    Ok(())
}

#[cfg(test)]
mod query_tests {
    use super::process_search_query;
    use crate::error::DatabaseError;

    #[test]
    fn test_process_search_query_basic() {
        // Test basic query with no special characters
        assert_eq!(
            process_search_query("hello world").unwrap(),
            r#""hello" "world""#
        );
    }

    #[test]
    fn test_process_search_query_with_tags() {
        // Test query with tag prefixes
        assert_eq!(
            process_search_query("+tag1 +tag2").unwrap(),
            r#"tags:"+tag1" tags:"+tag2""#
        );
    }

    #[test]
    fn test_process_search_query_with_quotes() {
        // Test query with quotes
        assert_eq!(
            process_search_query(r#"hello "world""#).unwrap(),
            r#""hello" "world""#
        );
    }

    #[test]
    fn test_process_search_query_with_tag_and_quotes() {
        // Test query with tag prefix and quotes
        assert_eq!(
            process_search_query(r#"+tag "hello""#).unwrap(),
            r#"tags:"+tag" "hello""#
        );
    }

    #[test]
    fn test_process_search_query_with_unbalanced_quotes() {
        // Test query with unbalanced quotes
        let result = process_search_query(r#"hello "world"#);
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                crate::error::NotelogError::DatabaseError(DatabaseError::InvalidSearchQuery(
                    msg,
                )) => {
                    assert!(msg.contains("Unbalanced quotes"));
                }
                _ => panic!("Expected InvalidSearchQuery error"),
            },
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_process_search_query_with_invalid_tag() {
        // Test query with invalid tag
        let result = process_search_query("+tag_invalid");
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                crate::error::NotelogError::DatabaseError(DatabaseError::InvalidSearchQuery(
                    msg,
                )) => {
                    assert!(msg.contains("Invalid tag"));
                }
                _ => panic!("Expected InvalidSearchQuery error"),
            },
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_process_search_query_with_empty_query() {
        // Test empty query
        assert_eq!(process_search_query("").unwrap(), "");
        assert_eq!(process_search_query("   ").unwrap(), "");
    }

    #[test]
    fn test_process_search_query_with_verbatim_plus() {
        // Test query with a verbatim '+'
        assert_eq!(
            process_search_query("foo + bar").unwrap(),
            r#""foo" + "bar""#
        );
    }

    #[test]
    fn test_process_search_query_with_mixed_content() {
        // Test query with mixed content
        assert_eq!(
            process_search_query(r#"foo +bar "quoted text" baz"#).unwrap(),
            r#""foo" tags:"+bar" "quoted text" "baz""#
        );
    }

    #[test]
    fn test_process_search_query_with_quoted_tags() {
        // Test query with tags in quotes
        assert_eq!(
            process_search_query(r#""text with +tag inside""#).unwrap(),
            r#""text with +tag inside""#
        );
    }

    #[test]
    fn test_process_search_query_with_backslash_escape() {
        // Test query with backslash escaping a quote
        assert_eq!(
            process_search_query(r#"text with \"escaped quotes\""#).unwrap(),
            r#""text" "with" "\"escaped" "quotes\"""#
        );
    }

    #[test]
    fn test_process_search_query_with_and_operator() {
        // Test query with AND operator
        assert_eq!(
            process_search_query("foo AND bar").unwrap(),
            r#""foo" AND "bar""#
        );
    }

    #[test]
    fn test_process_search_query_with_or_operator() {
        // Test query with OR operator
        assert_eq!(
            process_search_query("foo OR bar").unwrap(),
            r#""foo" OR "bar""#
        );
    }

    #[test]
    fn test_process_search_query_with_not_operator() {
        // Test query with NOT operator
        assert_eq!(
            process_search_query("foo NOT bar").unwrap(),
            r#""foo" NOT "bar""#
        );
    }

    #[test]
    fn test_process_search_query_with_parentheses() {
        // Test query with parentheses
        assert_eq!(
            process_search_query("(foo bar)").unwrap(),
            r#"("foo" "bar")"#
        );
    }

    #[test]
    fn test_process_search_query_with_complex_operators() {
        // Test query with complex operators
        assert_eq!(
            process_search_query("(foo AND bar) OR (baz NOT qux)").unwrap(),
            r#"("foo" AND "bar") OR ("baz" NOT "qux")"#
        );
    }

    #[test]
    fn test_process_search_query_with_tags_and_operators() {
        // Test query with tags and operators
        assert_eq!(
            process_search_query("+project AND (meeting OR call) NOT +cancelled").unwrap(),
            r#"tags:"+project" AND ("meeting" OR "call") NOT tags:"+cancelled""#
        );
    }

    #[test]
    fn test_process_search_query_with_nested_parentheses() {
        // Test query with nested parentheses
        assert_eq!(
            process_search_query("(foo AND (bar OR baz))").unwrap(),
            r#"("foo" AND ("bar" OR "baz"))"#
        );
    }

    #[test]
    fn test_process_search_query_with_unbalanced_parentheses() {
        // Test query with unbalanced parentheses (missing closing parenthesis)
        let result = process_search_query("(foo bar");
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                crate::error::NotelogError::DatabaseError(DatabaseError::InvalidSearchQuery(
                    msg,
                )) => {
                    assert!(msg.contains("Unbalanced parentheses"));
                }
                _ => panic!("Expected InvalidSearchQuery error"),
            },
            _ => panic!("Expected error"),
        }

        // Test query with unbalanced parentheses (missing opening parenthesis)
        let result = process_search_query("foo bar)");
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                crate::error::NotelogError::DatabaseError(DatabaseError::InvalidSearchQuery(
                    msg,
                )) => {
                    assert!(msg.contains("Unbalanced parentheses"));
                }
                _ => panic!("Expected InvalidSearchQuery error"),
            },
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_process_search_query_with_parentheses_in_quotes() {
        // Test query with parentheses inside quotes
        assert_eq!(
            process_search_query(r#""(foo bar)""#).unwrap(),
            r#""(foo bar)""#
        );
    }

    #[test]
    fn test_process_search_query_with_quotes_in_parentheses() {
        // Test query with quotes inside parentheses
        assert_eq!(
            process_search_query(r#"(foo "bar baz")"#).unwrap(),
            r#"("foo" "bar baz")"#
        );
    }

    #[test]
    fn test_process_search_query_with_quoted_operators() {
        // Test query with quoted operators
        assert_eq!(
            process_search_query(r#""AND OR NOT" +tag"#).unwrap(),
            r#""AND OR NOT" tags:"+tag""#
        );
    }
}
