use led_core::buffer::Editor;
use led_core::theme::Theme;
use led_core::config::Config;
use led_core::file_tree::FileTree;
use led_core::outline::{self, OutlineNode};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Files,
    Outline,
}

pub struct Workspace {
    pub editors: Vec<Editor>,
    pub active_editor_index: usize,
    pub theme: Theme,
    pub config: Config,
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
    pub file_tree: FileTree,
    pub outline_nodes: Vec<OutlineNode>,
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        let root_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_tree = FileTree::new(&root_path, false);
        let sidebar_visible = config.sidebar;
        Self {
            editors: vec![Editor::new()],
            active_editor_index: 0,
            theme: Theme::default(),
            config,
            sidebar_visible,
            sidebar_tab: SidebarTab::Files,
            file_tree,
            outline_nodes: Vec::new(),
        }
    }

    pub fn update_outline(&mut self) {
        if let Some(editor) = self.active_editor() {
            let text = editor.rope.to_string();
            self.outline_nodes = outline::parse_markdown_outline(&text);
        } else {
            self.outline_nodes.clear();
        }
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        self.config.sidebar = self.sidebar_visible;
        let _ = Config::write_key("sidebar", &self.sidebar_visible.to_string());
    }

    pub fn jump_to_line(&mut self, line: usize) {
        if let Some(editor) = self.active_editor_mut() {
            let line_count = editor.line_count();
            let target_line = line.min(line_count.saturating_sub(1));
            editor.cursor = editor.rope.line_to_char(target_line);
            editor.selection = None;
            editor.selection_anchor = None;
            editor.scroll_row = target_line.saturating_sub(5);
        }
    }

    pub fn active_editor(&self) -> Option<&Editor> {
        self.editors.get(self.active_editor_index)
    }

    pub fn active_editor_mut(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut(self.active_editor_index)
    }

    #[allow(dead_code)]
    pub fn find_editor_by_path(&self, path: &std::path::Path) -> Option<usize> {
        self.editors.iter().position(|e| e.path.as_ref() == Some(&path.to_path_buf()))
    }

    pub fn new_tab(&mut self) {
        self.editors.push(Editor::new());
        self.active_editor_index = self.editors.len() - 1;
    }

    pub fn add_editor(&mut self, editor: Editor) {
        if let Some(path) = &editor.path {
            if let Some(idx) = self.editors.iter().position(|e| e.path.as_ref() == Some(path)) {
                self.active_editor_index = idx;
                return;
            }

            // If there is only 1 tab and it is an untitled, unmodified empty buffer, replace it with the opened file
            if self.editors.len() == 1 {
                let first = &self.editors[0];
                if !first.is_modified() && first.path.is_none() && first.rope.len_chars() == 0 {
                    self.editors[0] = editor;
                    self.active_editor_index = 0;
                    return;
                }
            }

            if self.active_editor_index < self.editors.len() {
                let active = &self.editors[self.active_editor_index];
                if !active.is_modified() && active.path.is_none() && active.rope.len_chars() == 0 {
                    self.editors[self.active_editor_index] = editor;
                    return;
                }
            }
        }

        self.editors.push(editor);
        self.active_editor_index = self.editors.len() - 1;
    }

    pub fn close_editor(&mut self, index: usize) {
        if index < self.editors.len() {
            self.editors.remove(index);
            if self.editors.is_empty() {
                self.active_editor_index = 0;
            } else if self.active_editor_index >= self.editors.len() {
                self.active_editor_index = self.editors.len() - 1;
            } else if self.active_editor_index > index {
                self.active_editor_index -= 1;
            }
        }
    }

    pub fn close_active_editor(&mut self) {
        if !self.editors.is_empty() {
            self.close_editor(self.active_editor_index);
        }
    }

    pub fn next_tab(&mut self) {
        if !self.editors.is_empty() {
            self.active_editor_index = (self.active_editor_index + 1) % self.editors.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.editors.is_empty() {
            self.active_editor_index = (self.active_editor_index + self.editors.len() - 1) % self.editors.len();
        }
    }

    pub fn has_modified_buffers(&self) -> bool {
        self.editors.iter().any(|e| e.is_modified())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tab_always_adds_new_tab() {
        let mut workspace = Workspace::new(Config::default());
        assert_eq!(workspace.editors.len(), 1);
        assert_eq!(workspace.active_editor_index, 0);

        workspace.new_tab();
        assert_eq!(workspace.editors.len(), 2);
        assert_eq!(workspace.active_editor_index, 1);

        workspace.new_tab();
        assert_eq!(workspace.editors.len(), 3);
        assert_eq!(workspace.active_editor_index, 2);
    }

    #[test]
    fn test_add_editor_untitled_always_adds_tab() {
        let mut workspace = Workspace::new(Config::default());
        assert_eq!(workspace.editors.len(), 1);

        workspace.add_editor(Editor::new());
        assert_eq!(workspace.editors.len(), 2);
        assert_eq!(workspace.active_editor_index, 1);
    }

    #[test]
    fn test_close_editor_until_empty() {
        let mut workspace = Workspace::new(Config::default());
        assert_eq!(workspace.editors.len(), 1);
        workspace.close_editor(0);
        assert_eq!(workspace.editors.len(), 0);
        assert!(workspace.active_editor().is_none());
    }

    #[test]
    fn test_add_editor_replaces_clean_initial_buffer() {
        let mut workspace = Workspace::new(Config::default());
        assert_eq!(workspace.editors.len(), 1);
        assert!(workspace.editors[0].path.is_none());

        let mut new_editor = Editor::new();
        new_editor.path = Some(std::path::PathBuf::from("/tmp/test.rs"));
        workspace.add_editor(new_editor);
        assert_eq!(workspace.editors.len(), 1);
        assert_eq!(workspace.active_editor().unwrap().path.as_ref().unwrap().to_str().unwrap(), "/tmp/test.rs");
    }

    #[test]
    fn test_add_editor_keeps_modified_buffer() {
        let mut workspace = Workspace::new(Config::default());
        workspace.active_editor_mut().unwrap().insert(0, "modified text");
        assert!(workspace.has_modified_buffers());

        let mut new_editor = Editor::new();
        new_editor.path = Some(std::path::PathBuf::from("/tmp/second.rs"));
        workspace.add_editor(new_editor);
        assert_eq!(workspace.editors.len(), 2);
        assert_eq!(workspace.active_editor_index, 1);
    }

    #[test]
    fn test_close_active_editor_until_empty() {
        let mut workspace = Workspace::new(Config::default());
        workspace.new_tab();
        assert_eq!(workspace.editors.len(), 2);

        workspace.close_active_editor();
        assert_eq!(workspace.editors.len(), 1);

        workspace.close_active_editor();
        assert_eq!(workspace.editors.len(), 0);
        assert!(workspace.active_editor().is_none());

        // Calling close_active_editor on empty workspace should be a safe no-op
        workspace.close_active_editor();
        assert_eq!(workspace.editors.len(), 0);
    }
}
