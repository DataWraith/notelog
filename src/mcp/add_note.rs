use std::path::Path;

use rmcp::{
    Error as McpError,
    ServerHandler,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool,
};

use crate::core::frontmatter::Frontmatter;
use crate::core::note::Note;
use crate::core::tags::Tag;

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

/// AddNote tool for creating notes via MCP
#[derive(Debug, Clone)]
pub struct AddNote {
    /// The directory where notes will be stored
    notes_dir: String,
}

impl AddNote {
    /// Create a new AddNote handler with the specified notes directory
    pub fn new<P: AsRef<Path>>(notes_dir: P) -> Self {
        Self {
            notes_dir: notes_dir.as_ref().to_string_lossy().to_string(),
        }
    }
}

// Create a static toolbox to store the tool attributes
#[tool(tool_box)]
impl AddNote {
    /// Add a new note with the given content and tags
    #[tool(description = include_str!("add_note_instructions.md"))]
    fn add_note(&self, #[tool(aggr)] request: AddNoteRequest) -> Result<CallToolResult, McpError> {
        // Validate the number of tags
        if request.tags.len() > 10 {
            return Ok(CallToolResult::error(
                vec![Content::text("Too many tags provided. Maximum is 10 tags.")],
            ));
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
            return Ok(CallToolResult::error(vec![Content::text("Note content cannot be empty.")]));
        }

        // Create a frontmatter with the tags
        let frontmatter = Frontmatter::with_tags(tags);

        // Create a note with the frontmatter and content
        let note = Note::new(frontmatter, request.content);

        // Save the note
        match note.save(Path::new(&self.notes_dir), None) {
            Ok(note_path) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Note added successfully: {}",
                note_path
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))])),
        }
    }
}

// Implement ServerHandler for AddNote
#[tool(tool_box)]
impl ServerHandler for AddNote {
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

    #[test]
    fn test_add_note_new() {
        let temp_dir = TempDir::new().unwrap();
        let add_note = AddNote::new(temp_dir.path());
        assert_eq!(add_note.notes_dir, temp_dir.path().to_string_lossy());
    }
}
