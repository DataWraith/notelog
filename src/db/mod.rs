//! Database implementation for notelog

mod helpers;
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
// Re-export helper functions
pub use helpers::{
    add_date_conditions, check_multiple_id_matches, count_notes_with_id_prefix,
    is_valid_date_range, json_to_note, process_search_query,
};
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::{Path, PathBuf};

use crate::core::note::Note;

use crate::error::{DatabaseError, Result};

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
        let db_path = notes_dir.join(DB_FILENAME);
        let db_url = format!("sqlite:{}", db_path.display());

        // Create the database if it doesn't exist
        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url)
                .await
                .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        }

        let pool = SqlitePool::connect(&db_url)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        // Run migrations
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|e| DatabaseError::Migration(e.to_string()))?;

        Ok(Self {
            pool,
            notes_dir: notes_dir.to_path_buf(),
        })
    }

    /// Get the database connection pool
    #[cfg(test)]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Search for notes using fulltext search
    ///
    /// Returns a Vec of Notes that match the search query, ordered by relevance.
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
    ) -> Result<(Vec<Note>, usize)> {
        if query.trim().is_empty() {
            return Ok((Vec::new(), 0));
        }

        if !is_valid_date_range(before.as_ref(), after.as_ref()) {
            return Ok((Vec::new(), 0));
        }

        let base_count_query = String::from(
            r#"
            SELECT COUNT(*)
            FROM notes_fts fts
            JOIN notes n ON fts.rowid = n.id
            WHERE notes_fts MATCH ?
            "#,
        );

        let count_query =
            add_date_conditions(base_count_query, before.as_ref(), after.as_ref(), true);

        let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);

        // Process the query to handle tag prefixes (+ signs)
        // In FTS5, + is a special character, so we need to escape it or transform the query
        let processed_query = process_search_query(query)?;

        count_query_builder = count_query_builder.bind(&processed_query);

        // Bind date parameters if provided
        if let Some(before_date) = before.as_ref() {
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(before_str);
        }

        if let Some(after_date) = after.as_ref() {
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(after_str);
        }

        let total_count = count_query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        // If limit is 0, only return the count
        if let Some(limit_val) = limit {
            if limit_val == 0 {
                return Ok((Vec::new(), total_count as usize));
            }
        }

        // Build the main query
        let base_main_query = String::from(
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

        let mut main_query =
            add_date_conditions(base_main_query, before.as_ref(), after.as_ref(), true);

        // Add ORDER BY clause
        main_query.push_str(" ORDER BY rank, json_extract(n.metadata, '$.created') DESC");

        // Add LIMIT clause if provided
        if let Some(limit_val) = limit {
            main_query.push_str(&format!(" LIMIT {}", limit_val));
        }

        let mut main_query_builder = sqlx::query_as::<_, (i64, String, String, f64)>(&main_query);

        // Bind the processed search query parameter
        main_query_builder = main_query_builder.bind(&processed_query);

        // Bind date parameters if provided
        if let Some(before_date) = before.as_ref() {
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            main_query_builder = main_query_builder.bind(before_str);
        }

        if let Some(after_date) = after.as_ref() {
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            main_query_builder = main_query_builder.bind(after_str);
        }

        // Execute the query
        let notes_data = main_query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        // Convert the results to a Vec of Notes, preserving the order from the database query
        let mut notes = Vec::with_capacity(notes_data.len());
        for (_db_id, metadata_json, content, _rank) in notes_data {
            match json_to_note(&metadata_json, &content) {
                Ok(note) => notes.push(note),
                Err(e) => eprintln!("Error parsing note: {}", e),
            }
        }

        Ok((notes, total_count as usize))
    }

    /// Fetch a note by its ID prefix
    ///
    /// This function searches for notes with IDs that start with the provided prefix.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Note))` - If exactly one note is found with the given ID prefix
    /// * `Ok(None)` - If no notes are found with the given ID prefix
    /// * `Err(DatabaseError::MultipleMatches)` - If multiple notes are found with the given ID prefix
    pub async fn fetch_note_by_id(&self, id_prefix: &str) -> Result<Option<Note>> {
        // Check for multiple matches and get the count
        let count = check_multiple_id_matches(&self.pool, id_prefix).await?;

        if count > 1 {
            return Err(
                DatabaseError::MultipleMatches(id_prefix.to_string(), count as usize).into(),
            );
        }

        // If exactly one note matches, fetch it
        let note_data = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT
                metadata,
                content
            FROM notes
            WHERE json_extract(metadata, '$.id') LIKE ? || '%'
            "#,
        )
        .bind(id_prefix)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;

        if let Some((metadata_json, content)) = note_data {
            return Ok(Some(json_to_note(&metadata_json, &content)?));
        }

        // This should not happen, but we don't want to panic and kill the MCP server
        Ok(None)
    }

    /// Get the filepath of a note by its ID prefix
    ///
    /// This function searches for notes with IDs that start with the provided prefix
    /// and returns the filepath of the matching note.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - If exactly one note is found with the given ID prefix, returns its filepath
    /// * `Ok(None)` - If no notes are found with the given ID prefix
    /// * `Err(DatabaseError::MultipleMatches)` - If multiple notes are found with the given ID prefix
    pub async fn get_filepath_by_id_prefix(&self, id_prefix: &str) -> Result<Option<String>> {
        // Check for multiple matches and get the count
        let count = check_multiple_id_matches(&self.pool, id_prefix).await?;

        if count > 1 {
            return Err(
                DatabaseError::MultipleMatches(id_prefix.to_string(), count as usize).into(),
            );
        }

        // If exactly one note matches, fetch its filepath
        let filepath = sqlx::query_scalar::<_, String>(
            r#"
            SELECT filepath
            FROM notes
            WHERE json_extract(metadata, '$.id') LIKE ? || '%'
            "#,
        )
        .bind(id_prefix)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(filepath)
    }

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
        const MIN_PREFIX_LENGTH: usize = 2;

        let id_str = id.as_str();

        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM notes
            WHERE json_extract(metadata, '$.id') = ?
            "#,
        )
        .bind(id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;

        if exists == 0 {
            return Err(DatabaseError::Query(format!("ID not found: {}", id_str)).into());
        }

        // Start with the minimum prefix length and increase until we find a unique prefix
        for prefix_len in MIN_PREFIX_LENGTH..=id_str.len() {
            let prefix = &id_str[0..prefix_len];

            // Count how many notes have an ID that starts with this prefix
            let count = count_notes_with_id_prefix(&self.pool, prefix).await?;

            // If there's only one match, we've found the shortest unique prefix
            if count == 1 {
                return Ok(prefix.to_string());
            }
        }

        // If we've gone through the entire ID and still don't have a unique prefix,
        // return the full ID (this should never happen if the ID exists and is unique)
        Ok(id_str.to_string())
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
}
