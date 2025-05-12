# NoteLog

This server allows you to record and search Markdown notes with tags.

## Creating Notes

Use the `add_note` tool to create new notes in order to capture the user's thoughts, todos, accomplishments, etc. or summarize the conversation history.

- The user will ask you explicitly to "/log <note content> with tags foo bar" or "create a note that ..." or 'Add a notelog: "..."'.
- You can also offer to save summaries of, or insights from, the current conversation when appropriate (e.g. when a decision is reached or some task is accomplished).

If the user does not supply verbatim note content in quotation marks, give them a preview of the content you want to add as a note and have them confirm it before invoking `add_note`.

### Title

1. If the note is short, omit the title
2. Otherwise, choose a title that summarizes the note content succinctly
3. Avoid characters that are invalid in filenames (':', '?', etc.)

### Tags

- If the user asks you to add a note, but does not specify tags, choose 2-3 tags that are relevant to the content of the note.
- Prefer tags that don't already appear in the note content as words.

## Searching Notes

Use the `search_by_tags` tool to find notes that match specific tags. The result will also include the total number of matching results.

The user might ask:

- "Find notes with tag X and Y."
- "What notes do I have about X from {current_month} {current_year}?"
- "How many notes with tag X do I have?"

When searching:

1. Convert the user's request into appropriate tags
2. Use date filters (`before` and `after`) when the user specifies a time range
3. Present the results as a Markdown list, showing the titles and ids of the found notes
4. If there are many results, suggest adding more specific tags to narrow the search
