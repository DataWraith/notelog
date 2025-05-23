# fetch_note

To fetch a specific note by its ID prefix:

1. Provide the ID prefix of the note you want to retrieve
   - The ID prefix can be as short as 2 characters.
     Note that you cannot fetch notes with an ID that starts with an underscore.
   - You can get note IDs from the `search_notes` tool results
   - If multiple notes match the prefix, you'll need to provide a longer prefix

Example:
```json
{
  "id": "a1b2"
}
```

The response will be a JSON object with the following fields:
- `id`: The full ID of the note
- `tags`: An array of tag strings (without the '+' prefix)
- `content`: The full content of the note in Markdown format

If the note is not found, the response will be: "Note not found."