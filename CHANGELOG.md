# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- scriv-insert-here -->

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
