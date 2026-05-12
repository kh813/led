use gpui::*;
use crate::workspace::Workspace;
use led_core::i18n::I18n;
use crate::widgets::led_color_to_gpui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChangesIntent {
    Quit,
    CloseTab,
}

pub enum DialogType {
    About,
    GoToLine,
    #[allow(dead_code)]
    OpenFile,
    #[allow(dead_code)]
    SaveAs,
    UnsavedChanges { filename: String, intent: UnsavedChangesIntent },
    #[allow(dead_code)]
    Message { title: String, message: String },
}

pub struct Dialog {
    workspace: Entity<Workspace>,
    i18n: I18n,
    dialog_type: DialogType,
    input_text: String,
    focus_handle: FocusHandle,
    // File browser state
    current_dir: std::path::PathBuf,
    files: Vec<FileEntry>,
    selected_idx: usize,
    show_hidden: bool,
    // Button focus state for UnsavedChanges
    button_idx: usize,
}

#[derive(Clone)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    #[allow(dead_code)]
    modified: std::time::SystemTime,
}

impl Dialog {
    pub fn new(workspace: Entity<Workspace>, i18n: I18n, dialog_type: DialogType, cx: &mut Context<Self>) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let mut this = Self {
            workspace,
            i18n,
            dialog_type,
            input_text: String::new(),
            focus_handle: cx.focus_handle(),
            current_dir,
            files: Vec::new(),
            selected_idx: 0,
            show_hidden: false,
            button_idx: 0,
        };
        this.refresh_files();
        this
    }

    fn refresh_files(&mut self) {
        self.files.clear();
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    self.files.push(FileEntry {
                        name,
                        is_dir: meta.is_dir(),
                        size: meta.len(),
                        modified: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    });
                }
            }
        }
        self.files.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.cmp(&b.name)
            }
        });
        self.selected_idx = 0;
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "tab" => {
                if matches!(self.dialog_type, DialogType::UnsavedChanges { .. }) {
                    if event.keystroke.modifiers.shift {
                        self.button_idx = (self.button_idx + 2) % 3; // 0->2, 1->0, 2->1
                    } else {
                        self.button_idx = (self.button_idx + 1) % 3;
                    }
                }
            }
            "left" => {
                if matches!(self.dialog_type, DialogType::UnsavedChanges { .. }) {
                    self.button_idx = self.button_idx.saturating_sub(1);
                }
            }
            "right" => {
                if matches!(self.dialog_type, DialogType::UnsavedChanges { .. }) {
                    self.button_idx = (self.button_idx + 1).min(2);
                }
            }
            "up" => {
                if matches!(self.dialog_type, DialogType::OpenFile | DialogType::SaveAs) {
                    self.selected_idx = self.selected_idx.saturating_sub(1);
                    if !self.files.is_empty() {
                        self.input_text = self.files[self.selected_idx].name.clone();
                    }
                }
            }
            "down" => {
                if matches!(self.dialog_type, DialogType::OpenFile | DialogType::SaveAs) {
                    if !self.files.is_empty() {
                        self.selected_idx = (self.selected_idx + 1).min(self.files.len() - 1);
                        self.input_text = self.files[self.selected_idx].name.clone();
                    }
                }
            }
            "backspace" => {
                if matches!(self.dialog_type, DialogType::OpenFile | DialogType::SaveAs) && event.keystroke.modifiers.platform {
                    if let Some(parent) = self.current_dir.parent() {
                        self.current_dir = parent.to_path_buf();
                        self.refresh_files();
                    }
                } else {
                    self.input_text.pop();
                }
            }
            "enter" => {
                if let DialogType::UnsavedChanges { intent, .. } = &self.dialog_type {
                    match self.button_idx {
                        0 => {
                            cx.emit(DialogEvent::Save(*intent));
                            self.close(cx);
                        }
                        1 => {
                            cx.emit(DialogEvent::DontSave(*intent));
                            self.close(cx);
                        }
                        _ => self.close(cx),
                    }
                } else {
                    self.confirm(cx);
                }
            }
            "escape" => {
                self.close(cx);
            }
            k if k.len() == 1 => {
                self.input_text.push_str(k);
            }
            _ => {}
        }
        cx.notify();
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        match &self.dialog_type {
            DialogType::GoToLine => {
                if let Ok(line) = self.input_text.parse::<usize>() {
                    self.workspace.update(cx, |w, _| {
                        let editor = w.active_editor_mut();
                        editor.cursor = editor.rope.line_to_char(line.saturating_sub(1));
                        editor.selection = None;
                    });
                }
            }
            DialogType::OpenFile | DialogType::SaveAs => {
                if !self.files.is_empty() && self.files[self.selected_idx].is_dir && self.input_text == self.files[self.selected_idx].name {
                    self.current_dir.push(&self.input_text);
                    self.input_text.clear();
                    self.refresh_files();
                    return;
                }

                let path = self.current_dir.join(&self.input_text);
                if matches!(self.dialog_type, DialogType::OpenFile) {
                    self.workspace.update(cx, |w, _| {
                        if let Ok(editor) = led_core::buffer::Editor::from_file(&path) {
                            w.add_editor(editor);
                        }
                    });
                } else {
                    self.workspace.update(cx, |w, _| {
                        let editor = w.active_editor_mut();
                        let _ = editor.save_as(&path);
                    });
                }
            }
            _ => {}
        }
        self.close(cx);
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(DialogEvent::Close);
    }
}

pub enum DialogEvent {
    Close,
    #[allow(dead_code)]
    Confirm,
    Save(UnsavedChangesIntent),
    DontSave(UnsavedChangesIntent),
}

impl EventEmitter<DialogEvent> for Dialog {}

impl Render for Dialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;
        let bg = led_color_to_gpui(theme.ui.dialog_bg);
        let fg = led_color_to_gpui(theme.editor.foreground);
        let border = led_color_to_gpui(theme.ui.dialog_border);

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000066)) // Dim background
            .child(
                div()
                    .w(px(500.0))
                    .max_h(px(600.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
                    .border_1()
                    .border_color(border)
                    .shadow_xl()
                    .px_4()
                    .py_4()
                    .flex()
                    .flex_col()
                    .track_focus(&self.focus_handle)
                    .on_key_down(cx.listener(Self::handle_keydown))
                    .child(self.render_content(cx))
            )
    }
}

impl Dialog {
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;

        match &self.dialog_type {
            DialogType::About => {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(div().h_full().flex().items_center().text_size(px(20.0)).child("led-gui"))
                    .child(div().h_full().flex().items_center().child(format!("{} 0.1.0", self.i18n.get("about.version"))))
                    .child(div().h_full().flex().items_center().child("MIT License"))
                    .child(
                        div()
                            .mt_4()
                            .h_8()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
                            .child(self.i18n.get("dialog.ok").to_string())
                    )
            }
            DialogType::GoToLine => {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().h_8().flex().items_center().child(self.i18n.get("dialog.go_to_line").to_string()))
                    .child(
                        div()
                            .h_8()
                            .bg(led_color_to_gpui(theme.ui.panel_bg))
                            .border_1()
                            .border_color(led_color_to_gpui(theme.ui.dialog_border))
                            .px_2()
                            .flex()
                            .items_center()
                            .child(if self.input_text.is_empty() { "Line number...".to_string() } else { self.input_text.clone() })
                    )
            }
            DialogType::OpenFile | DialogType::SaveAs => {
                let title = if matches!(self.dialog_type, DialogType::OpenFile) {
                    self.i18n.get("dialog.open_file")
                } else {
                    self.i18n.get("dialog.save_as")
                };

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().h_8().flex().items_center().child(title.to_string()))
                    .child(div().h_6().flex().items_center().text_size(px(12.0)).child(self.current_dir.to_string_lossy().into_owned()))
                    .child(
                        div()
                            .flex_grow()
                            .min_h(px(200.0))
                            .overflow_hidden()
                            .bg(led_color_to_gpui(theme.editor.background))
                            .border_1()
                            .border_color(led_color_to_gpui(theme.ui.dialog_border))
                            .children(self.files.iter().enumerate().map(|(idx, entry)| {
                                let is_selected = idx == self.selected_idx;
                                div()
                                    .h(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .bg(if is_selected { led_color_to_gpui(theme.editor.selection) } else { hsla(0.,0.,0.,0.).into() })
                                    .child(div().h_full().flex().items_center().px_2().child(format!("{}{}", entry.name, if entry.is_dir { "/" } else { "" })))
                                    .child(div().h_full().flex().items_center().px_2().child(if entry.is_dir { "--".to_string() } else { self.format_size(entry.size) }))
                            }))
                    )
                    .child(
                        div()
                            .mt_2()
                            .h_8()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().h_full().flex().items_center().child(self.i18n.get("dialog.file_browser.filename").to_string()))
                            .child(
                                div()
                                    .flex_grow()
                                    .h_full()
                                    .bg(led_color_to_gpui(theme.ui.panel_bg))
                                    .border_1()
                                    .border_color(led_color_to_gpui(theme.ui.dialog_border))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .child(self.input_text.clone())
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .mt_2()
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.confirm(cx)))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.button_active_bg))
                                    .text_color(led_color_to_gpui(theme.ui.button_active_fg))
                                    .child(self.i18n.get("dialog.ok").to_string())
                            )
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.panel_bg))
                                    .child(self.i18n.get("dialog.cancel").to_string())
                            )
                    )
            }
            DialogType::UnsavedChanges { filename, intent: _ } => {
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().h_8().flex().items_center().child(self.i18n.get("dialog.unsaved_changes_title").to_string()))
                    .child(div().flex().items_center().child(self.i18n.get("dialog.unsaved_changes").replace("{filename}", filename)))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        if let DialogType::UnsavedChanges { intent, .. } = &this.dialog_type {
                                            cx.emit(DialogEvent::Save(*intent));
                                        }
                                        this.close(cx);
                                    }))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.button_active_bg))
                                    .text_color(led_color_to_gpui(theme.ui.button_active_fg))
                                    .border_2()
                                    .border_color(if self.button_idx == 0 { gpui::rgb(0xffffff) } else { hsla(0.,0.,0.,0.).into() })
                                    .child(self.i18n.get("dialog.save").to_string())
                            )
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        if let DialogType::UnsavedChanges { intent, .. } = &this.dialog_type {
                                            cx.emit(DialogEvent::DontSave(*intent));
                                        }
                                        this.close(cx);
                                    }))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.panel_bg))
                                    .border_2()
                                    .border_color(if self.button_idx == 1 { gpui::rgb(0xffffff) } else { hsla(0.,0.,0.,0.).into() })
                                    .child(self.i18n.get("dialog.dont_save").to_string())
                            )
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.panel_bg))
                                    .border_2()
                                    .border_color(if self.button_idx == 2 { gpui::rgb(0xffffff) } else { hsla(0.,0.,0.,0.).into() })
                                    .child(self.i18n.get("dialog.cancel").to_string())
                            )
                    )
            }
            DialogType::Message { title, message } => {
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().h_8().flex().items_center().child(title.clone()))
                    .child(div().flex().items_center().child(message.clone()))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .h_8()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
                                    .px_4()
                                    .bg(led_color_to_gpui(theme.ui.button_active_bg))
                                    .text_color(led_color_to_gpui(theme.ui.button_active_fg))
                                    .child(self.i18n.get("dialog.ok").to_string())
                            )
                    )
            }
        }
    }

    fn format_size(&self, size: u64) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f32 / 1024.0)
        } else {
            format!("{:.1} MB", size as f32 / (1024.0 * 1024.0))
        }
    }
}
