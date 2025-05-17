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
use tokio::sync::{Mutex, mpsc};

use crate::db::is_valid_note_file;
use crate::error::{DatabaseError, Result};

/// File monitoring handler that sends events to a channel
struct FileMonitoringHandler {
    /// Channel sender for file events
    sender: mpsc::UnboundedSender<Event>,
}

impl FileMonitoringHandler {
    /// Create a new file monitoring handler with a channel sender
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }
}

impl EventHandler for FileMonitoringHandler {
    /// Handle file events by sending them to the channel
    fn handle_event(&mut self, result: NotifyResult<Event>) {
        match result {
            Ok(event) => {
                // Send the event to the channel
                if let Err(e) = self.sender.send(event) {
                    eprintln!("Error sending file event to channel: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error watching files: {}", e);
            }
        }
    }
}

/// Process file events from the channel
async fn process_events(
    mut receiver: mpsc::UnboundedReceiver<Event>,
    pool: Pool<Sqlite>,
    notes_dir: PathBuf,
) {
    // Create a mutex to prevent concurrent processing of the same file
    let processing = Arc::new(Mutex::new(()));

    while let Some(event) = receiver.recv().await {
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
                        let _lock = processing.lock().await;

                        // Process the note file
                        if let Err(e) = crate::db::process_note_file(&pool, &notes_dir, &path).await {
                            eprintln!("Error processing note file {}: {}", path.display(), e);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Start a file monitoring task for the notes directory
pub async fn start_file_monitoring(pool: Pool<Sqlite>, notes_dir: &Path) -> Result<()> {
    // Create a channel for sending file events
    let (sender, receiver) = mpsc::unbounded_channel();

    // Create a new file monitoring handler with the sender
    let handler = FileMonitoringHandler::new(sender);

    // Configure the watcher
    let config = Config::default()
        .with_poll_interval(Duration::from_secs(20))
        .with_compare_contents(false); // No need to compare contents, we check mtime in process_note_file

    // Create a watcher with the handler
    let mut watcher = RecommendedWatcher::new(handler, config)
        .map_err(|e| DatabaseError::Monitoring(e.to_string()))?;

    // Watch the notes directory recursively
    watcher
        .watch(notes_dir, RecursiveMode::Recursive)
        .map_err(|e| DatabaseError::Monitoring(e.to_string()))?;

    // Start a task to process events from the channel
    let notes_dir_clone = notes_dir.to_path_buf();
    tokio::spawn(async move {
        process_events(receiver, pool, notes_dir_clone).await;
    });

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
