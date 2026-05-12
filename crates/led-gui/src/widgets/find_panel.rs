use gpui::*;
use crate::workspace::Workspace;
use led_core::search::{SearchQuery, SearchFlags};

pub struct FindPanel {
    workspace: Entity<Workspace>,
    find_text: String,
    replace_text: String,
    find_focus: FocusHandle,
    replace_focus: FocusHandle,
    is_replace_mode: bool,
    match_case: bool,
    whole_word: bool,
    use_regex: bool,
    is_visible: bool,
}

impl FindPanel {
    pub fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            find_text: String::new(),
            replace_text: String::new(),
            find_focus: cx.focus_handle(),
            replace_focus: cx.focus_handle(),
            is_replace_mode: false,
            match_case: false,
            whole_word: false,
            use_regex: false,
            is_visible: false,
        }
    }

    pub fn show(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.is_visible = true;
        self.is_replace_mode = replace;
        if replace {
            self.replace_focus.focus(window, cx);
        } else {
            self.find_focus.focus(window, cx);
        }
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.is_visible = false;
        cx.notify();
    }

    fn run_search(&self, cx: &mut Context<Self>) {
        let query = SearchQuery {
            pattern: self.find_text.clone(),
            flags: SearchFlags {
                match_case: self.match_case,
                whole_word: self.whole_word,
                use_regex: self.use_regex,
            },
        };
        self.workspace.update(cx, |w, _| {
            let editor = w.active_editor_mut();
            editor.find_results = editor.search(&query);
            if !editor.find_results.is_empty() {
                editor.current_match_idx = Some(0);
                editor.cursor = editor.find_results[0].char_range.start;
                editor.selection = Some(editor.find_results[0].char_range.clone());
            } else {
                editor.current_match_idx = None;
                editor.selection = None;
            }
        });
    }

    fn handle_search_next(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, _| {
            let editor = w.active_editor_mut();
            if editor.find_results.is_empty() { return; }
            let idx = match editor.current_match_idx {
                Some(i) => (i + 1) % editor.find_results.len(),
                None => 0,
            };
            editor.current_match_idx = Some(idx);
            let m = &editor.find_results[idx];
            editor.cursor = m.char_range.start;
            editor.selection = Some(m.char_range.clone());
        });
        cx.notify();
    }

    fn handle_search_prev(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, _| {
            let editor = w.active_editor_mut();
            if editor.find_results.is_empty() { return; }
            let idx = match editor.current_match_idx {
                Some(i) => if i == 0 { editor.find_results.len() - 1 } else { i - 1 },
                None => 0,
            };
            editor.current_match_idx = Some(idx);
            let m = &editor.find_results[idx];
            editor.cursor = m.char_range.start;
            editor.selection = Some(m.char_range.clone());
        });
        cx.notify();
    }

    fn handle_search_replace(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, _| {
            let editor = w.active_editor_mut();
            if let Some(idx) = editor.current_match_idx {
                let m = editor.find_results[idx].clone();
                editor.delete(m.char_range);
                editor.insert(editor.cursor, &self.replace_text);
                // Search again to update results
                // This is a bit inefficient but simple for now
            }
        });
        self.run_search(cx);
        cx.notify();
    }

    fn handle_search_replace_all(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, _| {
            let editor = w.active_editor_mut();
            // We iterate backwards to avoid range invalidation issues
            let results = editor.find_results.clone();
            for m in results.into_iter().rev() {
                editor.delete(m.char_range);
                editor.insert(editor.cursor, &self.replace_text);
            }
        });
        self.run_search(cx);
        cx.notify();
    }

    fn handle_find_keydown(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => {
                if event.keystroke.modifiers.shift {
                    self.handle_search_prev(window, cx);
                } else {
                    self.handle_search_next(window, cx);
                }
            }
            "escape" => self.hide(cx),
            "backspace" => {
                self.find_text.pop();
                self.run_search(cx);
            }
            k if k.len() == 1 => {
                self.find_text.push_str(k);
                self.run_search(cx);
            }
            _ => {}
        }
        cx.notify();
    }

    fn handle_replace_keydown(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.hide(cx),
            "backspace" => {
                self.replace_text.pop();
            }
            k if k.len() == 1 => {
                self.replace_text.push_str(k);
            }
            _ => {}
        }
        cx.notify();
    }

    fn led_color_to_gpui(&self, color: led_core::theme::Color) -> Rgba {
        match color {
            led_core::theme::Color::Rgb(r, g, b) => {
                Rgba {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                    a: 1.0,
                }
            }
            led_core::theme::Color::Ansi(i) => {
                let (r, g, b) = match i {
                    0 => (0, 0, 0),
                    1 => (170, 0, 0),
                    2 => (0, 170, 0),
                    3 => (170, 170, 0),
                    4 => (0, 0, 170),
                    5 => (170, 0, 170),
                    6 => (0, 170, 170),
                    7 => (170, 170, 170),
                    8 => (85, 85, 85),
                    9 => (255, 85, 85),
                    10 => (85, 255, 85),
                    11 => (255, 255, 85),
                    12 => (85, 85, 255),
                    13 => (255, 85, 255),
                    14 => (85, 255, 255),
                    15 => (255, 255, 255),
                    _ => (128, 128, 128),
                };
                Rgba {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                    a: 1.0,
                }
            }
        }
    }
}

impl Render for FindPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().h_0();
        }

        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;
        let bg = self.led_color_to_gpui(theme.ui.panel_bg);
        let fg = self.led_color_to_gpui(theme.ui.panel_fg);
        let border = self.led_color_to_gpui(theme.ui.dialog_border);
        let input_bg = self.led_color_to_gpui(theme.editor.background);
        let active_bg = self.led_color_to_gpui(theme.ui.button_active_bg);
        let active_fg = self.led_color_to_gpui(theme.ui.button_active_fg);

        div()
            .w_full()
            .flex()
            .flex_col()
            .bg(bg)
            .text_color(fg)
            .border_b_1()
            .border_color(border)
            .px_2()
            .py_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_grow()
                            .h_7()
                            .bg(input_bg)
                            .border_1()
                            .border_color(border)
                            .px_2()
                            .track_focus(&self.find_focus)
                            .on_key_down(cx.listener(Self::handle_find_keydown))
                            .child(if self.find_text.is_empty() { "Find...".to_string() } else { self.find_text.clone() })
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                div()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.match_case = !this.match_case;
                                        this.run_search(cx);
                                        cx.notify();
                                    }))
                                    .px_1()
                                    .bg(if self.match_case { active_bg } else { bg })
                                    .text_color(if self.match_case { active_fg } else { fg })
                                    .child("Aa")
                            )
                            .child(
                                div()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.whole_word = !this.whole_word;
                                        this.run_search(cx);
                                        cx.notify();
                                    }))
                                    .px_1()
                                    .bg(if self.whole_word { active_bg } else { bg })
                                    .text_color(if self.whole_word { active_fg } else { fg })
                                    .child("|W|")
                            )
                            .child(
                                div()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.use_regex = !this.use_regex;
                                        this.run_search(cx);
                                        cx.notify();
                                    }))
                                    .px_1()
                                    .bg(if self.use_regex { active_bg } else { bg })
                                    .text_color(if self.use_regex { active_fg } else { fg })
                                    .child(".*")
                            )
                    )
                    .child(
                        div()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.handle_search_prev(window, cx)))
                            .px_2()
                            .bg(active_bg)
                            .text_color(active_fg)
                            .child("Prev")
                    )
                    .child(
                        div()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.handle_search_next(window, cx)))
                            .px_2()
                            .bg(active_bg)
                            .text_color(active_fg)
                            .child("Next")
                    )
            )
            .child(if self.is_replace_mode {
                div()
                    .mt_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_grow()
                            .h_7()
                            .bg(input_bg)
                            .border_1()
                            .border_color(border)
                            .px_2()
                            .track_focus(&self.replace_focus)
                            .on_key_down(cx.listener(Self::handle_replace_keydown))
                            .child(if self.replace_text.is_empty() { "Replace...".to_string() } else { self.replace_text.clone() })
                    )
                    .child(
                        div()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.handle_search_replace(window, cx)))
                            .px_2()
                            .bg(active_bg)
                            .text_color(active_fg)
                            .child("Replace")
                    )
                    .child(
                        div()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.handle_search_replace_all(window, cx)))
                            .px_2()
                            .bg(active_bg)
                            .text_color(active_fg)
                            .child("Replace All")
                    )
            } else {
                div()
            })
    }
}
