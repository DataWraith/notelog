#[cfg(test)]
mod tests {
    use crate::core::frontmatter::Frontmatter;
    use crate::core::note::Note;
    use crate::core::tags::Tag;
    use crate::db::{DB_FILENAME, Database, index_notes_with_channel};
    use std::fs;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;

    #[test]
    fn test_database_initialization() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a tokio runtime for testing
        let rt = Runtime::new().unwrap();

        // Initialize the database
        let _db = rt.block_on(async { Database::initialize(notes_dir).await.unwrap() });

        // Verify the database file was created
        let db_path = notes_dir.join(DB_FILENAME);
        assert!(db_path.exists());
    }

    #[test]
    fn test_note_indexing() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a tokio runtime for testing
        let rt = Runtime::new().unwrap();

        // Create a test note
        rt.block_on(async {
            // Create year/month directories
            let year_dir = notes_dir.join("2025");
            let month_dir = year_dir.join("05");
            fs::create_dir_all(&month_dir).unwrap();

            // Create a test note with tags
            let mut frontmatter = Frontmatter::default();
            let tag1 = Tag::new("test").unwrap();
            let tag2 = Tag::new("example").unwrap();
            frontmatter.add_tag(tag1);
            frontmatter.add_tag(tag2);

            let content = "# Test Note\nThis is a test note for database indexing.";
            let note = Note::new(frontmatter, content.to_string());

            // Save the note to disk
            let note_path = note.save(notes_dir, Some("Test Note")).unwrap();
            assert!(notes_dir.join(&note_path).exists());

            // Initialize the database
            let db = Database::initialize(notes_dir).await.unwrap();

            // Run the indexing task
            index_notes_with_channel(db.pool().clone(), notes_dir)
                .await
                .unwrap();

            // Search for notes by tags
            let results = db.search_by_tags(&["test"]).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], note_path.to_string_lossy());

            // Search for notes by multiple tags
            let results = db.search_by_tags(&["test", "example"]).await.unwrap();
            assert_eq!(results.len(), 1);

            // Search for non-existent tag
            let results = db.search_by_tags(&["nonexistent"]).await.unwrap();
            assert_eq!(results.len(), 0);
        });
    }
}
