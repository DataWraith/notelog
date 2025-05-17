//! Indexing functionality for the database

use rmcp::serde_json;
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

use crate::constants::MAX_FILE_SIZE_BYTES;

use crate::core::note::Note;
use crate::error::{DatabaseError, NotelogError, Result};

/// Check if a file path is a valid note file
///
/// A valid note file must:
/// - Have a .md extension
/// - Have a filename that starts with '1' or '2' (for year 1xxx or 2xxx)
///   to filter out non-note files like README.md or monthly rollups
/// - Be less than 50 KiB in size
pub async fn is_valid_note_file(path: &Path) -> bool {
    // Check if it's a markdown file
    if path.extension().is_none_or(|ext| ext != "md") {
        return false;
    }

    // Check if the filename starts with a date pattern
    if let Some(filename) = path.file_name() {
        let filename_str = filename.to_string_lossy();
        // Only include files that start with '1' or '2' (for year 1xxx or 2xxx)
        // This assumes the program won't be used for notes in the year 3000
        if !filename_str.starts_with('1') && !filename_str.starts_with('2') {
            return false;
        }
    } else {
        return false;
    }

    // Check file size (must be less than MAX_FILE_SIZE_BYTES)
    if let Ok(metadata) = fs::metadata(path).await {
        let file_size = metadata.len();
        if file_size > MAX_FILE_SIZE_BYTES as u64 {
            return false;
        }
    } else {
        // If we can't get the metadata, consider it invalid
        return false;
    }

    true
}

/// Get all note filepaths from the database
pub async fn get_all_note_filepaths(pool: &Pool<Sqlite>) -> Result<Vec<String>> {
    let filepaths = sqlx::query_scalar::<_, String>(
        r#"
        SELECT filepath FROM notes
    "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| DatabaseError::Query(e.to_string()))?;

    Ok(filepaths)
}

/// Index all notes in the notes directory using channels
pub async fn index_notes_with_channel(pool: Pool<Sqlite>, notes_dir: &Path) -> Result<()> {
    // First, get all existing note filepaths from the database
    let existing_filepaths = get_all_note_filepaths(&pool).await?;

    // Create a HashSet to track which notes still exist on disk
    let mut filepaths_to_delete = existing_filepaths
        .into_iter()
        .collect::<std::collections::HashSet<String>>();

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
        // Get the relative path from the notes directory
        if let Ok(relative_path) = file_path
            .strip_prefix(&notes_dir_clone)
            .map(|p| p.to_string_lossy().to_string())
        {
            // Remove this filepath from the set of files to delete
            filepaths_to_delete.remove(&relative_path);

            // Process the note file
            if let Err(e) = process_note_file(&pool_clone, &notes_dir_clone, &file_path).await {
                eprintln!("Error processing note file {}: {}", file_path.display(), e);
            }
        }
    }

    // Wait for the collector task to complete
    if let Err(e) = collector_task.await {
        eprintln!("Error in collector task: {}", e);
    }

    // Delete notes that no longer exist on disk
    if !filepaths_to_delete.is_empty() {
        let filepaths_vec: Vec<String> = filepaths_to_delete.into_iter().collect();
        if let Err(e) = delete_notes_by_filepaths(&pool, &filepaths_vec).await {
            eprintln!("Error deleting notes from database: {}", e);
        }
    }

    Ok(())
}

/// Process a single note file
pub async fn process_note_file(
    pool: &Pool<Sqlite>,
    notes_dir: &Path,
    file_path: &Path,
) -> Result<()> {
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
    let existing = sqlx::query_as::<_, (i64, String)>(
        r#"
            SELECT
                id,
                mtime
            FROM notes
            WHERE filepath = ?
        "#,
    )
    .bind(&relative_path)
    .fetch_optional(pool)
    .await
    .map_err(|e| DatabaseError::Query(e.to_string()))?;

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
        .map_err(|e| DatabaseError::Serialization(e.to_string()))?;

    // Insert or update the note in the database
    if let Some((id, _)) = &existing {
        sqlx::query(
            r#"
            UPDATE notes
            SET
                mtime = ?,
                metadata = ?,
                content = ?
            WHERE id = ?
        "#,
        )
        .bind(&mtime_str)
        .bind(&metadata_json)
        .bind(note.content())
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO notes (
                filepath,
                mtime,
                metadata,
                content
            ) VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(&relative_path)
        .bind(&mtime_str)
        .bind(&metadata_json)
        .bind(note.content())
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;
    }

    Ok(())
}

/// Delete notes from the database by their filepaths
pub async fn delete_notes_by_filepaths(pool: &Pool<Sqlite>, filepaths: &[String]) -> Result<()> {
    // Use a transaction to ensure all deletions are atomic
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;

    for filepath in filepaths {
        // The after_note_delete trigger will handle removing tag relationships
        // and updating tag usage counts
        sqlx::query(
            r#"
            DELETE FROM notes
            WHERE filepath = ?
        "#,
        )
        .bind(filepath)
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;
    }

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?;

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
            continue;
        }

        if is_valid_note_file(&path).await {
            // Send valid note files to the channel
            if let Err(e) = tx.send(path).await {
                eprintln!("Error sending file path to channel: {}", e);
            }
        }
    }

    Ok(())
}
