NoteLog is a command-line tool for recording notes as Markdown files with YAML frontmatter, organized by year and month. 
Use the AddNote tool to create new notes in order to capture the user's thoughts, todos, accomplishments or summarize the conversation history.

To add a note, provide:
1. Markdown content for your note, beginning with a level 1 heading (e.g., # My Note Title)
2. Optional tags (up to 10) that are relevant to the content

Valid tags:
- Must start with a '+' prefix (e.g., +project)
- Can only contain lowercase letters, numbers, and dashes
- Cannot start or end with a dash

Example JSON:
{
  "content": "# Meeting Notes\nDiscussed project timeline and next steps.",
  "tags": ["+meeting", "+project"]
}
