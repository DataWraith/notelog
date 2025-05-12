CREATE TRIGGER after_note_insert
AFTER INSERT ON notes
BEGIN
    -- Insert new tags if they don't exist
    INSERT OR IGNORE INTO tags (tag_name, usage_count)
    SELECT 
        value, 
        0
    FROM json_each(NEW.metadata, '$.tags');
    
    -- Create relationships in the junction table
    INSERT INTO note_tags (note_id, tag_id)
    SELECT NEW.id, tag_id
    FROM tags
    WHERE tag_name IN (
        SELECT value 
        FROM json_each(NEW.metadata, '$.tags')
    );
    
    -- Update the usage counts for all affected tags
    UPDATE tags
    SET 
        usage_count = usage_count + 1
    WHERE tag_id IN (
        SELECT tag_id
        FROM note_tags
        WHERE note_id = NEW.id
    );
END;

CREATE TRIGGER after_note_update
AFTER UPDATE OF metadata ON notes
WHEN json_extract(OLD.metadata, '$.tags') IS NOT json_extract(NEW.metadata, '$.tags')
BEGIN
    -- First, decrement the count for tags that will be removed
    UPDATE tags
    SET usage_count = usage_count - 1
    WHERE tag_id IN (
        SELECT it.tag_id
        FROM note_tags it
        WHERE it.note_id = NEW.id
        AND it.tag_id NOT IN (
            SELECT t.tag_id
            FROM tags t
            WHERE t.tag_name IN (
                SELECT value 
                FROM json_each(NEW.metadata, '$.tags')
            )
        )
    );
    
    -- Remove old relationships
    DELETE FROM note_tags WHERE note_id = NEW.id;
    
    -- Insert any new tags
    INSERT OR IGNORE INTO tags (tag_name, usage_count)
    SELECT 
        value, 
        0
    FROM json_each(NEW.metadata, '$.tags');
    
    -- Recreate the relationships
    INSERT INTO note_tags (note_id, tag_id)
    SELECT NEW.id, tag_id
    FROM tags
    WHERE tag_name IN (
        SELECT value 
        FROM json_each(NEW.metadata, '$.tags')
    );
    
    -- Increment the count for all current tags on this note
    UPDATE tags
    SET 
        usage_count = usage_count + 1
    WHERE tag_id IN (
        SELECT tag_id
        FROM note_tags
        WHERE note_id = NEW.id
    );
END;

CREATE TRIGGER after_note_delete
AFTER DELETE ON notes
BEGIN
    -- Decrement usage count for all tags used by this note
    UPDATE tags
    SET usage_count = usage_count - 1
    WHERE tag_id IN (
        SELECT tag_id FROM note_tags WHERE note_id = OLD.id
    );
    
    -- Remove the tag relationships
    DELETE FROM note_tags WHERE note_id = OLD.id;
    
    -- Remove tags with usage count of 0
    DELETE FROM tags WHERE usage_count <= 0;
END;
