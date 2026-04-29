use std::collections::HashSet;
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
    fn rewrite_intra_links_replaces_known_filenames() {
        let mut files = HashSet::new();
        files.insert("foo.md");
        files.insert("bar.md");
        let html = "<a href=\"foo.md\">hi</a> <a href=\"http://x.com\">ext</a> <a href=\"bar.md\">b</a>";
        let result = rewrite_intra_links(html, "-encoded", &files);
        assert!(result.contains("href=\"#-encoded__foo.md\""));
        assert!(result.contains("href=\"#-encoded__bar.md\""));
        assert!(result.contains("href=\"http://x.com\""));
    }

    #[test]
    fn rewrite_intra_links_ignores_unknown_filenames() {
        let files = HashSet::new();
        let html = "<a href=\"unknown.md\">link</a>";
        let result = rewrite_intra_links(html, "-encoded", &files);
        assert_eq!(result, html);
    }

    #[test]
    fn render_page_adds_section_ids_and_rewrites_links() {
        let project = Project {
            real_path: "/Users/me/foo".into(),
            encoded: "-Users-me-foo".into(),
            files: vec![
                MemoryFile {
                    name: "MEMORY.md".into(),
                    frontmatter: vec![],
                    body: "[link](other.md)".into(),
                    mtime: std::time::SystemTime::UNIX_EPOCH,
                    is_orphan: false,
                },
                MemoryFile {
                    name: "other.md".into(),
                    frontmatter: vec![],
                    body: "body".into(),
                    mtime: std::time::SystemTime::UNIX_EPOCH,
                    is_orphan: false,
                },
            ],
        };
        let projects = vec![project];
        let tree = crate::tree::build_tree(&projects);
        let html = render_page(&tree, &projects);
        assert!(html.contains("id=\"-Users-me-foo__MEMORY.md\""));
        assert!(html.contains("id=\"-Users-me-foo__other.md\""));
        assert!(html.contains("href=\"#-Users-me-foo__other.md\""));
    }

    #[test]
    fn render_page_marks_orphan_sections() {
        let project = Project {
            real_path: "/Users/me/foo".into(),
            encoded: "-Users-me-foo".into(),
            files: vec![MemoryFile {
                name: "stray.md".into(),
                frontmatter: vec![],
                body: "body".into(),
                mtime: std::time::SystemTime::UNIX_EPOCH,
                is_orphan: true,
            }],
        };
        let html = render_page(&[], &[project]);
        assert!(html.contains("class=\"file orphan\""));
        assert!(html.contains("(not linked from MEMORY.md)"));
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
                is_orphan: false,
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
    let filenames: HashSet<&str> = proj.files.iter().map(|f| f.name.as_str()).collect();
    write!(
        out,
        "<article class=\"proj-view\" data-key=\"{}\" hidden>",
        html_escape(&proj.real_path),
    )
    .unwrap();
    write!(out, "<h1>{}</h1>", html_escape(&proj.real_path)).unwrap();
    for file in &proj.files {
        render_file(file, &proj.encoded, &filenames, out);
    }
    out.push_str("</article>");
}

fn render_file(file: &MemoryFile, encoded: &str, project_files: &HashSet<&str>, out: &mut String) {
    let class = if file.is_orphan { "file orphan" } else { "file" };
    write!(
        out,
        "<section class=\"{class}\" id=\"{}__{}\">",
        html_escape(encoded),
        html_escape(&file.name),
    )
    .unwrap();
    write!(out, "<h2>{}", html_escape(&file.name)).unwrap();
    if file.is_orphan {
        out.push_str(" <span class=\"orphan-tag\">(not linked from MEMORY.md)</span>");
    }
    out.push_str("</h2>");
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
    let body_html = render_markdown(&file.body);
    out.push_str(&rewrite_intra_links(&body_html, encoded, project_files));
    out.push_str("</div>");
    out.push_str("</section>");
}

fn rewrite_intra_links(html: &str, encoded: &str, project_files: &HashSet<&str>) -> String {
    let mut result = html.to_string();
    for filename in project_files {
        let old = format!("href=\"{filename}\"");
        let new = format!("href=\"#{encoded}__{filename}\"");
        result = result.replace(&old, &new);
    }
    result
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
button.proj::before { content: "\2022"; display: inline-block; width: 1em; color: #8e8e93; text-align: center; }
button.proj:hover { background: #e8e8ed; }
button.proj.selected { background: #cce4ff; }
button.proj.self { color: #6e6e73; font-style: italic; }
#empty { color: #6e6e73; font-style: italic; }
.proj-view h1 { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 1rem; color: #555; margin: 0 0 1.5rem 0; word-break: break-all; }
section.file { padding-top: 1rem; margin-top: 1rem; border-top: 1px solid #e5e5ea; }
section.file:first-of-type { border-top: 0; padding-top: 0; margin-top: 0; }
section.file.orphan { background: #fffaeb; border-radius: 4px; padding: 0.75rem 0.75rem 0.5rem 0.75rem; }
section.file.orphan + section.file { border-top: 1px solid #e5e5ea; }
section.file h2 { font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 1rem; margin: 0 0 0.75rem 0; color: #1d1d1f; }
.orphan-tag { font-family: -apple-system, BlinkMacSystemFont, sans-serif; font-size: 0.75rem; color: #b45309; font-weight: normal; margin-left: 0.5em; }
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
  document.getElementById('empty').hidden = !!key;
  document.querySelectorAll('button.proj').forEach(b => {
    b.classList.toggle('selected', !!key && b.dataset.key === key);
  });
}
function applyHash() {
  const h = decodeURIComponent(location.hash.slice(1));
  if (!h) { showProject(null); document.getElementById('content').scrollTo(0, 0); return; }
  const section = document.getElementById(h);
  if (section && section.classList.contains('file')) {
    const article = section.closest('.proj-view');
    if (article) {
      showProject(article.dataset.key);
      requestAnimationFrame(() => section.scrollIntoView({ block: 'start' }));
      return;
    }
  }
  showProject(h);
  document.getElementById('content').scrollTo(0, 0);
}
document.addEventListener('click', e => {
  const b = e.target.closest('button.proj');
  if (!b) return;
  location.hash = encodeURIComponent(b.dataset.key);
});
window.addEventListener('hashchange', applyHash);
document.addEventListener('DOMContentLoaded', applyHash);
"#;
