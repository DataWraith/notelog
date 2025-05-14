# NoteLog

This server allows you to record and search Markdown notes with tags.

## Creating Notes

Use the `add_note` tool to create new notes in order to capture the user's thoughts, todos, accomplishments, etc. or summarize the conversation history.

- The user will ask you explicitly to "/log <note content> with tags foo bar" or "create a note that ..." or 'Add a notelog: "..."'.
- Do not editorialize or summarize when the user supplies note content themselves. You may still add a title and tags when appropriate or fix obvious spelling mistakes.
- You can offer to save summaries of, or insights from, the current conversation from time to time (e.g. when a decision is reached or some task is accomplished).

### Title

1. If the note is short, omit the title
2. Otherwise, choose a title that summarizes the note content succinctly
3. Avoid characters that are invalid in filenames (':', '?', etc.)

### Tags

- If the user asks you to add a note, but does not specify tags, choose 2-3 tags that are relevant to the content of the note.
- Prefer tags that don't already appear in the note content as words.

## Searching Notes

Use the `search_notes` tool to find notes using fulltext search. This allows searching both note content and tags.

The user might ask:

- "Search for notes about meetings"
- "Find notes containing 'project plan' with tag +important"
- "Look for notes mentioning databases from May 2025"
- "Find notes with tag +project"
- "Search for notes tagged +sqlite and +til from May 2025"
- "How many notes tagged +todo do I have?"

When using search:

1. Convert the user's request into a search query
2. Include tag prefixes (e.g., "+project") when the user wants to search for specific tags
3. Use date filters (`before` and `after`) when the user specifies a time range
4. Present the results as a Markdown list, showing the titles and IDs of the found notes
5. If there are many results, suggest refining the search with more specific terms or tags
