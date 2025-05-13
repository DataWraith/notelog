//! Constants used throughout the application

/// Maximum file size for notes in KiB (50 KiB)
pub const MAX_FILE_SIZE_KIB: usize = 50;

/// Maximum number of search results that can be returned (25)
pub const MAX_SEARCH_RESULTS: usize = 25;

/// Default number of search results to return (10)
pub const DEFAULT_SEARCH_RESULTS: usize = 10;

/// Maximum file size in bytes (MAX_FILE_SIZE_KIB * 1024)
pub const MAX_FILE_SIZE_BYTES: usize = MAX_FILE_SIZE_KIB * 1024;
