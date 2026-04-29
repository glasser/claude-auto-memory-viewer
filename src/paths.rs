use std::collections::HashMap;
use serde_json::Value;

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
