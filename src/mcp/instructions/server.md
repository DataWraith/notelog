# NoteLog

This server allows you to record, search, and manage Markdown notes with tags.

## Creating Notes

Use the `add_note` tool to create new notes in order to capture the user's thoughts, todos, accomplishments, etc. or summarize the conversation history.

The user will ask you explicitly to "/log <note content> +tag1 +tag2" or "create a note that ..." or 'Add a notelog with tags X, Y, Z: "<note content>"'.

## Searching Notes

Use the `search_notes` tool to find notes using tag and/or fulltext search.

The user might ask:

- "Find notes with tag +project"
- "Find notes containing 'project plan' with tag +important"
- "Search for notes tagged +sqlite and +til from May 2025"
- "How many notes tagged +todo do I have?"

## Fetching Notes

Use the `fetch_note` tool to retrieve a specific note by its ID. This is useful when the user wants to see the full content of a note they found through search.

## Editing Tags

Use the `edit_tags` tool to add or remove tags from an existing note. This allows the user to organize their notes better over time.

The user might ask:

- "Add tags +important and +project to note abc123"
- "Remove the +draft tag from note xyz456"
- "Mark note def789 as done" (This should remove the +todo tag and add the +done tag)
