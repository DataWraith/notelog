# search_notes

To search for notes, provide a query string and optional parameters:

1. `query`: A search string to find matching notes (required)
   - Use `+tag` syntax to search for specific tags (e.g., `+project`)
   - Combine content and tag searches (e.g., `meeting notes +project`)
   - You can combine terms with AND, OR and NOT operators (parenthesize as needed)
   - To search for a phrase, enclose it in "quotation marks"

2. Optional date filters to narrow down the search:
   - `before`: Find notes created before this date (ISO8601 format, e.g., '2025-05-01T12:00:00Z')
   - `after`: Find notes created after this date (ISO8601 format, e.g., '2025-04-01T12:00:00Z')

3. Optional limit on the number of results to return:
   - `limit`: Maximum number of notes to return (default: 10, max: 25)
   - Set `limit` to 0 to only return the count of matching notes without their content

Tag search syntax:
- Tags must start with a '+' prefix (e.g., +project)
- Can only contain lowercase letters, numbers, and dashes
- Cannot end with a dash

Results are ordered by relevance to your query, with the most relevant notes appearing first.

Example:
```json
{
  "query": "meeting notes +project",
  "before": "2025-05-01T00:00:00Z",
  "after": "2025-04-01T00:00:00Z",
  "limit": 15
}
```

This will find all notes containing "meeting notes" that also have the "project" tag, created between April 1st and May 1st, 2025, and return up to 15 results.

The response will be a JSON array of note objects with the following fields:

- `id`: The shortest unique prefix of the note's ID
- `title`: The title extracted from the note content
- `tags`: An array of tags associated with the note
- `created`: The creation date

When displaying the results, create a Markdown list or Markdown table.
The output must contain the `id` and `title` fields at a minimum.
