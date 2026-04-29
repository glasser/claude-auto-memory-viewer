# Claude Auto-Memory Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single-binary Rust HTTP tool that browses Claude Code's auto-memory across all projects in a single SPA.

**Architecture:** One binary that on each request scans `~/.claude/projects/*/memory/`, resolves real project paths via `~/.claude.json`, builds a collapsed path tree, server-side renders all memory as HTML (Markdown via `comrak`), and serves the result. Tiny vanilla JS for tree expand/collapse (`<details>`) and project switching (hash routing).

**Tech Stack:** Rust 2024 edition, `tiny_http`, `serde_json`, `comrak`. macOS-only convenience: `open` for browser launch.

**Spec:** `docs/superpowers/specs/2026-04-29-claude-auto-memory-viewer-design.md`.

---

## File structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | Crate metadata, dependency declarations. |
| `src/main.rs` | Entry: bind port, open browser, request loop, dispatch to render. Declares modules. |
| `src/paths.rs` | Path encoding (`/` and `.` → `-`), naive decode, `~/.claude.json` lookup. |
| `src/memory.rs` | `Project`/`MemoryFile` types, frontmatter parser, filesystem scanner. |
| `src/tree.rs` | Trie tree of projects with single-child chain collapse. |
| `src/render.rs` | Full HTML page rendering (CSS + JS embedded), `comrak` Markdown. |

---

## Task 1: Bootstrap Cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize crate**

Run: `cd /Users/glasser/Projects/Apollo/claude-auto-memory-viewer && cargo init --name claude-auto-memory-viewer --vcs none`
Expected: `Cargo.toml` and `src/main.rs` created. (`--vcs none` because the repo already has `.git`.)

- [ ] **Step 2: Verify default project builds**

Run: `cargo build`
Expected: Compiles a "Hello, world!" binary in `target/debug/`. No warnings.

- [ ] **Step 3: Add dependencies**

Run: `cargo add tiny_http serde_json comrak`
Expected: `Cargo.toml` gets a `[dependencies]` section with all three. Each pinned to current version.

- [ ] **Step 4: Verify deps build**

Run: `cargo build`
Expected: Downloads and compiles all transitive deps. Final binary still builds. Tolerate any dep warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "Bootstrap Cargo crate with tiny_http, serde_json, comrak"
```

---

## Task 2: Path encoding and lookup (`paths.rs`)

**Files:**
- Create: `src/paths.rs`
- Modify: `src/main.rs` (add `mod paths;`)

- [ ] **Step 1: Wire up the new module**

Replace the contents of `src/main.rs` with:

```rust
mod paths;

fn main() {
    println!("Hello, world!");
}
```

- [ ] **Step 2: Write failing tests**

Create `src/paths.rs` with the test module only:

```rust
use std::collections::HashMap;
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn re_encode_replaces_slash_and_dot() {
        assert_eq!(
            re_encode("/Users/glasser/Projects/Apollo/monorepo.git"),
            "-Users-glasser-Projects-Apollo-monorepo-git",
        );
    }

    #[test]
    fn re_encode_leaves_normal_chars_alone() {
        assert_eq!(re_encode("foo-bar_baz"), "foo-bar_baz");
    }

    #[test]
    fn naive_decode_replaces_dash_with_slash() {
        assert_eq!(naive_decode("-Users-glasser-foo"), "/Users/glasser/foo");
    }

    #[test]
    fn build_lookup_indexes_projects_by_encoded_form() {
        let json = json!({
            "projects": {
                "/Users/me/foo": {},
                "/Users/me/bar.git": {}
            }
        });
        let lookup = build_lookup(&json);
        assert_eq!(
            lookup.get("-Users-me-foo"),
            Some(&"/Users/me/foo".to_string())
        );
        assert_eq!(
            lookup.get("-Users-me-bar-git"),
            Some(&"/Users/me/bar.git".to_string())
        );
    }

    #[test]
    fn build_lookup_handles_missing_projects_key() {
        let json = json!({});
        assert!(build_lookup(&json).is_empty());
    }

    #[test]
    fn resolve_uses_lookup_when_present() {
        let mut lookup = HashMap::new();
        lookup.insert(
            "-Users-me-bar-git".to_string(),
            "/Users/me/bar.git".to_string(),
        );
        assert_eq!(resolve("-Users-me-bar-git", &lookup), "/Users/me/bar.git");
    }

    #[test]
    fn resolve_falls_back_to_naive_decode() {
        let lookup = HashMap::new();
        assert_eq!(resolve("-Users-me-foo", &lookup), "/Users/me/foo");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --bin claude-auto-memory-viewer paths::`
Expected: Compilation errors — `re_encode`, `naive_decode`, `build_lookup`, `resolve` not found.

- [ ] **Step 4: Implement the four functions**

Append to `src/paths.rs` (keep the existing test module):

```rust
pub fn re_encode(real: &str) -> String {
    real.chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

pub fn naive_decode(encoded: &str) -> String {
    encoded.replace('-', "/")
}

pub fn build_lookup(claude_json: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(projects) = claude_json.get("projects").and_then(|v| v.as_object()) {
        for key in projects.keys() {
            map.insert(re_encode(key), key.clone());
        }
    }
    map
}

pub fn resolve(encoded: &str, lookup: &HashMap<String, String>) -> String {
    lookup
        .get(encoded)
        .cloned()
        .unwrap_or_else(|| naive_decode(encoded))
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --bin claude-auto-memory-viewer paths::`
Expected: 7 tests pass, 0 fail.

- [ ] **Step 6: Commit**

```bash
git add src/paths.rs src/main.rs
git commit -m "Add path encoding and ~/.claude.json lookup"
```

---

## Task 3: Frontmatter parser (`memory.rs`)

**Files:**
- Create: `src/memory.rs`
- Modify: `src/main.rs` (add `mod memory;`)

- [ ] **Step 1: Wire up the new module**

Edit `src/main.rs` so `mod` declarations look like:

```rust
mod memory;
mod paths;

fn main() {
    println!("Hello, world!");
}
```

- [ ] **Step 2: Write the file scaffold + failing parser tests**

Create `src/memory.rs`:

```rust
use std::path::Path;
use std::time::SystemTime;

pub struct Project {
    pub real_path: String,
    pub encoded: String,
    pub files: Vec<MemoryFile>,
}

pub struct MemoryFile {
    pub name: String,
    pub frontmatter: Vec<(String, String)>,
    pub body: String,
    pub mtime: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_with_frontmatter() {
        let content = "---\nname: Foo\ntype: feedback\n---\nBody text\nMore.\n";
        let (fm, body) = parse_file(content);
        assert_eq!(
            fm,
            vec![
                ("name".into(), "Foo".into()),
                ("type".into(), "feedback".into()),
            ]
        );
        assert_eq!(body, "Body text\nMore.\n");
    }

    #[test]
    fn parse_file_without_frontmatter() {
        let content = "Just body text\n";
        let (fm, body) = parse_file(content);
        assert!(fm.is_empty());
        assert_eq!(body, "Just body text\n");
    }

    #[test]
    fn parse_file_malformed_no_close() {
        let content = "---\nname: Foo\nbody but no close marker\n";
        let (fm, body) = parse_file(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn parse_file_handles_value_with_colon() {
        let content = "---\nurl: https://example.com\n---\nbody\n";
        let (fm, _) = parse_file(content);
        assert_eq!(
            fm,
            vec![("url".into(), "https://example.com".into())]
        );
    }
}

#[allow(dead_code)]
pub fn scan_all(_home: &Path) -> Vec<Project> {
    Vec::new()
}
```

(`scan_all` stub keeps later tasks compilable; will be implemented in Task 4.)

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --bin claude-auto-memory-viewer memory::`
Expected: Compilation errors — `parse_file` not found.

- [ ] **Step 4: Implement `parse_file`**

Append to `src/memory.rs`:

```rust
pub fn parse_file(content: &str) -> (Vec<(String, String)>, String) {
    if !content.starts_with("---\n") {
        return (Vec::new(), content.to_string());
    }
    let after_open = &content[4..];
    let close = match after_open.find("\n---\n") {
        Some(i) => i,
        None => return (Vec::new(), content.to_string()),
    };
    let fm_text = &after_open[..close];
    let body = after_open[close + 5..].to_string();
    let mut fm = Vec::new();
    for line in fm_text.lines() {
        if let Some(idx) = line.find(':') {
            let k = line[..idx].trim().to_string();
            let v = line[idx + 1..].trim().to_string();
            fm.push((k, v));
        }
    }
    (fm, body)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --bin claude-auto-memory-viewer memory::`
Expected: 4 tests pass, 0 fail.

- [ ] **Step 6: Commit**

```bash
git add src/memory.rs src/main.rs
git commit -m "Add Project/MemoryFile types and frontmatter parser"
```

---

## Task 4: Memory scanner (`memory.rs`)

**Files:**
- Modify: `src/memory.rs` (replace `scan_all` stub with real implementation)

This task has no automated test (filesystem-heavy, the user has real data we'll smoke-test against in Task 8). We add the implementation, prove it compiles, then move on.

- [ ] **Step 1: Replace the stub with the real `scan_all`**

In `src/memory.rs`, replace the `#[allow(dead_code)] pub fn scan_all(...)` stub with:

```rust
pub fn scan_all(home: &Path) -> Vec<Project> {
    let projects_root = home.join(".claude").join("projects");
    let claude_json_path = home.join(".claude.json");

    let lookup = std::fs::read_to_string(&claude_json_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| crate::paths::build_lookup(&v))
        .unwrap_or_default();

    let mut projects = Vec::new();
    let entries = match std::fs::read_dir(&projects_root) {
        Ok(e) => e,
        Err(_) => return projects,
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let mem_dir = dir.join("memory");
        if !mem_dir.is_dir() {
            continue;
        }
        let files = scan_memory_files(&mem_dir);
        if files.is_empty() {
            continue;
        }
        let encoded = entry.file_name().to_string_lossy().into_owned();
        let real_path = crate::paths::resolve(&encoded, &lookup);
        projects.push(Project { real_path, encoded, files });
    }
    projects.sort_by(|a, b| a.real_path.cmp(&b.real_path));
    projects
}

fn scan_memory_files(mem_dir: &Path) -> Vec<MemoryFile> {
    let entries = match std::fs::read_dir(mem_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut files: Vec<MemoryFile> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() {
                return None;
            }
            if p.extension().and_then(|s| s.to_str()) != Some("md") {
                return None;
            }
            let name = p.file_name()?.to_string_lossy().into_owned();
            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("skipping {}: {}", p.display(), err);
                    return None;
                }
            };
            let mtime = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let (frontmatter, body) = parse_file(&content);
            Some(MemoryFile {
                name,
                frontmatter,
                body,
                mtime,
            })
        })
        .collect();
    files.sort_by(|a, b| {
        if a.name == "MEMORY.md" {
            return std::cmp::Ordering::Less;
        }
        if b.name == "MEMORY.md" {
            return std::cmp::Ordering::Greater;
        }
        a.name.cmp(&b.name)
    });
    files
}
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Compiles with no errors. There will be a `dead_code` warning for the new private `scan_memory_files` if `scan_all` is unused at this point — that's fine; main calls it in Task 7.

- [ ] **Step 3: Sanity-check against real data via `cargo run` instrumentation**

Add a temporary debug print in `main.rs`:

```rust
mod memory;
mod paths;

use std::path::PathBuf;

fn main() {
    let home = PathBuf::from(std::env::var_os("HOME").expect("HOME unset"));
    let projects = memory::scan_all(&home);
    println!("found {} projects", projects.len());
    for p in &projects {
        println!("  {} ({} files)", p.real_path, p.files.len());
    }
}
```

Run: `cargo run`
Expected: Lists each project with a non-zero file count. Each `real_path` should look like a real path (e.g. `/Users/glasser/Projects/Apollo/monorepo.git`, dots and slashes correct).

- [ ] **Step 4: Revert `main.rs` to the minimal stub**

Replace `src/main.rs` with:

```rust
mod memory;
mod paths;

fn main() {
    println!("Hello, world!");
}
```

(The real `main` lands in Task 7.)

- [ ] **Step 5: Commit**

```bash
git add src/memory.rs src/main.rs
git commit -m "Implement memory scanner walking ~/.claude/projects"
```

---

## Task 5: Tree builder (`tree.rs`)

**Files:**
- Create: `src/tree.rs`
- Modify: `src/main.rs` (add `mod tree;`)

- [ ] **Step 1: Wire up the new module**

Edit `src/main.rs` to:

```rust
mod memory;
mod paths;
mod tree;

fn main() {
    println!("Hello, world!");
}
```

- [ ] **Step 2: Write the failing tests**

Create `src/tree.rs`:

```rust
use crate::memory::Project;

pub struct Node {
    pub name: String,
    pub project_key: Option<String>,
    pub children: Vec<Node>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Project;

    fn proj(path: &str) -> Project {
        Project {
            real_path: path.into(),
            encoded: path.replace('/', "-").replace('.', "-"),
            files: vec![],
        }
    }

    #[test]
    fn single_project_collapses_to_one_node() {
        let projects = vec![proj("/Users/me/foo")];
        let tree = build_tree(&projects);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Users/me/foo");
        assert_eq!(tree[0].project_key.as_deref(), Some("/Users/me/foo"));
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn two_siblings_share_collapsed_parent() {
        let projects = vec![proj("/Users/me/foo"), proj("/Users/me/bar")];
        let tree = build_tree(&projects);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Users/me");
        assert!(tree[0].project_key.is_none());
        let names: Vec<&str> = tree[0].children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["bar", "foo"]);
    }

    #[test]
    fn nested_project_keeps_children() {
        let projects = vec![proj("/Users/me"), proj("/Users/me/sub")];
        let tree = build_tree(&projects);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "Users/me");
        assert_eq!(tree[0].project_key.as_deref(), Some("/Users/me"));
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].name, "sub");
        assert_eq!(
            tree[0].children[0].project_key.as_deref(),
            Some("/Users/me/sub")
        );
    }

    #[test]
    fn unrelated_paths_make_separate_roots() {
        let projects = vec![proj("/Users/me/foo"), proj("/private/tmp/x")];
        let tree = build_tree(&projects);
        assert_eq!(tree.len(), 2);
        let names: Vec<&str> = tree.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["Users/me/foo", "private/tmp/x"]);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --bin claude-auto-memory-viewer tree::`
Expected: Compilation errors — `build_tree` not found.

- [ ] **Step 4: Implement the tree builder**

Append to `src/tree.rs`:

```rust
pub fn build_tree(projects: &[Project]) -> Vec<Node> {
    let mut roots: Vec<Node> = Vec::new();
    for project in projects {
        let segments: Vec<String> = project
            .real_path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        insert(&mut roots, &segments, &project.real_path);
    }
    for root in &mut roots {
        collapse(root);
    }
    sort(&mut roots);
    roots
}

fn insert(nodes: &mut Vec<Node>, segments: &[String], project_key: &str) {
    if segments.is_empty() {
        return;
    }
    let head = &segments[0];
    let idx = match nodes.iter().position(|n| &n.name == head) {
        Some(i) => i,
        None => {
            nodes.push(Node {
                name: head.clone(),
                project_key: None,
                children: Vec::new(),
            });
            nodes.len() - 1
        }
    };
    if segments.len() == 1 {
        nodes[idx].project_key = Some(project_key.to_string());
    } else {
        insert(&mut nodes[idx].children, &segments[1..], project_key);
    }
}

fn collapse(node: &mut Node) {
    while node.project_key.is_none() && node.children.len() == 1 {
        let child = node.children.remove(0);
        node.name = format!("{}/{}", node.name, child.name);
        node.project_key = child.project_key;
        node.children = child.children;
    }
    for child in &mut node.children {
        collapse(child);
    }
}

fn sort(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| a.name.cmp(&b.name));
    for n in nodes.iter_mut() {
        sort(&mut n.children);
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --bin claude-auto-memory-viewer tree::`
Expected: 4 tests pass, 0 fail.

- [ ] **Step 6: Commit**

```bash
git add src/tree.rs src/main.rs
git commit -m "Add path tree builder with single-child chain collapse"
```

---

## Task 6: HTML renderer (`render.rs`)

**Files:**
- Create: `src/render.rs`
- Modify: `src/main.rs` (add `mod render;`)

- [ ] **Step 1: Wire up the new module**

Edit `src/main.rs` to:

```rust
mod memory;
mod paths;
mod render;
mod tree;

fn main() {
    println!("Hello, world!");
}
```

- [ ] **Step 2: Write the failing tests**

Create `src/render.rs`:

```rust
use std::fmt::Write;

use comrak::{markdown_to_html, Options};

use crate::memory::{MemoryFile, Project};
use crate::tree::Node;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_handles_metachars() {
        assert_eq!(html_escape("<a&b>\"'"), "&lt;a&amp;b&gt;&quot;&#39;");
    }

    #[test]
    fn render_markdown_emits_html() {
        let html = render_markdown("# hello");
        assert!(html.contains("<h1>"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn render_markdown_supports_gfm_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = render_markdown(md);
        assert!(html.contains("<table>"));
    }

    #[test]
    fn render_markdown_strips_raw_html() {
        let html = render_markdown("<script>alert(1)</script>\n\nhello");
        assert!(!html.contains("<script>"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn render_page_includes_tree_and_articles() {
        let project = Project {
            real_path: "/Users/me/foo".into(),
            encoded: "-Users-me-foo".into(),
            files: vec![MemoryFile {
                name: "MEMORY.md".into(),
                frontmatter: vec![("type".into(), "feedback".into())],
                body: "# hi".into(),
                mtime: std::time::SystemTime::UNIX_EPOCH,
            }],
        };
        let projects = vec![project];
        let tree = crate::tree::build_tree(&projects);
        let html = render_page(&tree, &projects);
        assert!(html.contains("data-key=\"/Users/me/foo\""));
        assert!(html.contains("MEMORY.md"));
        assert!(html.contains("<dt>type</dt>"));
        assert!(html.contains("<h1>hi</h1>"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --bin claude-auto-memory-viewer render::`
Expected: Compilation errors — `html_escape`, `render_markdown`, `render_page` not found.

- [ ] **Step 4: Implement the helpers and renderer**

Append to `src/render.rs`:

```rust
pub fn render_page(tree: &[Node], projects: &[Project]) -> String {
    let mut out = String::new();
    out.push_str("<!doctype html>\n");
    out.push_str("<html lang=\"en\"><head><meta charset=\"utf-8\">");
    out.push_str("<title>Claude Auto-Memory Viewer</title>");
    out.push_str("<style>");
    out.push_str(CSS);
    out.push_str("</style></head><body>");

    out.push_str("<aside id=\"tree\">");
    if tree.is_empty() {
        out.push_str("<p class=\"empty-tree\">No auto-memory found.</p>");
    } else {
        for node in tree {
            render_node(node, &mut out);
        }
    }
    out.push_str("</aside>");

    out.push_str("<main id=\"content\">");
    out.push_str("<div id=\"empty\">Select a project on the left.</div>");
    for proj in projects {
        render_project(proj, &mut out);
    }
    out.push_str("</main>");

    out.push_str("<script>");
    out.push_str(JS);
    out.push_str("</script>");
    out.push_str("</body></html>");
    out
}

fn render_node(node: &Node, out: &mut String) {
    if node.children.is_empty() {
        if let Some(key) = &node.project_key {
            write!(
                out,
                "<button class=\"proj\" data-key=\"{}\">{}</button>",
                html_escape(key),
                html_escape(&node.name),
            )
            .unwrap();
        }
        return;
    }
    out.push_str("<details open><summary>");
    out.push_str(&html_escape(&node.name));
    out.push_str("</summary>");
    if let Some(key) = &node.project_key {
        write!(
            out,
            "<button class=\"proj self\" data-key=\"{}\">(this folder)</button>",
            html_escape(key),
        )
        .unwrap();
    }
    for child in &node.children {
        render_node(child, out);
    }
    out.push_str("</details>");
}

fn render_project(proj: &Project, out: &mut String) {
    write!(
        out,
        "<article class=\"proj-view\" data-key=\"{}\" hidden>",
        html_escape(&proj.real_path),
    )
    .unwrap();
    write!(out, "<h1>{}</h1>", html_escape(&proj.real_path)).unwrap();
    for file in &proj.files {
        render_file(file, out);
    }
    out.push_str("</article>");
}

fn render_file(file: &MemoryFile, out: &mut String) {
    out.push_str("<section class=\"file\">");
    write!(out, "<h2>{}</h2>", html_escape(&file.name)).unwrap();
    if !file.frontmatter.is_empty() {
        out.push_str("<dl class=\"frontmatter\">");
        for (k, v) in &file.frontmatter {
            write!(
                out,
                "<dt>{}</dt><dd>{}</dd>",
                html_escape(k),
                html_escape(v),
            )
            .unwrap();
        }
        out.push_str("</dl>");
    }
    out.push_str("<div class=\"body\">");
    out.push_str(&render_markdown(&file.body));
    out.push_str("</div>");
    out.push_str("</section>");
}

pub fn render_markdown(md: &str) -> String {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    opts.extension.tagfilter = true;
    // opts.render.unsafe_ defaults to false — raw HTML stays escaped.
    markdown_to_html(md, &opts)
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

const CSS: &str = r#"
* { box-sizing: border-box; }
body { margin: 0; height: 100vh; display: flex; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #1d1d1f; }
aside#tree { width: 340px; flex: 0 0 340px; height: 100vh; overflow-y: auto; padding: 0.75rem; background: #f5f5f7; border-right: 1px solid #d2d2d7; font-size: 0.9rem; }
main#content { flex: 1; height: 100vh; overflow-y: auto; padding: 2rem 2.5rem; }
.proj-view, #empty { max-width: 90ch; }
details { margin-left: 0.25rem; }
summary { cursor: pointer; padding: 2px 4px; border-radius: 4px; font-family: ui-monospace, "SF Mono", Menlo, monospace; }
summary:hover { background: #e8e8ed; }
details > details, details > button.proj { margin-left: 0.9rem; }
button.proj { display: block; width: 100%; text-align: left; padding: 3px 6px; margin: 1px 0; border: 0; background: transparent; border-radius: 4px; cursor: pointer; font: inherit; font-family: ui-monospace, "SF Mono", Menlo, monospace; color: #1d1d1f; }
button.proj:hover { background: #e8e8ed; }
button.proj.selected { background: #cce4ff; }
button.proj.self { color: #6e6e73; font-style: italic; }
#empty { color: #6e6e73; font-style: italic; }
.proj-view h1 { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 1rem; color: #555; margin: 0 0 1.5rem 0; word-break: break-all; }
section.file { padding-top: 1rem; margin-top: 1rem; border-top: 1px solid #e5e5ea; }
section.file:first-of-type { border-top: 0; padding-top: 0; margin-top: 0; }
section.file h2 { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 1rem; margin: 0 0 0.75rem 0; color: #1d1d1f; }
dl.frontmatter { background: #fff8e1; border-left: 3px solid #fbc02d; padding: 0.5rem 0.75rem; margin: 0 0 1rem 0; font-size: 0.85rem; display: grid; grid-template-columns: max-content 1fr; column-gap: 0.75rem; row-gap: 0.25rem; }
dl.frontmatter dt { font-family: ui-monospace, "SF Mono", Menlo, monospace; color: #6e6e73; }
dl.frontmatter dd { margin: 0; }
.body code, .body pre { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 0.9em; }
.body pre { background: #f5f5f7; padding: 0.75rem; border-radius: 6px; overflow-x: auto; }
.body p code { background: #f5f5f7; padding: 0.1em 0.3em; border-radius: 3px; }
.body h1, .body h2, .body h3 { line-height: 1.3; }
.body blockquote { margin: 0; padding-left: 1rem; border-left: 3px solid #d2d2d7; color: #6e6e73; }
.empty-tree { color: #6e6e73; font-style: italic; }
"#;

const JS: &str = r#"
function showProject(key) {
  document.querySelectorAll('.proj-view').forEach(el => {
    el.hidden = !key || el.dataset.key !== key;
  });
  const empty = document.getElementById('empty');
  empty.hidden = !!key;
  document.querySelectorAll('button.proj').forEach(b => {
    b.classList.toggle('selected', !!key && b.dataset.key === key);
  });
}
function selectFromHash() {
  const h = decodeURIComponent(location.hash.slice(1));
  showProject(h || null);
}
document.addEventListener('click', e => {
  const b = e.target.closest('button.proj');
  if (!b) return;
  location.hash = encodeURIComponent(b.dataset.key);
});
window.addEventListener('hashchange', selectFromHash);
document.addEventListener('DOMContentLoaded', selectFromHash);
"#;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --bin claude-auto-memory-viewer render::`
Expected: 5 tests pass, 0 fail.

- [ ] **Step 6: Commit**

```bash
git add src/render.rs src/main.rs
git commit -m "Render full HTML page with tree, project content, and Markdown"
```

---

## Task 7: HTTP server + browser open (`main.rs`)

**Files:**
- Modify: `src/main.rs` (full implementation)

- [ ] **Step 1: Replace `main.rs` with the real entry**

Replace `src/main.rs` with:

```rust
mod memory;
mod paths;
mod render;
mod tree;

use std::path::PathBuf;
use std::process::Command;

use tiny_http::{Header, Response, Server};

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn bind_in_range(start: u16, end: u16) -> Result<(Server, u16), String> {
    let mut last_err = String::new();
    for port in start..=end {
        match Server::http(format!("127.0.0.1:{port}")) {
            Ok(s) => return Ok((s, port)),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(format!(
        "no free port in {start}..={end}: last error: {last_err}"
    ))
}

fn main() {
    let home = match home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine $HOME.");
            std::process::exit(1);
        }
    };

    let (server, port) = match bind_in_range(4321, 4400) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to bind: {e}");
            std::process::exit(1);
        }
    };

    let url = format!("http://127.0.0.1:{port}");
    println!("Auto-memory viewer at {url}");
    if let Err(e) = Command::new("open").arg(&url).status() {
        eprintln!("(could not auto-open browser: {e}; visit the URL above)");
    }

    for request in server.incoming_requests() {
        if request.url() != "/" {
            let _ = request.respond(Response::from_string("not found").with_status_code(404));
            continue;
        }
        let projects = memory::scan_all(&home);
        let tree = tree::build_tree(&projects);
        let html = render::render_page(&tree, &projects);
        let header = Header::from_bytes(
            &b"Content-Type"[..],
            &b"text/html; charset=utf-8"[..],
        )
        .unwrap();
        let resp = Response::from_string(html).with_header(header);
        let _ = request.respond(resp);
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles cleanly. The previous "dead code" warnings around `scan_all`/`scan_memory_files`/etc. should now be gone since `main` uses them.

- [ ] **Step 3: Run the full test suite**

Run: `cargo test`
Expected: All previously-added tests still pass; nothing new fails.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Wire HTTP server, port discovery, and browser auto-open"
```

---

## Task 8: Manual smoke test against real data

**Files:** none (this is a manual verification step).

- [ ] **Step 1: Run the server**

Run: `cargo run`
Expected: Prints `Auto-memory viewer at http://127.0.0.1:4321` (or the next free port). A browser tab opens automatically.

- [ ] **Step 2: Verify the tree**

In the browser, confirm:

- The sidebar shows a folder tree, e.g. `Users/glasser/Projects/Apollo` collapsed via single-child chain compression with project leaves beneath it.
- All projects from `find ~/.claude/projects -maxdepth 3 -name memory -type d` (run in a separate shell to compare) appear in the tree.
- Real project paths are shown correctly (e.g. `monorepo.git`, not `monorepo-git`).

- [ ] **Step 3: Verify project rendering**

Click a project (e.g. `monorepo.git`):

- The right pane shows the project's real path as a heading.
- `MEMORY.md` appears first.
- Each memory file shows: filename heading, yellow-bordered frontmatter callout with `name` / `description` / `type`, then rendered Markdown body.
- Markdown formatting (bold, code, lists) renders.

- [ ] **Step 4: Verify hash routing**

- Reload the page after selecting a project: the same project should still be visible.
- Copy the URL with hash, paste into a new tab: same project loads.
- Click another project: URL hash updates, view swaps without reload.

- [ ] **Step 5: Stop the server**

Hit Ctrl-C in the terminal.

- [ ] **Step 6: If anything is wrong, fix and re-test**

Common issues to watch for:

- A project with `.` in the path (e.g. `monorepo.git`) shows wrong: check `paths::resolve` is hitting the lookup, not falling back to naive_decode. Print `lookup` for diagnosis.
- Markdown renders raw HTML (e.g. `<` characters appear escaped): expected — `tagfilter` + `unsafe_=false` is doing its job.
- Frontmatter with a value containing `:` is truncated: confirm `parse_file_handles_value_with_colon` test still passes.
- A memory file has no frontmatter and the whole content shows in the body: expected.

If a fix is needed, write a regression test before fixing, then commit.

---

## Task 9: Install and verify end-to-end

**Files:** none.

- [ ] **Step 1: Install the binary**

Run: `cargo install --path .`
Expected: Compiles in release mode and installs to `~/.cargo/bin/claude-auto-memory-viewer`.

- [ ] **Step 2: Verify it runs from anywhere**

Run: `cd /tmp && claude-auto-memory-viewer`
Expected: Same behavior as `cargo run` — prints URL, opens browser, serves page.

- [ ] **Step 3: Stop the server**

Ctrl-C.

- [ ] **Step 4: Document the install command in the commit log**

```bash
git log --oneline | head -10
```

Expected: Clean linear history with one commit per task. No further changes needed.

---

## Self-review notes

- **Spec coverage**: every section of the spec maps to a task — paths (Task 2), memory model + parser (Tasks 3–4), tree (Task 5), renderer + CSS/JS (Task 6), server + browser open + port range (Task 7), error fallbacks (covered inline in scanner + lookup), testing matrix (covered per-module + manual), file layout (matches Task file column).
- **No placeholders**: all code is concrete and complete; tests have actual asserts.
- **Type consistency**: `Project { real_path, encoded, files }`, `MemoryFile { name, frontmatter, body, mtime }`, `Node { name, project_key, children }`, `build_tree(&[Project]) -> Vec<Node>` are used identically across all tasks.
