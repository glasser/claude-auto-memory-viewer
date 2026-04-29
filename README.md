# claude-auto-memory-viewer

A small Rust tool that pops up a browser window for browsing Claude Code's auto-memory across all your projects.

Claude's auto-memory lives in `~/.claude/projects/<encoded-path>/memory/*.md`, with the project path encoded into the directory name. This tool scans those directories, resolves the original project paths via `~/.claude.json`, and serves a single page with a project tree on the left and rendered Markdown on the right.

## Install

```sh
cargo install --path .
```

Drops a binary at `~/.cargo/bin/claude-auto-memory-viewer`.

## Run

```sh
claude-auto-memory-viewer
```

Picks an ephemeral port, prints the URL, and opens your browser. `Ctrl-C` to stop.

The server rescans the filesystem on every request, so you can keep the tab open across Claude Code sessions and just refresh to see new memories.

## What you'll see

- **Left pane**: a folder tree of every project that has at least one memory file. Single-child path chains are collapsed (e.g. `/Users/you/Projects/Foo` shows as one segment with the projects underneath).
- **Right pane**: when you click a project, every memory file for it stacks vertically. `MEMORY.md` first as the index, then each individual memory with its YAML frontmatter (`name` / `description` / `type`) shown as a callout, followed by the rendered Markdown body.
- **Hash routing**: the URL's `#` fragment encodes the current project (and optionally a file section), so refreshing or sharing the URL keeps your place.
- **Intra-project links**: links inside `MEMORY.md` like `[foo](feedback_xxx.md)` are rewritten to in-page anchors that scroll to the matching file's section.

## Stack

- `tiny_http` — single-threaded HTTP server, no async runtime.
- `serde_json` — parses `~/.claude.json` for the project-path lookup.
- `comrak` — CommonMark + GFM Markdown rendering, server-side.
- Vanilla JS in the page for tree expand/collapse (`<details>`) and project switching (~25 lines).

## Limitations

- macOS-only auto-open (`open <url>`); on other platforms it'll print the URL and you visit it manually.
- Read-only — no editing or deletion of memories.
- Only bare-filename intra-project links are rewritten (e.g. `[foo](bar.md)`); variants like `./bar.md` or `bar.md#section` slip through.
- The project-path encoding (`/` and `.` → `-`) is ambiguous; we use `~/.claude.json` as the source of truth, falling back to a naive decode if the project is no longer in the config.
