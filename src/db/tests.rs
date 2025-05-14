#[cfg(test)]
mod tests {
    use crate::core::frontmatter::Frontmatter;
    use crate::core::note::Note;
    use crate::core::tags::Tag;
    use crate::db::{
        DB_FILENAME, Database, delete_notes_by_filepaths, get_all_note_filepaths,
        index_notes_with_channel,
    };
    use chrono::{Local, TimeZone};
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

            // Search for notes by tag using fulltext search
            let (notes, total_count) = db.search_notes("+test", None, None, None).await.unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(total_count, 1);

            // Search for notes by multiple tags using fulltext search
            let (notes, total_count) = db
                .search_notes("+test +example", None, None, None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(total_count, 1);

            // Search for non-existent tag using fulltext search
            let (notes, total_count) = db
                .search_notes("+nonexistent", None, None, None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 0);
            assert_eq!(total_count, 0);
        });
    }

    #[test]
    fn test_note_deletion() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a tokio runtime for testing
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create year/month directories
            let year_dir = notes_dir.join("2025");
            let month_dir = year_dir.join("05");
            fs::create_dir_all(&month_dir).unwrap();

            // Create two test notes with tags
            let mut frontmatter1 = Frontmatter::default();
            let tag1 = Tag::new("test").unwrap();
            let tag2 = Tag::new("example").unwrap();
            frontmatter1.add_tag(tag1.clone());
            frontmatter1.add_tag(tag2.clone());

            let content1 = "# Test Note 1\nThis is the first test note.";
            let note1 = Note::new(frontmatter1, content1.to_string());

            let mut frontmatter2 = Frontmatter::default();
            frontmatter2.add_tag(tag1);
            frontmatter2.add_tag(tag2);

            let content2 = "# Test Note 2\nThis is the second test note.";
            let note2 = Note::new(frontmatter2, content2.to_string());

            // Save the notes to disk
            let note_path1 = note1.save(notes_dir, Some("Test Note 1")).unwrap();
            let note_path2 = note2.save(notes_dir, Some("Test Note 2")).unwrap();

            assert!(notes_dir.join(&note_path1).exists());
            assert!(notes_dir.join(&note_path2).exists());

            // Initialize the database
            let db = Database::initialize(notes_dir).await.unwrap();

            // Run the indexing task
            index_notes_with_channel(db.pool().clone(), notes_dir)
                .await
                .unwrap();

            // Verify both notes are in the database
            let filepaths = get_all_note_filepaths(db.pool()).await.unwrap();
            assert_eq!(filepaths.len(), 2);
            assert!(filepaths.contains(&note_path1.to_string_lossy().to_string()));
            assert!(filepaths.contains(&note_path2.to_string_lossy().to_string()));

            // Delete the first note from disk
            fs::remove_file(notes_dir.join(&note_path1)).unwrap();

            // Run the indexing task again
            index_notes_with_channel(db.pool().clone(), notes_dir)
                .await
                .unwrap();

            // Verify only the second note remains in the database
            let filepaths = get_all_note_filepaths(db.pool()).await.unwrap();
            assert_eq!(filepaths.len(), 1);
            assert!(!filepaths.contains(&note_path1.to_string_lossy().to_string()));
            assert!(filepaths.contains(&note_path2.to_string_lossy().to_string()));

            // Test direct deletion using delete_notes_by_filepaths
            let to_delete = vec![note_path2.to_string_lossy().to_string()];
            delete_notes_by_filepaths(db.pool(), &to_delete)
                .await
                .unwrap();

            // Verify no notes remain in the database
            let filepaths = get_all_note_filepaths(db.pool()).await.unwrap();
            assert_eq!(filepaths.len(), 0);
        });
    }

    #[test]
    fn test_search_notes_with_date_range() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Create a tokio runtime for testing
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            // Create year/month directories
            let year_dir = notes_dir.join("2025");
            let month_dir = year_dir.join("05");
            fs::create_dir_all(&month_dir).unwrap();

            // Create three test notes with different creation dates
            // Note 1: Created 2025-05-01
            let date1 = Local.with_ymd_and_hms(2025, 5, 1, 12, 0, 0).unwrap();
            let mut frontmatter1 = Frontmatter::new(date1, vec![]);
            let tag1 = Tag::new("test").unwrap();
            frontmatter1.add_tag(tag1.clone());
            let content1 = "# Test Note 1\nThis is the first test note.";
            let note1 = Note::new(frontmatter1, content1.to_string());

            // Note 2: Created 2025-05-15
            let date2 = Local.with_ymd_and_hms(2025, 5, 15, 12, 0, 0).unwrap();
            let mut frontmatter2 = Frontmatter::new(date2, vec![]);
            frontmatter2.add_tag(tag1.clone());
            let content2 = "# Test Note 2\nThis is the second test note.";
            let note2 = Note::new(frontmatter2, content2.to_string());

            // Note 3: Created 2025-05-30
            let date3 = Local.with_ymd_and_hms(2025, 5, 30, 12, 0, 0).unwrap();
            let mut frontmatter3 = Frontmatter::new(date3, vec![]);
            frontmatter3.add_tag(tag1);
            let content3 = "# Test Note 3\nThis is the third test note.";
            let note3 = Note::new(frontmatter3, content3.to_string());

            // Save the notes to disk
            let _note_path1 = note1.save(notes_dir, Some("Test Note 1")).unwrap();
            let _note_path2 = note2.save(notes_dir, Some("Test Note 2")).unwrap();
            let _note_path3 = note3.save(notes_dir, Some("Test Note 3")).unwrap();

            // Initialize the database
            let db = Database::initialize(notes_dir).await.unwrap();

            // Run the indexing task
            index_notes_with_channel(db.pool().clone(), notes_dir)
                .await
                .unwrap();

            // Test 1: Search with no date filters (should return all 3 notes)
            let (notes, total_count) = db.search_notes("+test", None, None, None).await.unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(total_count, 3);

            // Just verify that we have 3 notes in the results
            assert_eq!(notes.len(), 3, "Should have 3 notes in the results");

            // Test 2: Search for notes before 2025-05-20
            let before_date = Local.with_ymd_and_hms(2025, 5, 20, 0, 0, 0).unwrap();
            let (notes, total_count) = db
                .search_notes("+test", Some(before_date), None, None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 2);
            assert_eq!(total_count, 2);

            // Just verify that we have 2 notes in the results
            assert_eq!(notes.len(), 2, "Should have 2 notes in the results");

            // Test 3: Search for notes after 2025-05-10
            let after_date = Local.with_ymd_and_hms(2025, 5, 10, 0, 0, 0).unwrap();
            let (notes, total_count) = db
                .search_notes("+test", None, Some(after_date), None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 2);
            assert_eq!(total_count, 2);

            // Just verify that we have 2 notes in the results
            assert_eq!(notes.len(), 2, "Should have 2 notes in the results");

            // Test 4: Search with both before and after filters
            let before_date = Local.with_ymd_and_hms(2025, 5, 25, 0, 0, 0).unwrap();
            let after_date = Local.with_ymd_and_hms(2025, 5, 10, 0, 0, 0).unwrap();
            let (notes, total_count) = db
                .search_notes("+test", Some(before_date), Some(after_date), None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(total_count, 1);

            // Just verify that we have 1 note in the results
            assert_eq!(notes.len(), 1, "Should have 1 note in the results");

            // Test 5: Non-overlapping date range (before < after)
            let before_date = Local.with_ymd_and_hms(2025, 5, 5, 0, 0, 0).unwrap();
            let after_date = Local.with_ymd_and_hms(2025, 5, 10, 0, 0, 0).unwrap();
            let (notes, total_count) = db
                .search_notes("+test", Some(before_date), Some(after_date), None)
                .await
                .unwrap();
            assert_eq!(notes.len(), 0);
            assert_eq!(total_count, 0);
        });
    }
}
