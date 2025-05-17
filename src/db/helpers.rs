//! Helper functions for database operations

use chrono::{DateTime, Local};
use rmcp::serde_json;
use sqlx::{Pool, Sqlite, query_scalar};

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
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

    // Create a Note from the frontmatter and content
    Ok(Note::new(frontmatter, content.to_string()))
}

