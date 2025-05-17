# add_note

To add a note, provide:

1. Markdown content for the note, beginning with a level 1 heading (e.g., "# Note Title\n\nNote content goes here")
2. Optional tags (up to 10) that are relevant to the content

## Title

If the user provides a title (e.g. the note starts with a markdown header) use that.
If not, generate a title based on the content:

1. If the note is short, omit the title
2. Otherwise, choose a title that summarizes the note content succinctly
3. Avoid characters that are invalid in filenames (':', '?', etc.) in the title

## Content

If the user-provided content can stand on its own, use it **as is** and don't editorialize or summarize.
If not, wrap the user-provided content in a blockquote and then add your own content below.

You should fix obvious spelling and grammar errors.

## Tags

Valid tags:
- Must start with a '+' prefix (e.g., +project)
- Can only contain lowercase letters, numbers, and dashes
- Cannot end with a dash

If the user provides tags, use those.

Add tags yourself if you can think of relevant tags and the user does provide less than three.
Tags are either metadata (e.g. '+todo') or must reflect the content of the note (e.g. '+meeting-notes').
Prefer tags that do not appear verbatim in the content to maximize the chance of a search hit.
