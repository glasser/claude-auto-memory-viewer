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
