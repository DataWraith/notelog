use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Local};
use rmcp::{
    Error as McpError, ServerHandler,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, serde_json, tool,
};

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
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

/// Request structure for the SearchByTags tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchByTagsRequest {
    /// Tags to search for
    #[schemars(
        description = "Tags to search for. Tags should start with '+' and can only contain lowercase letters, numbers, and dashes."
    )]
    pub tags: Vec<String>,

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
    #[tool(description = include_str!("add_note_instructions.md"))]
    fn add_note(&self, #[tool(aggr)] request: AddNoteRequest) -> Result<CallToolResult, McpError> {
        // Validate the number of tags
        if request.tags.len() > 10 {
            return Ok(CallToolResult::error(vec![Content::text(
                "Too many tags provided. Maximum is 10 tags.",
            )]));
        }

        // Convert tag strings to Tag objects
        let mut tags = Vec::new();
        for tag_str in &request.tags {
            match Tag::new(tag_str) {
                Ok(tag) => tags.push(tag),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(e.to_string())]));
                }
            }
        }

        // Validate the content
        if request.content.trim().is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Note content cannot be empty.",
            )]));
        }

        // Create a frontmatter with the tags
        let frontmatter = Frontmatter::with_tags(tags);

        // Create a note with the frontmatter and content
        let note = Note::new(frontmatter, request.content);

        // Save the note
        match note.save(&self.notes_dir, None) {
            Ok(relative_path) => {
                // Get the absolute path to the note file
                let absolute_path = self.notes_dir.join(&relative_path);

                // Process the note file to add it to the database
                // We use tokio::task::block_in_place since process_note_file is an async function
                // but add_note is not async
                if let Err(e) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        crate::db::process_note_file(
                            self.db.pool(),
                            &self.notes_dir,
                            &absolute_path,
                        )
                        .await
                    })
                }) {
                    // Return an error if the note couldn't be added to the database
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Error adding note to database: {}",
                        e
                    ))]));
                }

                // Return the relative path as the success message
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Note added successfully: {}",
                    relative_path.display()
                ))]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                e
            ))])),
        }
    }

    /// Search for notes by tags
    #[tool(description = include_str!("search_by_tags_instructions.md"))]
    async fn search_by_tags(
        &self,
        #[tool(aggr)] request: SearchByTagsRequest,
    ) -> Result<CallToolResult, McpError> {
        // Database is now always available
        let db = &self.db;

        // Validate that tags are provided
        if request.tags.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "At least one tag must be provided.",
            )]));
        }

        // Convert tag strings to Tag objects
        let mut tags = Vec::new();
        for tag_str in &request.tags {
            match Tag::new(tag_str) {
                Ok(tag) => tags.push(tag.as_str().to_string()),
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(e.to_string())]));
                }
            }
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

        // Convert tags to string slices for the database query
        let tag_strs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();

        // Search for notes with the specified tags
        let result = match db.search_by_tags(&tag_strs, before, after, Some(25)).await {
            Ok((notes, total_count)) => {
                // Create a map of note ID to title
                let mut id_to_title = BTreeMap::new();
                for (id, note) in &notes {
                    let title = note.extract_title();
                    id_to_title.insert(id.to_string(), title);
                }

                // Convert the map to JSON
                let json = serde_json::to_string(&id_to_title).unwrap_or_else(|_| "{}".to_string());

                // Add a message if there are more results than the limit
                let mut response = json;
                if total_count > notes.len() {
                    response.push_str(&format!(
                        "\n\nNOTE: The query matches {} notes. Be more specific by adding more tags or limit the search using `before` and `after`.",
                        total_count
                    ));
                }

                // If there are no results, add a message
                if notes.is_empty() {
                    response = "{}\n\nNo results found. You may need to specify fewer tags or a larger date range.".to_string();
                }

                response
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
        let instructions = include_str!("server_instructions.md");

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
