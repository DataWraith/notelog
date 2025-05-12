-- Primary table
CREATE TABLE notes (
    id INTEGER PRIMARY KEY,
    filepath TEXT UNIQUE NOT NULL,
    mtime TEXT NOT NULL,
    metadata TEXT NOT NULL,
    content TEXT NOT NULL,
);

-- Table for storing unique tags
CREATE TABLE tags (
    tag_id INTEGER PRIMARY KEY,
    tag_name TEXT UNIQUE
);

-- Junction table for the many-to-many relationship
CREATE TABLE note_tags (
    note_id INTEGER,
    tag_id INTEGER,
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notes(id),
    FOREIGN KEY (tag_id) REFERENCES tags(tag_id)
);

-- Create indexes for performance
CREATE INDEX idx_note_tags_tag ON note_tags(tag_id);
CREATE INDEX idx_tags_name ON tags(tag_name);
