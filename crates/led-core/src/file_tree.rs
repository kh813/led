use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone)]
pub struct FlatFileItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub has_children: bool,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct FileTree {
    pub root_path: PathBuf,
    pub root_node: FileTreeNode,
    pub show_hidden: bool,
}

impl FileTree {
    pub fn new<P: AsRef<Path>>(root_path: P, show_hidden: bool) -> Self {
        let root_path = root_path.as_ref().to_path_buf();
        let name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root_path.to_string_lossy().to_string());

        let mut root_node = FileTreeNode {
            name,
            path: root_path.clone(),
            is_dir: true,
            is_expanded: true,
            children: Vec::new(),
        };

        Self::populate_children(&mut root_node, show_hidden);

        Self {
            root_path,
            root_node,
            show_hidden,
        }
    }

    pub fn refresh(&mut self) {
        Self::refresh_node(&mut self.root_node, self.show_hidden);
    }

    fn refresh_node(node: &mut FileTreeNode, show_hidden: bool) {
        if node.is_dir {
            let mut existing_expanded: Vec<PathBuf> = Vec::new();
            Self::collect_expanded(node, &mut existing_expanded);

            Self::populate_children(node, show_hidden);
            Self::restore_expanded(node, &existing_expanded, show_hidden);
        }
    }

    fn collect_expanded(node: &FileTreeNode, list: &mut Vec<PathBuf>) {
        if node.is_dir && node.is_expanded {
            list.push(node.path.clone());
            for child in &node.children {
                Self::collect_expanded(child, list);
            }
        }
    }

    fn restore_expanded(node: &mut FileTreeNode, list: &[PathBuf], show_hidden: bool) {
        if node.is_dir && list.contains(&node.path) {
            node.is_expanded = true;
            if node.children.is_empty() {
                Self::populate_children(node, show_hidden);
            }
            for child in &mut node.children {
                Self::restore_expanded(child, list, show_hidden);
            }
        }
    }

    pub fn toggle_expand(&mut self, target_path: &Path) -> bool {
        Self::toggle_expand_node(&mut self.root_node, target_path, self.show_hidden)
    }

    fn toggle_expand_node(node: &mut FileTreeNode, target_path: &Path, show_hidden: bool) -> bool {
        if node.path == target_path {
            if node.is_dir {
                node.is_expanded = !node.is_expanded;
                if node.is_expanded && node.children.is_empty() {
                    Self::populate_children(node, show_hidden);
                }
                return true;
            }
            return false;
        }

        for child in &mut node.children {
            if Self::toggle_expand_node(child, target_path, show_hidden) {
                return true;
            }
        }
        false
    }

    fn populate_children(node: &mut FileTreeNode, show_hidden: bool) {
        if !node.is_dir {
            return;
        }

        let mut children = Vec::new();
        if let Ok(entries) = fs::read_dir(&node.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if !show_hidden && file_name.starts_with('.') {
                    continue;
                }

                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

                children.push(FileTreeNode {
                    name: file_name,
                    path,
                    is_dir,
                    is_expanded: false,
                    children: Vec::new(),
                });
            }
        }

        // Sort: directories first, then alphabetical (case-insensitive)
        children.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        node.children = children;
    }

    pub fn flatten(&self) -> Vec<FlatFileItem> {
        let mut items = Vec::new();
        Self::flatten_node(&self.root_node, 0, &mut items);
        items
    }

    fn flatten_node(node: &FileTreeNode, depth: usize, out: &mut Vec<FlatFileItem>) {
        let has_children = node.is_dir;
        out.push(FlatFileItem {
            name: node.name.clone(),
            path: node.path.clone(),
            is_dir: node.is_dir,
            is_expanded: node.is_expanded,
            has_children,
            depth,
        });

        if node.is_dir && node.is_expanded {
            for child in &node.children {
                Self::flatten_node(child, depth + 1, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_creation_and_flatten() {
        let temp_dir = std::env::temp_dir().join("led_test_tree");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("sub_dir")).unwrap();
        fs::write(temp_dir.join("a.txt"), "hello").unwrap();
        fs::write(temp_dir.join("b.txt"), "world").unwrap();
        fs::write(temp_dir.join("sub_dir").join("c.txt"), "sub").unwrap();

        let mut tree = FileTree::new(&temp_dir, false);
        let flat = tree.flatten();
        // root + sub_dir + a.txt + b.txt (sub_dir is not expanded initially)
        assert_eq!(flat.len(), 4);
        assert!(flat[1].is_dir); // sub_dir sorted first
        assert_eq!(flat[1].name, "sub_dir");

        // Expand sub_dir
        tree.toggle_expand(&temp_dir.join("sub_dir"));
        let flat_expanded = tree.flatten();
        assert_eq!(flat_expanded.len(), 5);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
