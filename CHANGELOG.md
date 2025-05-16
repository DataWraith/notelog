# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- scriv-insert-here -->

<a id='changelog-0.5.0'></a>
# 0.5.0 — 2025-05-16

## Changed

- Notes now have an explicit ID (a long alphanumeric string) that can be used to stably identify a note for fetching.

## Fixed

- Notes returned via the search tool are now ordered by relevance instead of by ID

<a id='changelog-0.4.0'></a>
# 0.4.0 — 2025-05-14

## Changed

- Searching for notes now uses a fulltext index (SQLite's FTS5 extension) instead of the tagging system.

<a id='changelog-0.3.0'></a>
# 0.3.0 — 2025-05-13

## Added

- Monitoring of the NoteLog directory to detect new/modified/deleted files

## Changed

- The `search_by_tags` MCP tool can now return the number of search results independently from the search results themselves
- The `search_by_tags` MCP tool can now ask for between 1 and 25 search results to be returned

<a id='changelog-0.2.0'></a>
# 0.2.0 — 2025-05-12

## Added

- New MCP server tool: `search_by_tags`

- `fetch_note` tool that allows the LLM to retrieve a note by its ID. IDs are returned by the `search_by_tags`-tool.

<a id='changelog-0.1.2'></a>
# 0.1.2 — 2025-05-11

## Changed

- Trailing periods are now stripped from titles to prevent filenames like "Title..md"

- The MCP server no longer returns the full path to the saved note to avoid leaking more information than is necessary to the LLM.

<a id='changelog-0.1.1'></a>
# 0.1.1 — 2025-05-09

## Fixed

- Whitespace at the end of a note is now trimmed when writing it to the filesystem.

- Notelog now strips '-' or '*' from the front of the title if present.

  This allows you to make a note that consists of a Markdown list and not end up with a filename that contains the leading dash/asterisk.

- Improved detection of empty notes when adding notes via $EDITOR.

- When opening a note in the editor, no longer ignores tags supplied on the command-line.

<a id='changelog-0.1.0'></a>
# 0.1.0 — 2025-05-08

## Added

- A CLI that can capture notes from the command-line, STDIN or by opening an editor.
- A Model Context Protocol server that can be used by LLMs to capture notes on your behalf
