# Notelog

**Status**: Abandoned.

Notelog is a command-line tool that you can use to record notes as you think of them -- thoughts, todos, insights, accomplishments, etc. It includes a *Model Context Protocol* (MCP) server for use by AI assistants as primary mode of interaction.

## Installation

If you're on Linux (x86-64), you can download a pre-built executable from the [Releases](https://github.com/DataWraith/notelog/releases) page.

To install from source:

1. Clone this repository
2. Build and install with `cargo install --path .`

## Usage

### Basic Usage

```bash
# Opens an editor to capture a note
notelog

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

# Opens the most recent note in the editor
notelog last

# Prints the most recent note to stdout
notelog last --print
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

Notelog can act as a server that receives commands from AI assistants, allowing you to create, (re-)tag  or search notes using natural language (see examples below).

#### JSON Configuration Example

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

##### How to set up an MCP server

* [Claude Desktop tutorial](https://modelcontextprotocol.info/docs/quickstart/user/)
* [Cursor IDE tutorial](https://docs.cursor.com/context/model-context-protocol)

#### Creating Notes

You can create notes by asking the LLM:

- `/log Added Model Context Protocol support to Notelog +mcp +done`
- `Create a note: "Use this text verbatim in the note"`
- `Summarize the conversation so far as a notelog`

The default prompt instructs the LLM to automatically add tags and a title to the notes as appropriate.

#### Searching for Notes

You can search for notes using fulltext search or by specific tags:

- `Find notes containing "project plan" with tag +important`
- `Search for notes tagged +sqlite and +til from May 2025`
- `How many notes tagged +todo do I have?`

To avoid bloating the context window too much, a maximum of 25 notes with their IDs will be returned. The LLM can then use the IDs to retrieve the note contents or edit its tags on request.

#### Editing Tags

You can edit the tags of existing notes by asking the LLM:

- `Remove the +draft tag from note abc123 and add +todo`
- `Mark note def456 as done` (this should remove the +todo tag and add the +done tag)

The note IDs can be found by searching for notes as detailed above.
