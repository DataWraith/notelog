# Notelog

**Status**: This is a prototype of a personal tool. Use at your own risk.


Notelog is a command-line tool that you can use to record notes as you think of them -- thoughts, todos, insights, accomplishments, etc. It includes a *Model Context Protocol* (MCP) server for use by AI assistants.

Notes are stored in a local directory as Markdown files with YAML frontmatter, organized by year and month.

## Installation

No pre-built binaries are provided at this time. To install:

1. Clone this repository
2. Build and install with `cargo install --path .`

## Usage

### Basic Usage

```bash
# Add a note with content from command line arguments
notelog add "This is a note" +example-tag

# Add a note with a specific title
notelog add --title "This is a note" "This is the content of the note"

# Add a note from a file
notelog add --title "This is a note" --file /path/to/file

# Add a note with a title and content
notelog foo bar baz --title 'Metasyntactic variables'

# Add a note from stdin
echo "Lorem ipsum" | notelog

# Add a note with a specific notes directory
notelog -d ~/Shanties add -t "Wellerman" There once was a ship

# Opens an editor to capture a note
notelog
```

### Notes Directory

> [!IMPORTANT]
> You will need to create the notes directory yourself before using Notelog.

By default, notes are stored in `~/NoteLog`. You can specify a different directory using the `-d` or `--notes-dir` option, or by setting the `NOTELOG_DIR` environment variable.

The notes directory is organized as follows:

```
~/NoteLog/
├── 2025/
│   ├── 01_January/
│   │   ├── 2025-01-01T17-45 First note.md
│   │   └── 2025-01-02T12-34 Second note.md
│   ├── 02_February/
│   │   └── ...
│   └── ...
├── .notes.db
└── ...
```

Notelog maintains an SQLite database in the specified notes directory for use as a search index. Notes are monitored for changes and synchronized with the database automatically.

### Model Context Protocol Server

Notelog can act as a server that receives commands from AI assistants, allowing you to create or search notes using natural language (see examples below).

#### JSON Configuration

```json
{
  "mcpServers": {
    "notelog": {
      "command": "notelog",
      "args": [
        "mcp",
        "-d",
        "/path/to/your/NoteLog/directory"
      ]
    }
  }
}
```

##### How to use the JSON Configuration

* [Claude Desktop tutorial](https://modelcontextprotocol.info/docs/quickstart/user/)
* [Cursor IDE tutorial](https://docs.cursor.com/context/model-context-protocol)

#### Creating Notes

You can create notes by just asking the LLM:

- `/log Added Model Context Protocol support to Notelog +mcp +done`
- `Create a note: "Use this text verbatim in the note"`
- `Summarize the conversation so far as a notelog`
- `Create a note about Topic X. Show me a preview before saving.`

The default prompt instructs the LLM to automatically add tags and a title to the notes if you don't specify them.

#### Searching for Notes

You can search for notes using fulltext search or by specific tags:

- `Find notes containing "project plan" with tag +important`
- `Look for notes mentioning databases from April 2025`
- `Search for notes tagged +sqlite and +til from May 2025`
- `How many notes tagged +todo do I have?`

The search supports:
- Content search (words or phrases in the note)
- Tag search (using the `+tag` syntax)
- Date filtering (e.g., "from May 2025")
- Combinations of the above

To avoid bloating the context window too much, a maximum of 25 notes with their IDs will be returned. The LLM can then use the IDs to retrieve the note contents on request.
