# Claude Auto-Memory Viewer — Design

A small Rust web tool that browses Claude Code's auto-memory across all projects in a single SPA. Reads `~/.claude/projects/<encoded>/memory/`, resolves the real project path from `~/.claude.json`, and serves an HTML page with a project tree on the left and rendered memory on the right.

## Goals

- Single-binary install via `cargo install --path .`. No runtime dependencies.
- Always shows current state: rescan filesystem on each request.
- Auto-opens browser when launched.
- Cleanly rendered Markdown for Claude's memory files (CommonMark + GFM).

## Non-goals (YAGNI)

- Search.
- Edit / delete memory.
- Live filesystem watch / auto-refresh.
- Syntax highlighting in code blocks.
- Authentication, multi-user.

## Architecture

Single-binary HTTP server bound to `127.0.0.1:<port>`. One handler. On each `GET /`:

1. Read `~/.claude.json`, extract the keys of its top-level `projects` map (these are the real project paths).
2. Build a lookup: re-encode each real path (every `/` and `.` → `-`) and store `encoded → real_path`.
3. Walk `~/.claude/projects/`. For each subdir, check `memory/` exists and contains at least one `.md` file. Skip otherwise.
4. For each surviving project: read all `.md` files; resolve the real path via the lookup; fall back to naive decode (`-` → `/`) only if there is no match (e.g. project removed from config).
5. Build a tree by splitting each real path on `/` and inserting into a trie. Collapse single-child internal chains so e.g. `Users/glasser/Projects/Apollo` shows as one segment with the projects nested beneath.
6. Render the full HTML page server-side, including every project's memory rendered to HTML via `comrak`.
7. Return.

Anything other than `GET /` returns 404. No assets, no other endpoints.

## Path encoding

Confirmed by inspection of `~/.claude.json`: Claude Code encodes a project path by replacing every `/` and `.` with `-`. So `/Users/glasser/Projects/Apollo/monorepo.git` becomes `-Users-glasser-Projects-Apollo-monorepo-git`. Decoding is ambiguous (a `-` could be `/`, `.`, or a literal `-`), which is why we use `~/.claude.json` as the source of truth.

## Components

### Memory scanner (`memory.rs`)

```rust
pub struct Project {
    pub real_path: String,
    pub encoded: String,
    pub files: Vec<MemoryFile>,
}

pub struct MemoryFile {
    pub name: String,
    pub frontmatter: Vec<(String, String)>, // ordered
    pub body: String,                        // raw markdown after frontmatter
    pub mtime: SystemTime,
}

pub fn scan_all(home: &Path) -> Vec<Project>;
```

Files within a project are sorted: `MEMORY.md` first, then alphabetical.

### Path lookup (`paths.rs`)

```rust
pub fn re_encode(real: &str) -> String;            // every '/' and '.' -> '-'
pub fn naive_decode(encoded: &str) -> String;      // every '-' -> '/'; encoded already starts with '-' so result starts with '/'
pub fn build_lookup(claude_json: &Value) -> HashMap<String, String>;  // encoded -> real
pub fn resolve(encoded: &str, lookup: &HashMap<String, String>) -> String;
```

### Frontmatter parser (in `memory.rs`)

Strip `^---\n…\n---\n` from the start of file content. Split each line on the first `:`, trim both sides. Preserve insertion order. The body is everything after the closing `---\n`. Files without frontmatter render their entire content as the body.

### Tree builder (`tree.rs`)

```rust
pub struct Node {
    pub name: String,                   // single segment, or collapsed segments joined by '/'
    pub project: Option<Project>,       // Some if this node corresponds to a project
    pub children: Vec<Node>,
}

pub fn build_tree(projects: Vec<Project>) -> Vec<Node>;
```

A `Node` can carry both a project and children (handles the unlikely case where one project's path is the parent of another's).

Algorithm: trie insert by path segment, then walk and collapse — a node with exactly one child, no project of its own, joins its `name` with the child's via `/` and adopts the child's children/project.

### Renderer (`render.rs`)

`pub fn render_page(tree: &[Node], projects: &[Project]) -> String` returns the full HTML document.

Markdown rendered via `comrak::markdown_to_html` with options:

- `extension.table = true`
- `extension.strikethrough = true`
- `extension.autolink = true`
- `extension.tasklist = true`
- `extension.footnotes = true`
- `extension.tagfilter = true` (sanitize raw `<script>` etc.)
- `render.unsafe_ = false` — do not pass through raw HTML.

### Main (`main.rs`)

- Try to bind `127.0.0.1` starting at port 4321; on `AddrInUse`, increment up to 4400.
- Print `Auto-memory viewer at http://127.0.0.1:<port>`.
- Spawn `open <url>` (macOS) via `Command`. If it fails, log and continue.
- Handle requests in the calling thread; this is a single-user local tool.

## Page structure

```html
<!doctype html>
<html>
  <head>… CSS …</head>
  <body>
    <aside id="tree">
      <!-- nested <details><summary> for folders, <button class="proj" data-key="..."> for leaves -->
    </aside>
    <main id="content">
      <div id="empty">Select a project on the left.</div>
      <article class="proj-view" data-key="/Users/glasser/Projects/Apollo/monorepo.git" hidden>
        <h1>…real path…</h1>
        <section class="file">
          <h2>MEMORY.md</h2>
          <dl class="frontmatter">…</dl>
          <div class="body">…rendered markdown…</div>
        </section>
        … one section per file …
      </article>
      … one article per project …
    </main>
    <script>
      // ~20 lines: hash-based routing.
      // showProject(key): hide #empty + all .proj-view, show the one whose data-key === key.
      // On DOMContentLoaded and 'hashchange': read decodeURIComponent(location.hash.slice(1)) and call showProject.
      // On click of .proj button: location.hash = encodeURIComponent(button.dataset.key).
      // Mark currently-selected button with class for highlight.
    </script>
  </body>
</html>
```

The hash is the real project path, so refreshing keeps your place.

## Styling

- Flexbox: `aside` fixed 340px, `main` flex-1, both 100vh, both independently scrollable.
- Sidebar: light background, slightly smaller text. `<summary>` styled as folder; `.proj` button as leaf with selected highlight.
- Right pane: padded, comfortable line length (~80ch).
- Frontmatter `<dl>`: yellow-left-border callout, monospace keys.
- `code` / `pre`: muted background, monospace.
- File sections: thin top border; filename heading in monospace.
- System font stack for prose; `SF Mono` / `Menlo` for code.

All CSS lives in a `<style>` block in the rendered page; no external assets.

## Error handling

| Situation | Behavior |
|---|---|
| `~/.claude/projects/` missing | Render the page with an empty tree and an explanatory message. |
| `~/.claude.json` missing or unparseable | Continue with empty lookup; all paths fall back to naive decode. |
| Memory file unreadable | Log to stderr, skip that file. |
| Malformed frontmatter | Treat the whole file as body. |
| Port range exhausted | Print error and exit 1. |
| `open` fails | Print URL and a hint, keep serving. |

## Testing

- Unit: `re_encode("/Users/foo/bar.git") == "-Users-foo-bar-git"`.
- Unit: `parse_frontmatter` on (present, absent, malformed) inputs.
- Unit: tree builder collapses single-child chains; preserves multi-child branching.
- Unit: `resolve` returns the real path when present, naive decode when absent.
- Manual: run against the real `~/.claude/projects` and verify each project appears with the correct real path, MEMORY.md sorted first, and Markdown renders.

## File layout

```
Cargo.toml
src/
  main.rs       # entry: server, request loop, browser open
  memory.rs     # Project, MemoryFile, scan_all, frontmatter parsing
  paths.rs      # encoding, lookup
  tree.rs       # tree build + collapse
  render.rs     # full HTML rendering, comrak invocation
docs/superpowers/specs/2026-04-29-claude-auto-memory-viewer-design.md
```

## Crates

```toml
[dependencies]
tiny_http = "0.12"
serde_json = "1"
comrak = "0.28"
```

Versions nominal; `cargo add` will pick current.

## Run

```
cargo install --path .
claude-auto-memory-viewer
```

Prints URL, opens browser, serves until Ctrl-C.
