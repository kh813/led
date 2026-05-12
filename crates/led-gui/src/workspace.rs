use led_core::buffer::Editor;
use led_core::theme::Theme;
use led_core::config::Config;

pub struct Workspace {
    pub editors: Vec<Editor>,
    pub active_editor_index: usize,
    pub theme: Theme,
    pub config: Config,
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        Self {
            editors: vec![Editor::new()],
            active_editor_index: 0,
            theme: Theme::default(),
            config,
        }
    }

    pub fn active_editor(&self) -> &Editor {
        &self.editors[self.active_editor_index]
    }

    pub fn active_editor_mut(&mut self) -> &mut Editor {
        &mut self.editors[self.active_editor_index]
    }

    pub fn find_editor_by_path(&self, path: &std::path::Path) -> Option<usize> {
        self.editors.iter().position(|e| e.path.as_ref() == Some(&path.to_path_buf()))
    }

    pub fn add_editor(&mut self, editor: Editor) {
        if let Some(path) = &editor.path {
            if let Some(idx) = self.editors.iter().position(|e| e.path.as_ref() == Some(path)) {
                self.active_editor_index = idx;
                return;
            }
        }
        self.editors.push(editor);
        self.active_editor_index = self.editors.len() - 1;
    }

    pub fn close_active_editor(&mut self) {
        if self.editors.len() > 1 {
            self.editors.remove(self.active_editor_index);
            if self.active_editor_index >= self.editors.len() {
                self.active_editor_index = self.editors.len() - 1;
            }
        } else {
            self.editors[0] = Editor::new();
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
