//! Database implementation for notelog

mod indexing;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use indexing::{delete_notes_by_filepaths, get_all_note_filepaths};

// Re-export indexing functions
pub use indexing::{index_notes_with_channel, process_note_file};
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
            "SELECT metadata, content FROM notes WHERE id = ?",
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

    /// Search for notes by tags
    ///
    /// Returns a BTreeMap of note IDs to Notes that have all the specified tags.
    ///
    /// # Parameters
    ///
    /// * `tags` - A slice of tag strings to search for
    /// * `before` - Optional DateTime to filter notes created before this time
    /// * `after` - Optional DateTime to filter notes created after this time
    /// * `limit` - Optional limit on the number of results to return
    ///
    /// If both `before` and `after` are provided and `before` is less than `after`,
    /// an empty result will be returned as this represents a non-overlapping date range.
    pub async fn search_by_tags(
        &self,
        tags: &[&str],
        before: Option<chrono::DateTime<chrono::Local>>,
        after: Option<chrono::DateTime<chrono::Local>>,
        limit: Option<usize>,
    ) -> Result<(std::collections::BTreeMap<i64, Note>, usize)> {
        if tags.is_empty() {
            return Ok((std::collections::BTreeMap::new(), 0));
        }

        // Check for non-overlapping date range
        if let (Some(before_date), Some(after_date)) = (before.as_ref(), after.as_ref()) {
            if before_date < after_date {
                // Non-overlapping date range, return empty result
                return Ok((std::collections::BTreeMap::new(), 0));
            }
        }

        // First, get the total count of matching notes
        let mut count_query = String::from(
            "SELECT COUNT(DISTINCT n.id) FROM notes n JOIN note_tags nt ON n.id = nt.note_id JOIN tags t ON nt.tag_id = t.tag_id WHERE t.tag_name IN (",
        );

        // Add placeholders for each tag
        for (i, _) in tags.iter().enumerate() {
            if i > 0 {
                count_query.push_str(", ");
            }
            count_query.push('?');
        }

        // Complete the query to group by note_id and ensure all tags are present
        count_query.push_str(") GROUP BY n.id HAVING COUNT(DISTINCT t.tag_name) = ?");

        // Add date range conditions if provided
        if before.is_some() || after.is_some() {
            count_query.push_str(" AND n.id IN (SELECT id FROM notes WHERE ");

            let mut conditions_added = false;

            if before.is_some() {
                count_query.push_str("json_extract(metadata, '$.created') <= ?");
                conditions_added = true;
            }

            if after.is_some() {
                if conditions_added {
                    count_query.push_str(" AND ");
                }
                count_query.push_str("json_extract(metadata, '$.created') >= ?");
            }

            count_query.push(')');
        }

        // Create the count query string
        let count_query_str = format!("SELECT COUNT(*) FROM ({}) as count_query", count_query);

        // Create a query builder for the count
        let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query_str);

        // Bind each tag parameter for the count query
        for tag in tags {
            count_query_builder = count_query_builder.bind(tag);
        }

        // Bind the count parameter (number of tags) for the count query
        count_query_builder = count_query_builder.bind(tags.len() as i64);

        // Bind date parameters if provided for the count query
        if let Some(before_date) = before {
            // Format the date as ISO8601 string with the same format used in frontmatter
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(before_str);
        }

        if let Some(after_date) = after {
            // Format the date as ISO8601 string with the same format used in frontmatter
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            count_query_builder = count_query_builder.bind(after_str);
        }

        // Execute the count query
        let total_count = count_query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        if let Some(limit) = limit {
            if limit == 0 {
                return Ok((std::collections::BTreeMap::new(), total_count as usize));
            }
        }

        // Build the SQL query dynamically based on the number of tags
        let mut query = String::from(
            "SELECT n.id, n.metadata, n.content FROM notes n JOIN note_tags nt ON n.id = nt.note_id JOIN tags t ON nt.tag_id = t.tag_id WHERE t.tag_name IN (",
        );

        // Add placeholders for each tag
        for (i, _) in tags.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push('?');
        }

        // Complete the query to group by note_id and ensure all tags are present
        query.push_str(") GROUP BY n.id HAVING COUNT(DISTINCT t.tag_name) = ?");

        // Add date range conditions if provided
        if before.is_some() || after.is_some() {
            query.push_str(" AND n.id IN (SELECT id FROM notes WHERE ");

            let mut conditions_added = false;

            if before.is_some() {
                query.push_str("json_extract(metadata, '$.created') <= ?");
                conditions_added = true;
            }

            if after.is_some() {
                if conditions_added {
                    query.push_str(" AND ");
                }
                query.push_str("json_extract(metadata, '$.created') >= ?");
            }

            query.push(')');
        }

        // Add ORDER BY and LIMIT clauses
        query.push_str(" ORDER BY json_extract(n.metadata, '$.created') DESC");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        // Create a query builder
        let mut query_builder = sqlx::query_as::<_, (i64, String, String)>(&query);

        // Bind each tag parameter
        for tag in tags {
            query_builder = query_builder.bind(tag);
        }

        // Bind the count parameter (number of tags)
        query_builder = query_builder.bind(tags.len() as i64);

        // Bind date parameters if provided
        if let Some(before_date) = before {
            // Format the date as ISO8601 string with the same format used in frontmatter
            let before_str = before_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            query_builder = query_builder.bind(before_str);
        }

        if let Some(after_date) = after {
            // Format the date as ISO8601 string with the same format used in frontmatter
            let after_str = after_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
            query_builder = query_builder.bind(after_str);
        }

        // Execute the query
        let notes_data = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        // Convert the results to a BTreeMap of id => Note
        let mut notes = std::collections::BTreeMap::new();
        for (id, metadata_json, content) in notes_data {
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
