//! File monitoring functionality for the database

use notify::{
    Config, Event, EventHandler, EventKind, RecommendedWatcher, RecursiveMode,
    Result as NotifyResult, Watcher,
};
use sqlx::Pool;
use sqlx::Sqlite;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::db::{is_valid_note_file, process_note_file};
use crate::error::{DatabaseError, Result};

/// File monitoring handler that processes file events
struct FileMonitoringHandler {
    /// SQLite connection pool
    pool: Pool<Sqlite>,
    /// Path to the notes directory
    notes_dir: PathBuf,
    /// Mutex to prevent concurrent processing of the same file
    processing: Arc<Mutex<()>>,
}

impl FileMonitoringHandler {
    /// Create a new file monitoring handler
    fn new(pool: Pool<Sqlite>, notes_dir: PathBuf) -> Self {
        Self {
            pool,
            notes_dir,
            processing: Arc::new(Mutex::new(())),
        }
    }

    /// Process a file event
    async fn process_event(&self, event: Event) -> Result<()> {
        // Only process events that are related to file modifications
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Process each path in the event
                for path in event.paths {
                    // Skip directories
                    if path.is_dir() {
                        continue;
                    }

                    // Check if the file is a valid note file
                    if is_valid_note_file(&path).await {
                        // Acquire the lock to prevent concurrent processing
                        let _lock = self.processing.lock().await;

                        // Process the note file
                        if let Err(e) = process_note_file(&self.pool, &self.notes_dir, &path).await
                        {
                            eprintln!("Error processing note file {}: {}", path.display(), e);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl EventHandler for FileMonitoringHandler {
    /// Handle file events
    fn handle_event(&mut self, result: NotifyResult<Event>) {
        match result {
            Ok(event) => {
                // Process the event in a tokio task
                let handler = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = handler.process_event(event).await {
                        eprintln!("Error processing event: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error watching files: {}", e);
            }
        }
    }
}

// Implement Clone for FileMonitoringHandler
impl Clone for FileMonitoringHandler {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            notes_dir: self.notes_dir.clone(),
            processing: self.processing.clone(),
        }
    }
}

/// Start a file monitoring task for the notes directory
pub async fn start_file_monitoring(pool: Pool<Sqlite>, notes_dir: &Path) -> Result<()> {
    // Create a new file monitoring handler
    let handler = FileMonitoringHandler::new(pool, notes_dir.to_path_buf());

    // Configure the watcher
    let config = Config::default()
        .with_poll_interval(Duration::from_secs(2))
        .with_compare_contents(false); // No need to compare contents, we check mtime in process_note_file

    // Create a watcher with the handler
    let mut watcher = RecommendedWatcher::new(handler, config)
        .map_err(|e| DatabaseError::MonitoringError(e.to_string()))?;

    // Watch the notes directory recursively
    watcher
        .watch(notes_dir, RecursiveMode::Recursive)
        .map_err(|e| DatabaseError::MonitoringError(e.to_string()))?;

    // Keep the watcher alive by moving it into a tokio task
    tokio::spawn(async move {
        // This task will keep running as long as the watcher is alive
        // The watcher will be dropped when the task is dropped
        let _watcher = watcher;
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });

    Ok(())
}
