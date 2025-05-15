//! Database implementation for notelog

mod indexing;
mod monitoring;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use indexing::{delete_notes_by_filepaths, get_all_note_filepaths};

// Re-export indexing functions
pub use indexing::{index_notes_with_channel, is_valid_note_file, process_note_file};
// Re-export monitoring functions
pub use monitoring::start_file_monitoring;
use rmcp::serde_json;
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::{Path, PathBuf};

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;

use crate::error::{DatabaseError, Result};

/// The name of the SQLite database file
const DB_FILENAME: &str = ".notes.db";

/// Database connection pool
#[derive(Debug)]
pub struct Database {
    /// The SQLite connection pool
    pool: Pool<Sqlite>,
    /// The path to the notes directory
    notes_dir: PathBuf,
}

impl Database {
    /// Find the shortest unique prefix of a given ID
    ///
    /// This function uses the note_id_idx index to find the shortest prefix of the given ID
    /// that uniquely identifies a note in the database. It will always return at least
    /// 2 characters, even if a shorter prefix would be unique.
    ///
    /// # Parameters
    ///
    /// * `id` - The Id struct to find a unique prefix for
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The shortest unique prefix of the ID (minimum 2 characters)
    /// * `Err` - If an error occurs or if the ID doesn't exist in the database
    pub async fn find_shortest_unique_id_prefix(&self, id: &crate::core::id::Id) -> Result<String> {
        // Minimum prefix length is 2 characters
        const MIN_PREFIX_LENGTH: usize = 2;

        // Get the string representation of the ID
        let id_str = id.as_str();

        // Ensure the ID is at least the minimum length
        if id_str.len() < MIN_PREFIX_LENGTH {
            return Err(DatabaseError::QueryError(format!("ID is too short: {}", id_str)).into());
        }

        // Check if the full ID exists in the database
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM notes
            WHERE json_extract(metadata, '$.id') = ?
            "#
        )
        .bind(id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        if exists == 0 {
            return Err(DatabaseError::QueryError(format!("ID not found: {}", id_str)).into());
        }

        // Start with the minimum prefix length and increase until we find a unique prefix
        for prefix_len in MIN_PREFIX_LENGTH..=id_str.len() {
            let prefix = &id_str[0..prefix_len];

            // Count how many notes have an ID that starts with this prefix
            let count = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM notes
                WHERE json_extract(metadata, '$.id') LIKE ? || '%'
                "#
            )
            .bind(prefix)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

            // If there's only one match, we've found the shortest unique prefix
            if count == 1 {
                return Ok(prefix.to_string());
            }
        }

        // If we've gone through the entire ID and still don't have a unique prefix,
        // return the full ID (this should never happen if the ID exists and is unique)
        Ok(id_str.to_string())
    }

    /// Initialize the database
    ///
    /// This will create the database file if it doesn't exist and run migrations.
    pub async fn initialize(notes_dir: &Path) -> Result<Self> {
        // Create the database path
        let db_path = notes_dir.join(DB_FILENAME);
        let db_url = format!("sqlite:{}", db_path.display());

        // Create the database if it doesn't exist
        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url)
                .await
                .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        }

        // Connect to the database
        let pool = SqlitePool::connect(&db_url)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Run migrations
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;

        Ok(Self {
            pool,
            notes_dir: notes_dir.to_path_buf(),
        })
    }

    /// Start a background task to index all notes in the notes directory
    pub async fn start_indexing_task(&self) -> Result<()> {
        // Clone the pool and notes_dir for the background task
        let pool = self.pool.clone();
        let notes_dir = self.notes_dir.clone();

        // Spawn a background task to index notes using channels
        tokio::spawn(async move {
            if let Err(e) = index_notes_with_channel(pool, &notes_dir).await {
                eprintln!("Error indexing notes: {}", e);
            }
        });

        Ok(())
    }

    /// Start a background task to monitor the notes directory for changes
    pub async fn start_monitoring_task(&self) -> Result<()> {
        // Clone the pool and notes_dir for the background task
        let pool = self.pool.clone();
        let notes_dir = self.notes_dir.clone();

        // Start the file monitoring task
        start_file_monitoring(pool, &notes_dir).await
    }

    /// Get the database connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Fetch a note by its ID
    ///
    /// Returns the note if found, or None if no note with the given ID exists.
    pub async fn fetch_note_by_id(&self, id: i64) -> Result<Option<Note>> {
        // Query the database for the note with the given ID
        let note_data = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT
                metadata,
                content
            FROM notes
            WHERE id = ?
        "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        // If no note was found, return None
        if let Some((metadata_json, content)) = note_data {
            // Parse the frontmatter from the metadata JSON
            let frontmatter: Frontmatter = serde_json::from_str(&metadata_json)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

            // Create a Note from the frontmatter and content
            let note = Note::new(frontmatter, content);

            Ok(Some(note))
        } else {
            Ok(None)
        }
    }

    /// Search for notes using fulltext search
    ///
    /// Returns a BTreeMap of note IDs to Notes that match the search query.
    ///
    /// # Parameters
    ///
    /// * `query` - The search query string
    /// * `before` - Optional DateTime to filter notes created before this time
    /// * `after` - Optional DateTime to filter notes created after this time
    /// * `limit` - Optional limit on the number of results to return
    ///
    /// The query can include tag prefixes (e.g., "+project") to search for specific tags.
    /// If both `before` and `after` are provided and `before` is less than `after`,
    /// an empty result will be returned as this represents a non-overlapping date range.
    pub async fn search_notes(
        &self,
        query: &str,
        before: Option<chrono::DateTime<chrono::Local>>,
        after: Option<chrono::DateTime<chrono::Local>>,
        limit: Option<usize>,
    ) -> Result<(std::collections::BTreeMap<i64, Note>, usize)> {
        if query.trim().is_empty() {
            return Ok((std::collections::BTreeMap::new(), 0));
        }

        // Check for non-overlapping date range
        if let (Some(before_date), Some(after_date)) = (before.as_ref(), after.as_ref()) {
            if before_date < after_date {
                // Non-overlapping date range, return empty result
                return Ok((std::collections::BTreeMap::new(), 0));
            }
        }

        // Build the count query with parameter placeholders
        let mut count_query = String::from(
            r#"
            SELECT COUNT(*)
            FROM notes_fts fts
            JOIN notes n ON fts.rowid = n.id
            WHERE notes_fts MATCH ?
            "#,
        );

        // Add date conditions to the count query if needed
        if before.is_some() {
            count_query.push_str(" AND json_extract(n.metadata, '$.created') <= ?");
        }

        if after.is_some() {
            count_query.push_str(" AND json_extract(n.metadata, '$.created') >= ?");
        }

        // Create a query builder for the count query
        let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);

        // Process the query to handle tag prefixes (+ signs)
        // In FTS5, + is a special character, so we need to escape it or transform the query
        let processed_query = process_search_query(query);

        // Bind the processed search query parameter
        count_query_builder = count_query_builder.bind(&processed_query);

        // Bind date parameters if provided
        if let Some(before_date) = &before {
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(before_str);
        }

        if let Some(after_date) = &after {
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(after_str);
        }

        // Execute the count query
        let total_count = count_query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        // If limit is 0, only return the count
        if let Some(limit_val) = limit {
            if limit_val == 0 {
                return Ok((std::collections::BTreeMap::new(), total_count as usize));
            }
        }

        // Build the main query with parameter placeholders
        let mut main_query = String::from(
            r#"
            SELECT
                n.id,
                n.metadata,
                n.content,
                rank
            FROM notes_fts fts
            JOIN notes n ON fts.rowid = n.id
            WHERE notes_fts MATCH ?
            "#,
        );

        // Add date conditions to the main query if needed
        if before.is_some() {
            main_query.push_str(" AND json_extract(n.metadata, '$.created') <= ?");
        }

        if after.is_some() {
            main_query.push_str(" AND json_extract(n.metadata, '$.created') >= ?");
        }

        // Add ORDER BY clause
        main_query.push_str(" ORDER BY rank, json_extract(n.metadata, '$.created') DESC");

        // Add LIMIT clause if provided
        if let Some(limit_val) = limit {
            main_query.push_str(&format!(" LIMIT {}", limit_val));
        }

        // Create a query builder for the main query
        let mut main_query_builder = sqlx::query_as::<_, (i64, String, String, f64)>(&main_query);

        // Bind the processed search query parameter (reuse the one we created earlier)
        main_query_builder = main_query_builder.bind(&processed_query);

        // Bind date parameters if provided
        if let Some(before_date) = &before {
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            main_query_builder = main_query_builder.bind(before_str);
        }

        if let Some(after_date) = &after {
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            main_query_builder = main_query_builder.bind(after_str);
        }

        // Execute the query
        let notes_data = main_query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        // Convert the results to a BTreeMap of id => Note
        let mut notes = std::collections::BTreeMap::new();
        for (id, metadata_json, content, _rank) in notes_data {
            // Parse the frontmatter from the metadata JSON
            let frontmatter: Frontmatter = serde_json::from_str(&metadata_json)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

            // Create a Note from the frontmatter and content
            let note = Note::new(frontmatter, content);

            // Add the note to the map
            notes.insert(id, note);
        }

        Ok((notes, total_count as usize))
    }
}

/// Process a search query to handle tag prefixes (+ signs)
///
/// In FTS5, + is a special character that means "required term", so we need to
/// handle it specially when users want to search for tags with the + prefix.
///
/// This function transforms queries with tag prefixes into a format that works with FTS5.
fn process_search_query(query: &str) -> String {
    // If the query is empty, return an empty string
    if query.trim().is_empty() {
        return String::new();
    }

    // Split the query into words, preserving quoted phrases
    let mut processed_words = Vec::new();
    let mut current_word = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for c in query.chars() {
        match c {
            // Handle escape character
            '\\' if !escape_next => {
                escape_next = true;
                current_word.push(c);
            }
            // Handle quotes
            '"' if !escape_next => {
                in_quotes = !in_quotes;
                // Don't include the quote in the processed word
                // We'll add our own quotes later if needed
                current_word.push(c);
            }
            // Handle spaces
            ' ' if !in_quotes && !escape_next => {
                if !current_word.is_empty() {
                    processed_words.push(current_word);
                    current_word = String::new();
                }
            }
            // Handle all other characters
            _ => {
                escape_next = false;
                current_word.push(c);
            }
        }
    }

    // Add the last word if there is one
    if !current_word.is_empty() {
        processed_words.push(current_word);
    }

    // Process each word
    let processed_words: Vec<String> = processed_words
        .into_iter()
        .map(|word| {
            if word.starts_with('+') {
                // For tag searches (words starting with +), we need to escape the +
                // We'll wrap it in quotes, but first escape any existing quotes
                let escaped_word = word.replace('"', "\"\"");
                format!("\"{}\"", escaped_word)
            } else if word.contains('"') {
                // If the word contains quotes, escape them for SQLite FTS5
                word.replace('"', "\"\"")
            } else {
                // For regular words, keep them as is
                word
            }
        })
        .collect();

    // Join the processed words back into a query string
    processed_words.join(" ")
}

#[cfg(test)]
mod query_tests {
    use super::process_search_query;

    #[test]
    fn test_process_search_query_basic() {
        // Test basic query with no special characters
        assert_eq!(process_search_query("hello world"), "hello world");
    }

    #[test]
    fn test_process_search_query_with_tags() {
        // Test query with tag prefixes
        assert_eq!(process_search_query("+tag1 +tag2"), "\"+tag1\" \"+tag2\"");
    }

    #[test]
    fn test_process_search_query_with_quotes() {
        // Test query with quotes
        assert_eq!(
            process_search_query("hello \"world\""),
            "hello \"\"world\"\""
        );
    }

    #[test]
    fn test_process_search_query_with_tag_and_quotes() {
        // Test query with tag prefix and quotes
        assert_eq!(
            process_search_query("+tag \"hello\""),
            "\"+tag\" \"\"hello\"\""
        );
    }

    #[test]
    fn test_process_search_query_with_malformed_quotes() {
        // Test query with malformed quotes
        assert_eq!(process_search_query("hello \"world"), "hello \"\"world");
    }

    #[test]
    fn test_process_search_query_with_malformed_tag_and_quotes() {
        // Test query with malformed tag prefix and quotes
        assert_eq!(process_search_query("+tag \"hello"), "\"+tag\" \"\"hello");
    }

    #[test]
    fn test_process_search_query_with_empty_query() {
        // Test empty query
        assert_eq!(process_search_query(""), "");
        assert_eq!(process_search_query("   "), "");
    }
}
