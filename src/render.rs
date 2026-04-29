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
