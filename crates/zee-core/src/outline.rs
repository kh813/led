use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineNode {
    pub title: String,
    pub level: usize,
    pub line: usize, // 0-based line index in buffer
    pub children: Vec<OutlineNode>,
    pub is_expanded: bool,
}

impl OutlineNode {
    pub fn new(title: String, level: usize, line: usize) -> Self {
        Self {
            title,
            level,
            line,
            children: Vec::new(),
            is_expanded: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlatOutlineItem {
    pub title: String,
    pub level: usize,
    pub depth: usize,
    pub line: usize,
    pub is_expanded: bool,
    pub has_children: bool,
    pub node_id: usize, // index in flat list or identifier
}

/// Parses Markdown content and returns a hierarchical tree of headings.
pub fn parse_markdown_outline(content: &str) -> Vec<OutlineNode> {
    let mut root_nodes: Vec<OutlineNode> = Vec::new();
    let mut stack: Vec<OutlineNode> = Vec::new();
    let mut in_code_block = false;

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        if let Some((level, title)) = parse_heading_line(trimmed) {
            let node = OutlineNode::new(title, level, line_idx);

            while let Some(top) = stack.last() {
                if top.level >= level {
                    let popped = stack.pop().unwrap();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(popped);
                    } else {
                        root_nodes.push(popped);
                    }
                } else {
                    break;
                }
            }
            stack.push(node);
        }
    }

    while let Some(popped) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(popped);
        } else {
            root_nodes.push(popped);
        }
    }

    root_nodes
}

fn parse_heading_line(line: &str) -> Option<(usize, String)> {
    if !line.starts_with('#') {
        return None;
    }
    let count = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&count) {
        let rest = line[count..].trim();
        if !rest.is_empty() {
            return Some((count, rest.to_string()));
        }
    }
    None
}

/// Flattens an outline tree according to expanded/collapsed state.
pub fn flatten_outline(nodes: &[OutlineNode], depth: usize, out: &mut Vec<FlatOutlineItem>) {
    for node in nodes {
        let has_children = !node.children.is_empty();
        out.push(FlatOutlineItem {
            title: node.title.clone(),
            level: node.level,
            depth,
            line: node.line,
            is_expanded: node.is_expanded,
            has_children,
            node_id: out.len(),
        });
        if has_children && node.is_expanded {
            flatten_outline(&node.children, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_outline_parsing() {
        let md = r#"# Main Title
Intro text

## Section 1
Content 1

```markdown
### Code block heading (should be ignored)
```

### Subsection 1.1
Content 1.1

## Section 2
Content 2
"#;
        let outline = parse_markdown_outline(md);
        assert_eq!(outline.len(), 1);
        assert_eq!(outline[0].title, "Main Title");
        assert_eq!(outline[0].line, 0);
        assert_eq!(outline[0].children.len(), 2);

        let s1 = &outline[0].children[0];
        assert_eq!(s1.title, "Section 1");
        assert_eq!(s1.line, 3);
        assert_eq!(s1.children.len(), 1);
        assert_eq!(s1.children[0].title, "Subsection 1.1");
        assert_eq!(s1.children[0].line, 10);

        let s2 = &outline[0].children[1];
        assert_eq!(s2.title, "Section 2");
        assert_eq!(s2.line, 13);
    }

    #[test]
    fn test_flatten_outline() {
        let mut n1 = OutlineNode::new("Root".to_string(), 1, 0);
        let n2 = OutlineNode::new("Child".to_string(), 2, 5);
        n1.children.push(n2);

        let mut flat = Vec::new();
        flatten_outline(&[n1.clone()], 0, &mut flat);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].title, "Root");
        assert_eq!(flat[1].title, "Child");
        assert_eq!(flat[1].depth, 1);

        // Test collapsed
        n1.is_expanded = false;
        let mut flat_collapsed = Vec::new();
        flatten_outline(&[n1], 0, &mut flat_collapsed);
        assert_eq!(flat_collapsed.len(), 1);
    }
}
