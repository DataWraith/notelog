# Edit Tags

Edit the tags of a note by adding and/or removing tags.

## Arguments

- `id` (string): A unique prefix of the ID of the note to edit
- `add` (array of strings): Tags to add to the note (can be empty)
- `remove` (array of strings): Tags to remove from the note (can be empty)

Valid tags:
- Must start with a '+' prefix (e.g., +project)
- Can only contain lowercase letters, numbers, and dashes
- Cannot end with a dash

## Example

```json
{
  "id": "abc123",
  "add": ["+project", "+important"],
  "remove": ["+draft"]
}
```

This will add the tags "project" and "important" to the note with ID starting with "abc123", and remove the tag "draft" if it exists.
