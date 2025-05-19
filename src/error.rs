use std::io;
use thiserror::Error;

use crate::constants::MAX_FILE_SIZE_KIB;

/// Specific error type for tag validation errors
#[derive(Error, Debug)]
pub enum TagError {
    #[error("Tag cannot be empty")]
    Empty,

    #[error("Tag '{0}' cannot start or end with a dash")]
    InvalidDashPosition(String),

    #[error("Tag '{0}' can only contain lowercase letters, numbers, and dashes")]
    InvalidCharacters(String),
}

/// Specific error type for Id validation errors
#[derive(Error, Debug)]
pub enum IdError {
    #[error("Id cannot be empty")]
    Empty,

    #[error("Id must be exactly 16 characters, got {0}")]
    InvalidLength(usize),

    #[error("Id '{0}' can only contain lowercase letters and numbers")]
    InvalidCharacters(String),
}

/// Specific error type for frontmatter validation errors
#[derive(Error, Debug)]
pub enum FrontmatterError {
    #[error("Invalid YAML format: {0}")]
    InvalidYaml(String),

    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),
}

/// Specific error type for database operations
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Database migration error: {0}")]
    Migration(String),

    #[error("Database query error: {0}")]
    Query(String),

    #[error("Database serialization error: {0}")]
    Serialization(String),

    #[error("File monitoring error: {0}")]
    Monitoring(String),

    #[error("Multiple notes found with ID prefix '{0}': {1} matches")]
    MultipleMatches(String, usize),
}

#[derive(Error, Debug)]
pub enum NotelogError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Notes directory does not exist or is not a directory: {0}")]
    NotesDirectoryNotFound(String),

    #[error("Notes directory is not writable: {0}")]
    NotesDirectoryNotWritable(String),

    #[error("Note content is empty")]
    EmptyContent,

    #[error("Note content is too large (> {MAX_FILE_SIZE_KIB}KiB)")]
    ContentTooLarge,

    #[error("Note content contains null bytes")]
    ContentContainsNullBytes,

    #[error("Note content contains invalid UTF-8")]
    InvalidUtf8Content,

    #[error("Cannot use both stdin and file input")]
    ConflictingInputMethods,

    #[error("Cannot use both stdin and command line arguments")]
    ConflictingStdinAndArgs,

    #[error("Failed to launch editor: {0}")]
    EditorLaunchFailed(String),

    #[error("Invalid options for 'mcp' command: only the global --notes-dir option is allowed.")]
    InvalidMcpOptions,

    #[error("Invalid options for 'last' command: only the global --notes-dir and --print options are allowed.")]
    InvalidLastOptions,

    #[error("No valid note found")]
    NoValidNoteFound,

    #[error("MCP server error: {0}")]
    McpServerError(String),

    #[error("Tag validation error: {0}")]
    TagError(#[from] TagError),

    #[error("Id validation error: {0}")]
    IdError(#[from] IdError),

    #[error("Frontmatter validation error: {0}")]
    FrontmatterError(#[from] FrontmatterError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Path error: {0}")]
    PathError(String),
}

pub type Result<T> = std::result::Result<T, NotelogError>;
