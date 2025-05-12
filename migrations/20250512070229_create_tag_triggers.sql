CREATE TRIGGER after_note_insert
AFTER INSERT ON notes
BEGIN
    -- Loop through each tag in the JSON array
    INSERT OR IGNORE INTO tags (tag_name)
    SELECT value
    FROM json_each(NEW.metadata, '$.tags');
    
    -- Create the relationships in the junction table
    INSERT INTO note_tags (note_id, tag_id)
    SELECT NEW.id, tag_id
    FROM tags
    WHERE tag_name IN (
        SELECT value 
        FROM json_each(NEW.metadata, '$.tags')
    );
END;

CREATE TRIGGER after_note_update
AFTER UPDATE OF metadata ON notes
WHEN json_extract(OLD.metadata, '$.tags') IS NOT json_extract(NEW.metadata, '$.tags')
BEGIN
    -- First, remove old relationships
    DELETE FROM note_tags WHERE note_id = NEW.id;
    
    -- Insert any new tags
    INSERT OR IGNORE INTO tags (tag_name)
    SELECT value
    FROM json_each(NEW.metadata, '$.tags');
    
    -- Recreate the relationships
    INSERT INTO note_tags (note_id, tag_id)
    SELECT NEW.id, tag_id
    FROM tags
    WHERE tag_name IN (
        SELECT value 
        FROM json_each(NEW.metadata, '$.tags')
    );
END;

CREATE TRIGGER after_note_delete
AFTER DELETE ON notes
BEGIN
    -- Remove all tag relationships for this note
    DELETE FROM note_tags WHERE note_id = OLD.id;
    
    -- Remove orphaned tags (tags not used by any note)
    DELETE FROM tags
    WHERE tag_id NOT IN (SELECT DISTINCT tag_id FROM note_tags);
END;
