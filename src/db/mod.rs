//! Database implementation for notelog

mod indexing;
#[cfg(test)]
mod tests;

// Re-export indexing functions
pub use indexing::{delete_notes_by_filepaths, get_all_note_filepaths, index_notes_with_channel};

use chrono;
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::{Path, PathBuf};

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

    /// Search for notes by tags
    ///
    /// Returns a list of notes that have all the specified tags.
    ///
    /// # Parameters
    ///
    /// * `tags` - A slice of tag strings to search for
    /// * `before` - Optional DateTime to filter notes created before this time
    /// * `after` - Optional DateTime to filter notes created after this time
    ///
    /// If both `before` and `after` are provided and `before` is less than `after`,
    /// an empty result will be returned as this represents a non-overlapping date range.
    pub async fn search_by_tags(
        &self,
        tags: &[&str],
        before: Option<chrono::DateTime<chrono::Local>>,
        after: Option<chrono::DateTime<chrono::Local>>,
    ) -> Result<Vec<String>> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        // Check for non-overlapping date range
        if let (Some(before_date), Some(after_date)) = (before.as_ref(), after.as_ref()) {
            if before_date < after_date {
                // Non-overlapping date range, return empty result
                return Ok(Vec::new());
            }
        }

        // Build the SQL query dynamically based on the number of tags
        let mut query = String::from(
            "SELECT n.filepath FROM notes n JOIN note_tags nt ON n.id = nt.note_id JOIN tags t ON nt.tag_id = t.tag_id WHERE t.tag_name IN (",
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

            if let Some(_) = before {
                query.push_str("json_extract(metadata, '$.created') <= ?");
                conditions_added = true;
            }

            if let Some(_) = after {
                if conditions_added {
                    query.push_str(" AND ");
                }
                query.push_str("json_extract(metadata, '$.created') >= ?");
            }

            query.push_str(")");
        }

        // Create a query builder
        let mut query_builder = sqlx::query_scalar::<_, String>(&query);

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
        let filepaths = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(filepaths)
    }
}
