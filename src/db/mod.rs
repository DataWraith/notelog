//! Database implementation for notelog

#[cfg(test)]
mod tests;

use chrono;
use rmcp::serde_json;
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

use crate::core::note::Note;
use crate::error::{DatabaseError, NotelogError, Result};

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
    pub async fn search_by_tags(&self, tags: &[&str]) -> Result<Vec<String>> {
        if tags.is_empty() {
            return Ok(Vec::new());
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

        // Create a query builder
        let mut query_builder = sqlx::query_scalar::<_, String>(&query);

        // Bind each tag parameter
        for tag in tags {
            query_builder = query_builder.bind(tag);
        }

        // Bind the count parameter (number of tags)
        query_builder = query_builder.bind(tags.len() as i64);

        // Execute the query
        let filepaths = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

        Ok(filepaths)
    }
}

/// Index all notes in the notes directory using channels
async fn index_notes_with_channel(pool: Pool<Sqlite>, notes_dir: &Path) -> Result<()> {
    // Create a channel for sending file paths
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PathBuf>(100);

    // Spawn a task to collect note files and send them to the channel
    let notes_dir_clone = notes_dir.to_path_buf();
    let collector_task = tokio::spawn(async move {
        if let Err(e) = collect_note_files_with_channel(&notes_dir_clone, tx).await {
            eprintln!("Error collecting note files: {}", e);
        }
    });

    // Process notes as they arrive through the channel
    let pool_clone = pool.clone();
    let notes_dir_clone = notes_dir.to_path_buf();

    // Process files as they come in
    while let Some(file_path) = rx.recv().await {
        if let Err(e) = process_note_file(&pool_clone, &notes_dir_clone, &file_path).await {
            eprintln!("Error processing note file {}: {}", file_path.display(), e);
        }
    }

    // Wait for the collector task to complete
    if let Err(e) = collector_task.await {
        eprintln!("Error in collector task: {}", e);
    }

    Ok(())
}

/// Collect note files and send them to a channel
async fn collect_note_files_with_channel(
    notes_dir: &Path,
    tx: tokio::sync::mpsc::Sender<PathBuf>,
) -> Result<()> {
    // Process the current directory
    let mut entries = fs::read_dir(notes_dir).await?;

    // First, collect all entries at this level
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = fs::metadata(&path).await?;

        if metadata.is_dir() {
            // Process subdirectories recursively using Box::pin to avoid infinite size
            Box::pin(collect_note_files_with_channel(&path, tx.clone())).await?;
            return Ok(());
        }

        if path.extension().map_or(false, |ext| ext == "md") {
            // Only include markdown files that start with a date pattern
            if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy();
                // Check if the filename starts with a date pattern (YYYY-MM-DDT)
                if filename_str.len() > 10
                    && filename_str.starts_with(|c: char| c.is_ascii_digit())
                    && filename_str[4..5] == *"-"
                    && filename_str[7..8] == *"-"
                    && filename_str[10..11] == *"T"
                {
                    // Send Markdown files that match the date pattern to the channel
                    if let Err(e) = tx.send(path).await {
                        eprintln!("Error sending file path to channel: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a single note file
async fn process_note_file(pool: &Pool<Sqlite>, notes_dir: &Path, file_path: &Path) -> Result<()> {
    // Get the file's modification time
    let metadata = fs::metadata(file_path).await?;
    let mtime = metadata.modified().unwrap_or(SystemTime::now());

    // Convert SystemTime to DateTime<Local> and format as ISO8601 with millisecond precision
    let datetime = chrono::DateTime::<chrono::Local>::from(mtime);
    let mtime_str = datetime.format("%Y-%m-%d %H:%M:%S.%3f").to_string();

    // Get the relative path from the notes directory
    let relative_path = file_path
        .strip_prefix(notes_dir)
        .map_err(|e| NotelogError::PathError(format!("Failed to create relative path: {}", e)))?
        .to_string_lossy()
        .to_string();

    // Check if the note already exists in the database with the same mtime
    let existing =
        sqlx::query_as::<_, (i64, String)>("SELECT id, mtime FROM notes WHERE filepath = ?")
            .bind(&relative_path)
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;

    // If the note exists and has the same mtime, skip processing
    if let Some((_, db_mtime)) = &existing {
        if db_mtime == &mtime_str {
            return Ok(());
        }
    }

    // Read the file content
    let content = fs::read_to_string(file_path).await?;

    // Parse the note
    let note = content.parse::<Note>()?;

    // Convert frontmatter to JSON
    let metadata_json = serde_json::to_string(note.frontmatter())
        .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

    // Insert or update the note in the database
    if let Some((id, _)) = &existing {
        sqlx::query("UPDATE notes SET mtime = ?, metadata = ?, content = ? WHERE id = ?")
            .bind(&mtime_str)
            .bind(&metadata_json)
            .bind(note.content())
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
    } else {
        sqlx::query("INSERT INTO notes (filepath, mtime, metadata, content) VALUES (?, ?, ?, ?)")
            .bind(&relative_path)
            .bind(&mtime_str)
            .bind(&metadata_json)
            .bind(note.content())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
    }

    Ok(())
}
