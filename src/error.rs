use std::io;
use thiserror::Error;

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
    ConnectionError(String),

    #[error("Database migration error: {0}")]
    MigrationError(String),

    #[error("Database query error: {0}")]
    QueryError(String),

    #[error("Database serialization error: {0}")]
    SerializationError(String),
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

    #[error("Note content is too large (> 50KiB)")]
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

    #[error("MCP server error: {0}")]
    McpServerError(String),

    #[error("Tag validation error: {0}")]
    TagError(#[from] TagError),

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
