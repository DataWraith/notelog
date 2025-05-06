# Notelog

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
echo "Lorem ipsum" | notelog -d ~/Notes

# Add a note with a specific notes directory
notelog -d ~/Notes add -t "Wellerman" There once was a ship
```

### Notes Directory

By default, notes are stored in `~/NoteLog`. You can specify a different directory using the `-d` or `--notes-dir` option.

The notes directory is organized as follows:
```
~/NoteLog/
├── 2025/
│   ├── 01 January/
│   │   ├── 2025-01-01 First note.md
│   │   └── 2025-01-02 Second note.md
│   ├── 02 February/
│   │   └── ...
│   └── ...
└── ...
```

### Note Format

Each note is stored as a Markdown file with YAML frontmatter:

```markdown
---
created: 2025-04-01T12:00:00+02:00
tags:
  - tag1
  - tag2
  - tag3
---

# The note title

Lorem ipsum dolor sit amet.

```

## Features

- Multiple input methods: command line arguments, file, stdin, or interactive editor
- Automatic organization of notes by year and month
- YAML frontmatter with creation timestamp
- Placeholder for future LLM-based tagging functionality
