# Notelog

[![No Maintenance Intended](http://unmaintained.tech/badge.svg)](http://unmaintained.tech/)

**Status**: Early prototype. Use at your own risk.

Notelog is a command-line tool that records notes as you think of them -- shower thoughts, todos, insights, etc.

Notes are stored in a local directory as Markdown files with YAML frontmatter, organized by year and month.

## Installation

```bash
cargo install --path .
```

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
notelog -d ~/Notes add -t "Wellerman" There once was a ship
```

### MCP Server

The MCP (Model Context Protocol) server allows AI assistants to interact with Notelog directly. When running in MCP mode, Notelog acts as a server that can receive commands from AI models that support the protocol.

#### Example JSON Configuration

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

### Notes Directory

By default, notes are stored in `~/NoteLog`. You can specify a different directory using the `-d` or `--notes-dir` option or by setting the `NOTELOG_DIR` environment variable. You will need to create the directory yourself before using Notelog.

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
└── ...
```
