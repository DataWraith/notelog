# Notelog

[![No Maintenance Intended](http://unmaintained.tech/badge.svg)](http://unmaintained.tech/)

**Status**: This is an early prototype of a personal tool. Use at your own risk.


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
notelog add "This is a note"

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

NOTE: You will need to create the notes directory yourself before using Notelog.

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

Notelog keeps an SQLite database in the notes directory that serves as a search index.

The database is updated when adding notes through the MCP server, but not when you add or edit notes manually or through the CLI, so you may need to restart the MCP server to see added or changed notes.

### Model Context Protocol Server

When running in MCP mode, Notelog acts as a server that can receive commands from client software that supports the protocol. This allows AI assistants to interact with Notelog directly.

You can create notes by just asking the LLM:

- `/log Added Model Context Protocol support to Notelog +mcp +done`
- `Create a note: "Use this text verbatim in the note"`
- `Summarize the conversation so far as a notelog`
- `Create a note about Topic X. Show me a preview before saving.`

The default prompt instructs the LLM to automatically add tags and a title to the notes if you don't specify them.

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

#### Available Tools

| Tool Name | Description |
|-----------|-------------|
| add_note  | Add a new note to your NoteLog directory |
| search_by_tags | Search for notes that match specific tags |
| fetch_note | Retrieve a specific note by its ID |

You can search for notes by asking the LLM:

- `Find notes with tag +project`
- `Search for notes about programming from last month`
- `Show me notes with tags +important and +urgent`
