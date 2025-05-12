To search for notes by tags, provide:

1. An array of tags to search for (at least one tag is required)
2. Optional date filters to narrow down the search:
   - `before`: Find notes created before this date (ISO8601 format, e.g., '2025-05-01T12:00:00Z')
   - `after`: Find notes created after this date (ISO8601 format, e.g., '2025-04-01T12:00:00Z')

Valid tags:
- Must start with a '+' prefix (e.g., +project)
- Can only contain lowercase letters, numbers, and dashes
- Cannot start or end with a dash

The search will return notes that have ALL the specified tags (AND logic).

Example:
```json
{
  "tags": ["+project", "+important"],
  "before": "2025-05-01T00:00:00Z",
  "after": "2025-04-01T00:00:00Z"
}
```

This will find all notes that have both the "project" and "important" tags, created between April 1st and May 1st, 2025.

The response will be a JSON object mapping note IDs to titles, limited to 25 results.
