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

        // Bind the search query parameter
        count_query_builder = count_query_builder.bind(query);

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

        // Bind the search query parameter
        main_query_builder = main_query_builder.bind(query);

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
