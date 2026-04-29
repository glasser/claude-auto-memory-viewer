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

#[allow(dead_code)]
pub fn scan_all(_home: &Path) -> Vec<Project> {
    Vec::new()
}
