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
    #[allow(dead_code)]
    pub mtime: SystemTime,
    pub is_orphan: bool,
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

    fn mem(name: &str, body: &str) -> MemoryFile {
        MemoryFile {
            name: name.into(),
            frontmatter: vec![],
            body: body.into(),
            mtime: SystemTime::UNIX_EPOCH,
            is_orphan: false,
        }
    }

    #[test]
    fn extract_index_picks_up_bare_md_links_in_order() {
        let body = "- [a](feedback_a.md)\n- [b](feedback_b.md)\n- [c](http://x/y.md)\n";
        assert_eq!(
            extract_memory_index_order(body),
            vec!["feedback_a.md".to_string(), "feedback_b.md".to_string()],
        );
    }

    #[test]
    fn extract_index_strips_anchor_and_dot_slash_prefix() {
        let body = "[a](./foo.md#section) [b](bar.md)";
        assert_eq!(
            extract_memory_index_order(body),
            vec!["foo.md".to_string(), "bar.md".to_string()],
        );
    }

    #[test]
    fn extract_index_dedups_repeated_references() {
        let body = "[a](foo.md) [b](foo.md)";
        assert_eq!(extract_memory_index_order(body), vec!["foo.md".to_string()]);
    }

    #[test]
    fn order_files_uses_memory_md_order() {
        let files = vec![
            mem("zzz.md", ""),
            mem("MEMORY.md", "[a](aaa.md) [b](bbb.md)"),
            mem("aaa.md", ""),
            mem("bbb.md", ""),
        ];
        let ordered = order_files(files);
        let names: Vec<&str> = ordered.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["MEMORY.md", "aaa.md", "bbb.md", "zzz.md"]);
    }

    #[test]
    fn order_files_marks_orphans_when_memory_md_exists() {
        let files = vec![
            mem("MEMORY.md", "[a](aaa.md)"),
            mem("aaa.md", ""),
            mem("orphan.md", ""),
        ];
        let ordered = order_files(files);
        let by_name: std::collections::HashMap<&str, bool> =
            ordered.iter().map(|f| (f.name.as_str(), f.is_orphan)).collect();
        assert_eq!(by_name["MEMORY.md"], false);
        assert_eq!(by_name["aaa.md"], false);
        assert_eq!(by_name["orphan.md"], true);
    }

    #[test]
    fn order_files_no_memory_md_means_no_orphans() {
        let files = vec![mem("foo.md", ""), mem("bar.md", "")];
        let ordered = order_files(files);
        assert!(ordered.iter().all(|f| !f.is_orphan));
        let names: Vec<&str> = ordered.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["bar.md", "foo.md"]);
    }
}

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
    let raw: Vec<MemoryFile> = entries
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
                is_orphan: false,
            })
        })
        .collect();
    order_files(raw)
}

pub fn order_files(files: Vec<MemoryFile>) -> Vec<MemoryFile> {
    let mut by_name: std::collections::HashMap<String, MemoryFile> =
        files.into_iter().map(|f| (f.name.clone(), f)).collect();
    let memory_order = match by_name.get("MEMORY.md") {
        Some(f) => extract_memory_index_order(&f.body),
        None => Vec::new(),
    };
    let has_memory = by_name.contains_key("MEMORY.md");

    let mut sorted = Vec::with_capacity(by_name.len());
    if let Some(f) = by_name.remove("MEMORY.md") {
        sorted.push(f);
    }
    for name in &memory_order {
        if let Some(f) = by_name.remove(name) {
            sorted.push(f);
        }
    }
    let mut remaining: Vec<MemoryFile> = by_name.into_values().collect();
    remaining.sort_by(|a, b| a.name.cmp(&b.name));
    for mut f in remaining {
        if has_memory {
            f.is_orphan = true;
        }
        sorted.push(f);
    }
    sorted
}

pub fn extract_memory_index_order(content: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut order = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b')' {
                j += 1;
            }
            if j < bytes.len() {
                if let Some(name) = clean_intra_md_link(&content[start..j]) {
                    if seen.insert(name.clone()) {
                        order.push(name);
                    }
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    order
}

fn clean_intra_md_link(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.contains("://") || url.starts_with('/') {
        return None;
    }
    let url = url.split('#').next()?;
    let url = url.strip_prefix("./").unwrap_or(url);
    if !url.ends_with(".md") {
        return None;
    }
    Some(url.to_string())
}
