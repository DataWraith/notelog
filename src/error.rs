use std::io;
use thiserror::Error;

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

    #[error("Failed to parse YAML front matter: {0}")]
    YamlParseError(String),
}

pub type Result<T> = std::result::Result<T, NotelogError>;
