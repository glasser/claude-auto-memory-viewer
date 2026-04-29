use std::path::Path;
use std::time::SystemTime;

pub struct Project {
    pub real_path: String,
    #[allow(dead_code)]
    pub encoded: String,
    pub files: Vec<MemoryFile>,
}

pub struct MemoryFile {
    pub name: String,
    pub frontmatter: Vec<(String, String)>,
    pub body: String,
    #[allow(dead_code)]
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
