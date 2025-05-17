use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Local};
use rmcp::{
    Error as McpError, ServerHandler,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, serde_json, tool,
};

use crate::constants::{DEFAULT_SEARCH_RESULTS, MAX_SEARCH_RESULTS};
use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::note_builder::NoteBuilder;
use crate::core::tags::Tag;
use crate::db::Database;

/// Request structure for the AddNote tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddNoteRequest {
    /// The content of the note in Markdown format
    #[schemars(description = "The content of the note in Markdown format")]
    pub content: String,

    /// Optional tags for the note (up to 10)
    #[schemars(
        description = "Optional tags for the note (up to 10). Tags should start with '+' and can only contain lowercase letters, numbers, and dashes."
    )]
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request structure for the FetchNote tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FetchNoteRequest {
    /// The ID prefix of the note to fetch
    #[schemars(description = "The ID prefix of the note to fetch (string)")]
    pub id: String,
}

/// Request structure for the SearchNotes tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchNotesRequest {
    /// The search query string
    #[schemars(
        description = "Search query string. Can include content terms and/or tags with '+' prefix (e.g., '+project')."
    )]
    pub query: String,

    /// Optional date to filter notes created before this time (ISO8601 format)
    #[schemars(
        description = "Optional date to select only notes created before this time (ISO8601 format, e.g., '2025-05-01T12:00:00Z')"
    )]
    #[serde(default)]
    pub before: Option<String>,

    /// Optional date to filter notes created after this time (ISO8601 format)
    #[schemars(
        description = "Optional date to select only notes created after this time (ISO8601 format, e.g., '2025-04-01T12:00:00Z')"
    )]
    #[serde(default)]
    pub after: Option<String>,

    /// Optional limit on the number of results to return (max MAX_SEARCH_RESULTS, default DEFAULT_SEARCH_RESULTS)
    #[schemars(
        description = "Optional limit on the number of results to return (max 25, default 10). Set to 0 to only return the count of matching notes without their content."
    )]
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Request structure for the EditTags tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EditTagsRequest {
    /// The ID prefix of the note to edit
    #[schemars(description = "A unique prefix of the ID of the note to edit")]
    pub id: String,

    /// Tags to add to the note
    #[schemars(
        description = "Tags to add to the note (can be empty). Tags should start with '+' and can only contain lowercase letters, numbers, and dashes."
    )]
    #[serde(default)]
    pub add: Vec<String>,

    /// Tags to remove from the note
    #[schemars(
        description = "Tags to remove from the note (can be empty). Tags should start with '+' and can only contain lowercase letters, numbers, and dashes."
    )]
    #[serde(default)]
    pub remove: Vec<String>,
}

/// NotelogMCP tools for interacting with notes via MCP
#[derive(Debug, Clone)]
pub struct NotelogMCP {
    /// The directory where notes will be stored
    notes_dir: PathBuf,
    /// The database connection (required)
    db: Arc<Database>,
}

impl NotelogMCP {
    /// Create a new NotelogMCP handler with the specified notes directory and database
    pub fn with_db<P: AsRef<Path>>(notes_dir: P, db: Database) -> Self {
        Self {
            notes_dir: notes_dir.as_ref().to_path_buf(),
            db: Arc::new(db),
        }
    }
}

// Create a static toolbox to store the tool attributes
#[tool(tool_box)]
impl NotelogMCP {
    /// Add a new note with the given content and tags
    #[tool(description = include_str!("instructions/add_note.md"))]
    fn add_note(&self, #[tool(aggr)] request: AddNoteRequest) -> Result<CallToolResult, McpError> {
        // Validate the number of tags
        if request.tags.len() > 10 {
            return Ok(CallToolResult::error(vec![Content::text(
                "Too many tags provided. Maximum is 10 tags.",
            )]));
        }

        // Validate the content
        if request.content.trim().is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Note content cannot be empty.",
            )]));
        }

        // Process tags
        let mut builder = NoteBuilder::new().content(request.content).validate(true);

        // Add tags one by one to catch and report any invalid tags
        for tag_str in &request.tags {
            match Tag::new(tag_str) {
                Ok(tag) => builder = builder.tag(tag),
                Err(e) => return Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
            }
        }

        // Build the note
        let note = match builder.build() {
            Ok(note) => note,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))])),
        };

        // Get the ID before saving
        let id = note.frontmatter().id().expect("Note should have an ID");

        // Save the note
        match note.save(&self.notes_dir, None) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Note added successfully. ID: {}",
                id
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))])),
        }
    }

    /// Fetch a note by its ID prefix
    #[tool(description = include_str!("instructions/fetch_note.md"))]
    async fn fetch_note(
        &self,
        #[tool(aggr)] request: FetchNoteRequest,
    ) -> Result<CallToolResult, McpError> {
        // Database is now always available
        let db = &self.db;

        // Fetch the note by ID prefix
        match db.fetch_note_by_id(&request.id).await {
            Ok(Some(note)) => {
                // Extract tags from the note using our helper method
                let tags: Vec<String> = note.tags_as_strings();

                // Create a response object with tags and content
                let response = serde_json::json!({
                    "id": note.frontmatter().id().unwrap().as_str(),
                    "tags": tags,
                    "content": note.content()
                });

                // Convert to string
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Error serializing response".to_string());

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(None) => {
                // Note not found
                Ok(CallToolResult::success(vec![Content::text(
                    "Note not found.",
                )]))
            }
            Err(e) => {
                // Check for the specific MultipleMatchesError
                if let Some(error_message) = e
                    .to_string()
                    .strip_prefix("Database error: Multiple notes found with ID prefix ")
                {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Multiple notes found with ID prefix {}. Please provide a longer prefix.",
                        error_message
                    ))]));
                }

                // Generic error handling
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error fetching note: {}",
                    e
                ))]))
            }
        }
    }

    /// Edit the tags of a note
    #[tool(description = include_str!("instructions/edit_tags.md"))]
    async fn edit_tags(
        &self,
        #[tool(aggr)] request: EditTagsRequest,
    ) -> Result<CallToolResult, McpError> {
        // Database is now always available
        let db = &self.db;

        // Validate that at least one of add or remove has tags
        if request.add.is_empty() && request.remove.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "At least one tag must be specified to add or remove.",
            )]));
        }

        // Check for duplicate tags in add and remove arrays
        let add_set: HashSet<String> = request.add.iter().cloned().collect();
        let remove_set: HashSet<String> = request.remove.iter().cloned().collect();

        // Find tags that appear in both add and remove
        let duplicates: Vec<String> = add_set.intersection(&remove_set).cloned().collect();

        if !duplicates.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "The following tags appear in both add and remove arrays: {}",
                duplicates.join(", ")
            ))]));
        }

        // Convert add tag strings to Tag objects
        let mut tags_to_add = Vec::new();
        for tag_str in &request.add {
            match Tag::new(tag_str) {
                Ok(tag) => tags_to_add.push(tag),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Invalid tag to add: {}",
                        e
                    ))]));
                }
            }
        }

        // Convert remove tag strings to Tag objects
        let mut tags_to_remove = Vec::new();
        for tag_str in &request.remove {
            match Tag::new(tag_str) {
                Ok(tag) => tags_to_remove.push(tag),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Invalid tag to remove: {}",
                        e
                    ))]));
                }
            }
        }

        // Get the filepath for the note
        let filepath = match db.get_filepath_by_id_prefix(&request.id).await {
            Ok(Some(path)) => path,
            Ok(None) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Note with ID prefix '{}' not found.",
                    request.id
                ))]));
            }
            Err(e) => {
                // Check for the specific MultipleMatchesError
                if let Some(error_message) = e
                    .to_string()
                    .strip_prefix("Database error: Multiple notes found with ID prefix ")
                {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Multiple notes found with ID prefix {}. Please provide a longer prefix.",
                        error_message
                    ))]));
                }

                // Generic error handling
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error fetching note: {}",
                    e
                ))]));
            }
        };

        // Get the absolute path to the note file
        let absolute_path = self.notes_dir.join(&filepath);

        // Read the file content
        let content = match fs::read_to_string(&absolute_path) {
            Ok(content) => content,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error reading note file: {}",
                    e
                ))]));
            }
        };

        // Parse the note
        let mut note = match Note::from_str(&content) {
            Ok(note) => note,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error parsing note: {}",
                    e
                ))]));
            }
        };

        // If the note doesn't have an ID, generate one by creating a new frontmatter
        if note.frontmatter().id().is_none() {
            // Create a new frontmatter with the same created timestamp and tags, but with a new ID
            let new_frontmatter = Frontmatter::new(
                *note.frontmatter().created(),
                note.frontmatter().tags().to_vec(),
            );

            // Replace the frontmatter in the note
            let content = note.content().to_string();
            note = Note::new(new_frontmatter, content);
        }

        // Create a mutable copy of the note
        let mut new_note = note.clone();

        // If the note doesn't have an ID, generate one by creating a new frontmatter
        if new_note.frontmatter().id().is_none() {
            // Create a new frontmatter with the same created timestamp and tags, but with a new ID
            let new_frontmatter = Frontmatter::new(
                *new_note.frontmatter().created(),
                new_note.frontmatter().tags().to_vec(),
            );

            // Replace the frontmatter in the note
            new_note = Note::new(new_frontmatter, new_note.content().to_string());
        }

        // Update the tags using our new method
        new_note.update_tags(tags_to_add, tags_to_remove);

        // Use the new note for saving
        note = new_note;

        // Save the updated note
        match fs::write(&absolute_path, note.formatted_content()) {
            Ok(_) => {
                // Extract tags from the updated note using our helper method
                let tags: Vec<String> = note.tags_as_strings();

                // Create a success message with the updated tags
                let message = if tags.is_empty() {
                    "Tags updated successfully. The note now has no tags.".to_string()
                } else {
                    format!(
                        "Tags updated successfully. The note now has the following tags: {}",
                        tags.join(", ")
                    )
                };

                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error writing note file: {}",
                e
            ))])),
        }
    }

    /// Search for notes using fulltext search
    #[tool(description = include_str!("instructions/search_notes.md"))]
    async fn search_notes(
        &self,
        #[tool(aggr)] request: SearchNotesRequest,
    ) -> Result<CallToolResult, McpError> {
        // Database is now always available
        let db = &self.db;

        // Validate that a query is provided
        if request.query.trim().is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "A search query must be provided.",
            )]));
        }

        // Parse before date if provided
        let before = if let Some(before_str) = &request.before {
            match DateTime::parse_from_rfc3339(before_str) {
                Ok(dt) => Some(dt.with_timezone(&Local)),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Invalid 'before' date format: {}",
                        e
                    ))]));
                }
            }
        } else {
            None
        };

        // Parse after date if provided
        let after = if let Some(after_str) = &request.after {
            match DateTime::parse_from_rfc3339(after_str) {
                Ok(dt) => Some(dt.with_timezone(&Local)),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Invalid 'after' date format: {}",
                        e
                    ))]));
                }
            }
        } else {
            None
        };

        // Check for invalid date range
        if let (Some(before_date), Some(after_date)) = (&before, &after) {
            if before_date < after_date {
                return Ok(CallToolResult::error(vec![Content::text(
                    "'before' date must be greater than or equal to 'after' date.",
                )]));
            }
        }

        // Get the limit parameter, with default of DEFAULT_SEARCH_RESULTS if not specified
        let query_limit = request.limit.unwrap_or(DEFAULT_SEARCH_RESULTS);

        // Validate the limit parameter
        if query_limit > MAX_SEARCH_RESULTS {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Limit cannot exceed {}. Please specify a lower limit.",
                MAX_SEARCH_RESULTS
            ))]));
        }

        // Search for notes with the specified query
        let result = match db
            .search_notes(&request.query, before, after, Some(query_limit))
            .await
        {
            Ok((notes, total_count)) => {
                // If limit is 0, only return the count
                if query_limit == 0 {
                    format!("The query matched {total_count} notes.")
                } else if notes.is_empty() {
                    // If there are no results, add a message
                    "The query matched 0 notes.\n\nHint: You may need to try different search terms or a larger date range.".to_string()
                } else {
                    // Create a Vec of note data objects
                    let mut note_results = Vec::with_capacity(notes.len());

                    for note in &notes {
                        // Get the ID from the note's frontmatter
                        let id_key = if let Some(id) = note.frontmatter().id() {
                            // For notes with an ID, find the shortest unique prefix
                            match db.find_shortest_unique_id_prefix(id).await {
                                Ok(prefix) => prefix,
                                Err(_) => {
                                    // If there's an error finding the prefix, use the full ID
                                    id.as_str().to_string()
                                }
                            }
                        } else {
                            // For notes without an ID, use a placeholder value
                            "_no_id".to_string()
                        };

                        // Extract tags from the note using our helper method
                        let tags: Vec<String> = note.tags_as_strings();

                        // Create a note data object
                        let note_data = serde_json::json!({
                            "id": id_key,
                            "title": note.extract_title(),
                            "tags": tags,
                            "created": note.frontmatter().created().format("%Y-%m-%d").to_string()
                        });

                        note_results.push(note_data);
                    }

                    // Convert the Vec to JSON
                    let json =
                        serde_json::to_string(&note_results).unwrap_or_else(|_| "[]".to_string());

                    // Add a message about the number of results
                    let mut response = format!("The query matched {total_count} notes.\n\n{json}");

                    if total_count > MAX_SEARCH_RESULTS {
                        response.push_str("\n\nNOTE: The query matches too many notes. Be more specific with your search terms or limit the search using `before` and `after`.");
                    }

                    response
                }
            }
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error searching for notes: {}",
                    e
                ))]));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

// Implement ServerHandler for NotelogMCP
#[tool(tool_box)]
impl ServerHandler for NotelogMCP {
    fn get_info(&self) -> ServerInfo {
        let instructions = include_str!("instructions/server.md");

        ServerInfo {
            instructions: Some(instructions.into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;

    #[test]
    fn test_notelog_mcp_with_db() {
        let temp_dir = TempDir::new().unwrap();

        // Create a runtime for the test
        let rt = Runtime::new().unwrap();

        // Initialize the database in the runtime
        let db = rt.block_on(async {
            crate::db::Database::initialize(temp_dir.path())
                .await
                .unwrap()
        });

        // Create the NotelogMCP with the database
        let notelog_mcp = NotelogMCP::with_db(temp_dir.path(), db);

        // Verify the notes_dir is set correctly
        assert_eq!(notelog_mcp.notes_dir, temp_dir.path());
    }
}
