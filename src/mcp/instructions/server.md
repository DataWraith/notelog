# NoteLog

This server allows you to record and search Markdown notes with tags.

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
