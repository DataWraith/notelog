To fetch a specific note by its ID:

1. Provide the ID of the note you want to retrieve
   - The ID must be a valid integer (i64)
   - You can get note IDs from the `search_by_tags` tool results

Example:
```json
{
  "id": 42
}
```

The response will be a JSON object with the following fields:
- `tags`: An array of tag strings (without the '+' prefix)
- `content`: The full content of the note in Markdown format

If the note is not found, the response will be: "Note not found."
