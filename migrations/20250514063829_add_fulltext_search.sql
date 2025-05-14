-- Create a virtual table using FTS5 for full-text search
CREATE VIRTUAL TABLE notes_fts USING fts5(
    content,           -- The note content
    tags               -- Space-separated tags with + prefix
);

-- Populate the FTS table with existing data
INSERT INTO notes_fts(rowid, content, tags)
SELECT
    n.id,
    n.content,
    COALESCE(
        (
            SELECT group_concat('+' || value, ' ')
            FROM json_each(n.metadata, '$.tags')
            WHERE json_valid(n.metadata)
        ),
        ''
    )
FROM notes n
WHERE NOT EXISTS (SELECT 1 FROM notes_fts WHERE rowid = n.id);

-- Insert trigger for FTS
CREATE TRIGGER notes_after_insert_fts AFTER INSERT ON notes BEGIN
    -- Extract tags from metadata JSON and format them with + prefix
    INSERT INTO notes_fts(rowid, content, tags)
    VALUES (
        NEW.id,
        NEW.content,
        (
            SELECT group_concat('+' || value, ' ')
            FROM json_each(NEW.metadata, '$.tags')
        )
    );
END;

-- Update trigger for content
CREATE TRIGGER notes_after_update_content_fts AFTER UPDATE OF content ON notes BEGIN
    UPDATE notes_fts
    SET content = NEW.content
    WHERE rowid = NEW.id;
END;

-- Update trigger for metadata (tags)
CREATE TRIGGER notes_after_update_metadata_fts AFTER UPDATE OF metadata ON notes BEGIN
    -- Extract tags from metadata JSON and format them with + prefix
    UPDATE notes_fts
    SET tags = (
        SELECT group_concat('+' || value, ' ')
        FROM json_each(NEW.metadata, '$.tags')
    )
    WHERE rowid = NEW.id;
END;

-- Delete trigger
CREATE TRIGGER notes_after_delete_fts AFTER DELETE ON notes BEGIN
    DELETE FROM notes_fts WHERE rowid = OLD.id;
END;