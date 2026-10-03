use gpui::*;
use crate::workspace::Workspace;
use zee_core::syntax::TokenType;
use zee_core::theme::Theme;
use crate::widgets::{led_color_to_gpui, mono_font_family, with_alpha};

pub struct EditorView {
    pub workspace: Entity<Workspace>,
    pub focus_handle: FocusHandle,
    last_click_at: Option<std::time::Instant>,
    click_count: usize,
    preedit_text: Option<String>,
    preedit_range: Option<std::ops::Range<usize>>,
    pending_d: bool,
    pending_y: bool,
    pending_c: bool,
    pending_g: bool,
    pending_r: bool,
}

impl EditorView {
    pub fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
        cx.observe(&workspace, |_, _, cx| {
            cx.notify();
        }).detach();
        
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            last_click_at: None,
            click_count: 0,
            preedit_text: None,
            preedit_range: None,
            pending_d: false,
            pending_y: false,
            pending_c: false,
            pending_g: false,
            pending_r: false,
        }
    }

    fn token_color(&self, token_type: TokenType, theme: &Theme) -> Rgba {
        let color = match token_type {
            TokenType::Keyword => theme.syntax.keyword,
            TokenType::TypeName => theme.syntax.type_name,
            TokenType::Function => theme.syntax.function,
            TokenType::String => theme.syntax.string,
            TokenType::Number => theme.syntax.number,
            TokenType::Comment => theme.syntax.comment,
            TokenType::Operator => theme.syntax.operator,
            TokenType::Punctuation => theme.syntax.punctuation,
            TokenType::Constant => theme.syntax.constant,
            TokenType::Attribute => theme.syntax.attribute,
            TokenType::Error => theme.syntax.error,
        };
        led_color_to_gpui(color.unwrap_or(theme.editor.foreground))
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke.key;
        let shift = event.keystroke.modifiers.shift;
        let control = event.keystroke.modifiers.control;
        let cmd = event.keystroke.modifiers.platform;
        let _alt = event.keystroke.modifiers.alt;

        let vi_mode_enabled = self.workspace.read(cx).config.vi_mode;
        let current_vi_mode = self.workspace.read(cx).active_editor().map(|e| e.vi_mode);

        if vi_mode_enabled {
            // Handle Ctrl+V / Cmd+V in Normal/Visual mode for Visual Block
            if (control || cmd) && key.as_str() == "v" {
                if let Some(vi_mode) = current_vi_mode {
                    if vi_mode != zee_core::ViMode::Insert {
                        self.workspace.update(cx, |w, cx| {
                            if let Some(editor) = w.active_editor_mut() {
                                if editor.vi_mode == zee_core::ViMode::VisualBlock {
                                    editor.vi_mode = zee_core::ViMode::Normal;
                                    editor.selection = None;
                                    editor.selection_anchor = None;
                                } else {
                                    editor.vi_mode = zee_core::ViMode::VisualBlock;
                                    editor.ensure_selection();
                                }
                            }
                            cx.notify();
                        });
                        return;
                    }
                }
            }

            // If other Cmd/Ctrl is pressed, key combinations are handled as shortcuts/actions
            if control || cmd {
                return;
            }

            if let Some(vi_mode) = current_vi_mode {
                match vi_mode {
                    zee_core::ViMode::Normal => {
                        self.handle_vi_normal_key(key, shift, cx);
                        return;
                    }
                    zee_core::ViMode::Visual | zee_core::ViMode::VisualLine | zee_core::ViMode::VisualBlock => {
                        self.handle_vi_visual_key(key, cx);
                        return;
                    }
                    zee_core::ViMode::Insert => {
                        if key.as_str() == "escape" {
                            self.workspace.update(cx, |w, cx| {
                                if let Some(editor) = w.active_editor_mut() {
                                    editor.vi_mode = zee_core::ViMode::Normal;
                                    editor.selection = None;
                                    editor.selection_anchor = None;
                                }
                                cx.notify();
                            });
                            return;
                        }
                    }
                }
            }
        } else {
            // If Cmd/Ctrl is pressed, key combinations are handled as shortcuts/actions
            if control || cmd {
                return;
            }
        }

        self.workspace.update(cx, |w, cx| {
            let expand_tab = w.config.expand_tab;
            let tab_size = w.config.tab_size as usize;
            let editor = match w.active_editor_mut() {
                Some(e) => e,
                None => return,
            };
            match key.as_str() {
                "up" => editor.move_cursor_up(shift),
                "down" => editor.move_cursor_down(shift),
                "left" => editor.move_cursor_left(shift),
                "right" => editor.move_cursor_right(shift),
                "home" => editor.move_cursor_home(shift),
                "end" => editor.move_cursor_end(shift),
                "pageup" => {
                    for _ in 0..20 {
                        editor.move_cursor_up(shift);
                    }
                }
                "pagedown" => {
                    for _ in 0..20 {
                        editor.move_cursor_down(shift);
                    }
                }
                "tab" => {
                    let text = if expand_tab {
                        " ".repeat(tab_size)
                    } else {
                        "\t".to_string()
                    };
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    }
                    editor.insert(editor.cursor, &text);
                }
                "backspace" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    } else if editor.cursor > 0 {
                        editor.delete(editor.cursor - 1..editor.cursor);
                    }
                }
                "delete" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    } else if editor.cursor < editor.rope.len_chars() {
                        editor.delete(editor.cursor..editor.cursor + 1);
                    }
                }
                "enter" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    }
                    editor.insert(editor.cursor, "\n");
                }
                "space" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    }
                    editor.insert(editor.cursor, " ");
                }
                _ => {}
            }
            cx.notify();
        });
    }

    fn handle_vi_normal_key(&mut self, key: &str, shift: bool, cx: &mut Context<Self>) {
        if self.pending_r {
            if key != "escape" && key.chars().count() == 1 {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.cursor < editor.rope.len_chars() {
                            let (line, col) = editor.char_to_line_col(editor.cursor);
                            let max_col = editor.get_line_max_col(line);
                            if col < max_col {
                                editor.delete(editor.cursor..editor.cursor + 1);
                                editor.insert(editor.cursor, key);
                                editor.cursor = editor.cursor.saturating_sub(1);
                            }
                        }
                    }
                    cx.notify();
                });
            }
            self.pending_r = false;
            return;
        }

        if self.pending_d {
            let mut text_to_copy = None;
            let mut handled = true;

            self.workspace.update(cx, |w, cx| {
                if let Some(editor) = w.active_editor_mut() {
                    let start_pos = editor.cursor;
                    let target_pos = match key {
                        "d" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            editor.select_line(line);
                            if let Some(range) = editor.selection.clone() {
                                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                                editor.delete(range);
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                            None
                        }
                        "w" => {
                            editor.move_word_forward(false);
                            Some(editor.cursor)
                        }
                        "e" => {
                            editor.move_word_end(false);
                            Some((editor.cursor + 1).min(editor.rope.len_chars()))
                        }
                        "b" => {
                            editor.move_word_backward(false);
                            Some(editor.cursor)
                        }
                        "$" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                            Some(line_end)
                        }
                        "0" | "^" => {
                            let line = editor.rope.char_to_line(editor.cursor);
                            let line_start = editor.rope.line_to_char(line);
                            Some(line_start)
                        }
                        "h" => {
                            Some(editor.cursor.saturating_sub(1))
                        }
                        "l" => {
                            Some((editor.cursor + 1).min(editor.rope.len_chars()))
                        }
                        _ => {
                            handled = false;
                            None
                        }
                    };

                    if let Some(end_pos) = target_pos {
                        let range = if start_pos <= end_pos {
                            start_pos..end_pos
                        } else {
                            end_pos..start_pos
                        };
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                            editor.delete(range.clone());
                            editor.cursor = range.start;
                            editor.selection = None;
                            editor.selection_anchor = None;
                        }
                    }
                }
                cx.notify();
            });

            if let Some(text) = text_to_copy {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            self.pending_d = false;
            if handled {
                return;
            }
        }

        if self.pending_c {
            let mut text_to_copy = None;
            let mut handled = true;

            self.workspace.update(cx, |w, cx| {
                if let Some(editor) = w.active_editor_mut() {
                    let start_pos = editor.cursor;
                    let target_pos = match key {
                        "c" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            let line_start = editor.rope.line_to_char(line);
                            let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                            let range = line_start..line_end;
                            if !range.is_empty() {
                                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                                editor.delete(range);
                                editor.cursor = line_start;
                            }
                            editor.vi_mode = zee_core::ViMode::Insert;
                            None
                        }
                        "w" => {
                            editor.move_word_forward(false);
                            Some(editor.cursor)
                        }
                        "e" => {
                            editor.move_word_end(false);
                            Some((editor.cursor + 1).min(editor.rope.len_chars()))
                        }
                        "b" => {
                            editor.move_word_backward(false);
                            Some(editor.cursor)
                        }
                        "$" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                            Some(line_end)
                        }
                        "0" | "^" => {
                            let line = editor.rope.char_to_line(editor.cursor);
                            let line_start = editor.rope.line_to_char(line);
                            Some(line_start)
                        }
                        _ => {
                            handled = false;
                            None
                        }
                    };

                    if let Some(end_pos) = target_pos {
                        let range = if start_pos <= end_pos {
                            start_pos..end_pos
                        } else {
                            end_pos..start_pos
                        };
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                            editor.delete(range.clone());
                            editor.cursor = range.start;
                            editor.selection = None;
                            editor.selection_anchor = None;
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                }
                cx.notify();
            });

            if let Some(text) = text_to_copy {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            self.pending_c = false;
            if handled {
                return;
            }
        }

        if self.pending_y {
            let mut text_to_copy = None;
            let mut handled = true;

            self.workspace.update(cx, |w, cx| {
                if let Some(editor) = w.active_editor_mut() {
                    let start_pos = editor.cursor;
                    let target_pos = match key {
                        "y" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            editor.select_line(line);
                            if let Some(range) = editor.selection.clone() {
                                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                            None
                        }
                        "w" => {
                            editor.move_word_forward(false);
                            let pos = editor.cursor;
                            editor.cursor = start_pos;
                            Some(pos)
                        }
                        "e" => {
                            editor.move_word_end(false);
                            let pos = (editor.cursor + 1).min(editor.rope.len_chars());
                            editor.cursor = start_pos;
                            Some(pos)
                        }
                        "b" => {
                            editor.move_word_backward(false);
                            let pos = editor.cursor;
                            editor.cursor = start_pos;
                            Some(pos)
                        }
                        "$" => {
                            let (line, _) = editor.char_to_line_col(editor.cursor);
                            let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                            Some(line_end)
                        }
                        "0" | "^" => {
                            let line = editor.rope.char_to_line(editor.cursor);
                            let line_start = editor.rope.line_to_char(line);
                            Some(line_start)
                        }
                        _ => {
                            handled = false;
                            None
                        }
                    };

                    if let Some(end_pos) = target_pos {
                        let range = if start_pos <= end_pos {
                            start_pos..end_pos
                        } else {
                            end_pos..start_pos
                        };
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range).to_string());
                        }
                    }
                }
                cx.notify();
            });

            if let Some(text) = text_to_copy {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            self.pending_y = false;
            if handled {
                return;
            }
        }

        match key {
            "i" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "I" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let line = editor.rope.char_to_line(editor.cursor);
                        let line_str = editor.rope.line(line).to_string();
                        let indent = line_str.chars().take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r').count();
                        editor.cursor = editor.line_col_to_char(line, indent);
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "a" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_right(false);
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "A" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        editor.cursor = editor.line_col_to_char(line, editor.get_line_max_col(line));
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "o" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_end(false);
                        editor.insert(editor.cursor, "\n");
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "O" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_home(false);
                        editor.insert(editor.cursor, "\n");
                        editor.move_cursor_up(false);
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "v" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.vi_mode = zee_core::ViMode::Visual;
                        editor.ensure_selection();
                    }
                    cx.notify();
                });
            }
            "V" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.vi_mode = zee_core::ViMode::VisualLine;
                        editor.ensure_selection();
                        editor.update_selection();
                    }
                    cx.notify();
                });
            }
            "h" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_left(false);
                    }
                    cx.notify();
                });
            }
            "j" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_down(false);
                    }
                    cx.notify();
                });
            }
            "k" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_up(false);
                    }
                    cx.notify();
                });
            }
            "l" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_right(false);
                    }
                    cx.notify();
                });
            }
            "w" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_forward(false);
                    }
                    cx.notify();
                });
            }
            "b" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_backward(false);
                    }
                    cx.notify();
                });
            }
            "e" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_end(false);
                    }
                    cx.notify();
                });
            }
            "0" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let line = editor.rope.char_to_line(editor.cursor);
                        editor.cursor = editor.rope.line_to_char(line);
                    }
                    cx.notify();
                });
            }
            "^" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let line = editor.rope.char_to_line(editor.cursor);
                        let line_str = editor.rope.line(line).to_string();
                        let indent = line_str.chars().take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r').count();
                        editor.cursor = editor.line_col_to_char(line, indent);
                    }
                    cx.notify();
                });
            }
            "$" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        editor.cursor = editor.line_col_to_char(line, editor.get_line_max_col(line));
                    }
                    cx.notify();
                });
            }
            "u" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.undo();
                    }
                    cx.notify();
                });
            }
            "x" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.cursor < editor.rope.len_chars() {
                            editor.delete(editor.cursor..editor.cursor + 1);
                        }
                    }
                    cx.notify();
                });
            }
            "r" => {
                self.pending_r = true;
                return;
            }
            "s" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.cursor < editor.rope.len_chars() {
                            editor.delete(editor.cursor..editor.cursor + 1);
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "S" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        let line_start = editor.rope.line_to_char(line);
                        let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                        let range = line_start..line_end;
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                            editor.delete(range);
                            editor.cursor = line_start;
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "C" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                        let range = editor.cursor..line_end;
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                            editor.delete(range);
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "D" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                        let range = editor.cursor..line_end;
                        if !range.is_empty() {
                            text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                            editor.delete(range);
                        }
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "Y" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        editor.select_line(line);
                        if let Some(range) = editor.selection.clone() {
                            text_to_copy = Some(editor.rope.slice(range).to_string());
                            editor.selection = None;
                            editor.selection_anchor = None;
                        }
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "J" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        let (line, _) = editor.char_to_line_col(editor.cursor);
                        if line + 1 < editor.line_count() {
                            let line_end = editor.line_col_to_char(line, editor.get_line_max_col(line));
                            let next_line_start = editor.rope.line_to_char(line + 1);
                            let next_line_str = editor.rope.line(line + 1).to_string();
                            let next_indent = next_line_str.chars().take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r').count();
                            let next_text_start = next_line_start + next_indent;
                            editor.delete(line_end..next_text_start);
                            editor.insert(line_end, " ");
                            editor.cursor = line_end;
                        }
                    }
                    cx.notify();
                });
            }
            "d" => {
                self.pending_d = true;
                return;
            }
            "c" => {
                self.pending_c = true;
                return;
            }
            "y" => {
                self.pending_y = true;
                return;
            }
            "p" => {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        let text = text.clone();
                        self.workspace.update(cx, |w, cx| {
                            if let Some(editor) = w.active_editor_mut() {
                                if let Some(range) = editor.selection.clone() {
                                    editor.delete(range);
                                }
                                if text.ends_with('\n') {
                                    // Line paste below
                                    let (line, _) = editor.char_to_line_col(editor.cursor);
                                    let next_line_start = if line + 1 < editor.line_count() {
                                        editor.rope.line_to_char(line + 1)
                                    } else {
                                        editor.rope.len_chars()
                                    };
                                    editor.insert(next_line_start, &text);
                                    editor.cursor = next_line_start;
                                } else {
                                    editor.move_cursor_right(false);
                                    editor.insert(editor.cursor, &text);
                                }
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                            cx.notify();
                        });
                    }
                }
            }
            "P" => {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        let text = text.clone();
                        self.workspace.update(cx, |w, cx| {
                            if let Some(editor) = w.active_editor_mut() {
                                if let Some(range) = editor.selection.clone() {
                                    editor.delete(range);
                                }
                                if text.ends_with('\n') {
                                    // Line paste above
                                    let (line, _) = editor.char_to_line_col(editor.cursor);
                                    let line_start = editor.rope.line_to_char(line);
                                    editor.insert(line_start, &text);
                                    editor.cursor = line_start;
                                } else {
                                    editor.insert(editor.cursor, &text);
                                }
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                            cx.notify();
                        });
                    }
                }
            }
            "g" => {
                if self.pending_g {
                    self.workspace.update(cx, |w, cx| {
                        if let Some(editor) = w.active_editor_mut() {
                            editor.cursor = 0;
                            editor.selection = None;
                            editor.selection_anchor = None;
                        }
                        cx.notify();
                    });
                    self.pending_g = false;
                } else {
                    self.pending_g = true;
                }
                return;
            }
            "G" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.cursor = editor.rope.len_chars();
                        let line = editor.line_count().saturating_sub(1);
                        let col = editor.get_line_max_col(line);
                        editor.cursor = editor.line_col_to_char(line, col);
                        editor.selection = None;
                        editor.selection_anchor = None;
                    }
                    cx.notify();
                });
            }
            "escape" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.selection = None;
                        editor.selection_anchor = None;
                    }
                    cx.notify();
                });
                self.pending_d = false;
                self.pending_y = false;
                self.pending_c = false;
                self.pending_g = false;
                self.pending_r = false;
            }
            "up" | "down" | "left" | "right" | "home" | "end" | "pageup" | "pagedown" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        match key {
                            "up" => editor.move_cursor_up(shift),
                            "down" => editor.move_cursor_down(shift),
                            "left" => editor.move_cursor_left(shift),
                            "right" => editor.move_cursor_right(shift),
                            "home" => editor.move_cursor_home(shift),
                            "end" => editor.move_cursor_end(shift),
                            "pageup" => {
                                for _ in 0..20 {
                                    editor.move_cursor_up(shift);
                                }
                            }
                            "pagedown" => {
                                for _ in 0..20 {
                                    editor.move_cursor_down(shift);
                                }
                            }
                            _ => {}
                        }
                    }
                    cx.notify();
                });
            }
            _ => {
                self.pending_d = false;
                self.pending_y = false;
                self.pending_c = false;
                self.pending_g = false;
                self.pending_r = false;
            }
        }
    }

    fn handle_vi_visual_key(&mut self, key: &str, cx: &mut Context<Self>) {
        let current_mode = self.workspace.read(cx).active_editor().map(|e| e.vi_mode);
        let is_block = current_mode == Some(zee_core::ViMode::VisualBlock);

        match key {
            "escape" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.vi_mode = zee_core::ViMode::Normal;
                        editor.selection = None;
                        editor.selection_anchor = None;
                    }
                    cx.notify();
                });
            }
            "v" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.vi_mode == zee_core::ViMode::Visual {
                            editor.vi_mode = zee_core::ViMode::Normal;
                            editor.selection = None;
                            editor.selection_anchor = None;
                        } else {
                            editor.vi_mode = zee_core::ViMode::Visual;
                            editor.update_selection();
                        }
                    }
                    cx.notify();
                });
            }
            "V" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.vi_mode == zee_core::ViMode::VisualLine {
                            editor.vi_mode = zee_core::ViMode::Normal;
                            editor.selection = None;
                            editor.selection_anchor = None;
                        } else {
                            editor.vi_mode = zee_core::ViMode::VisualLine;
                            editor.update_selection();
                        }
                    }
                    cx.notify();
                });
            }
            "h" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_left(true);
                    }
                    cx.notify();
                });
            }
            "j" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_down(true);
                    }
                    cx.notify();
                });
            }
            "k" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_up(true);
                    }
                    cx.notify();
                });
            }
            "l" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_right(true);
                    }
                    cx.notify();
                });
            }
            "w" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_forward(true);
                    }
                    cx.notify();
                });
            }
            "b" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_backward(true);
                    }
                    cx.notify();
                });
            }
            "e" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_word_end(true);
                    }
                    cx.notify();
                });
            }
            "0" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_home(true);
                    }
                    cx.notify();
                });
            }
            "$" => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        editor.move_cursor_end(true);
                    }
                    cx.notify();
                });
            }
            "d" | "x" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.vi_mode == zee_core::ViMode::VisualBlock {
                            text_to_copy = Some(editor.get_visual_block_text());
                            editor.delete_visual_block();
                        } else {
                            if let Some(range) = editor.selection.clone() {
                                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                                editor.delete(range);
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                        }
                        editor.vi_mode = zee_core::ViMode::Normal;
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "c" | "s" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.vi_mode == zee_core::ViMode::VisualBlock {
                            text_to_copy = Some(editor.get_visual_block_text());
                            editor.delete_visual_block();
                        } else {
                            if let Some(range) = editor.selection.clone() {
                                text_to_copy = Some(editor.rope.slice(range.clone()).to_string());
                                editor.delete(range);
                                editor.selection = None;
                                editor.selection_anchor = None;
                            }
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "y" => {
                let mut text_to_copy = None;
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if editor.vi_mode == zee_core::ViMode::VisualBlock {
                            text_to_copy = Some(editor.get_visual_block_text());
                        } else {
                            if let Some(range) = editor.selection.clone() {
                                text_to_copy = Some(editor.rope.slice(range).to_string());
                            }
                        }
                        editor.selection = None;
                        editor.selection_anchor = None;
                        editor.vi_mode = zee_core::ViMode::Normal;
                    }
                    cx.notify();
                });
                if let Some(text) = text_to_copy {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "p" => {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        let text = text.clone();
                        self.workspace.update(cx, |w, cx| {
                            if let Some(editor) = w.active_editor_mut() {
                                if editor.vi_mode == zee_core::ViMode::VisualBlock {
                                    editor.delete_visual_block();
                                } else if let Some(range) = editor.selection.clone() {
                                    editor.delete(range);
                                }
                                editor.insert(editor.cursor, &text);
                                editor.selection = None;
                                editor.selection_anchor = None;
                                editor.vi_mode = zee_core::ViMode::Normal;
                            }
                            cx.notify();
                        });
                    }
                }
            }
            "I" if is_block => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if let Some(anchor) = editor.selection_anchor {
                            let (anchor_line, anchor_col) = editor.char_to_line_col(anchor);
                            let (cursor_line, cursor_col) = editor.char_to_line_col(editor.cursor);
                            let target_line = anchor_line.min(cursor_line);
                            let target_col = anchor_col.min(cursor_col);
                            editor.cursor = editor.line_col_to_char(target_line, target_col);
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            "A" if is_block => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(editor) = w.active_editor_mut() {
                        if let Some(anchor) = editor.selection_anchor {
                            let (anchor_line, anchor_col) = editor.char_to_line_col(anchor);
                            let (cursor_line, cursor_col) = editor.char_to_line_col(editor.cursor);
                            let target_line = anchor_line.min(cursor_line);
                            let target_col = anchor_col.max(cursor_col) + 1;
                            editor.cursor = editor.line_col_to_char(target_line, target_col);
                        }
                        editor.vi_mode = zee_core::ViMode::Insert;
                    }
                    cx.notify();
                });
            }
            _ => {}
        }
    }


    fn mouse_pos_to_char_pos(&self, position: Point<Pixels>, cx: &mut Context<Self>) -> usize {
        let workspace = self.workspace.read(cx);
        let editor = match workspace.active_editor() {
            Some(e) => e,
            None => return 0,
        };
        
        let line_height = px(workspace.config.line_height);
        let font_size = workspace.config.font_size;
        let gutter_width = if workspace.config.line_numbers { px(52.0) } else { px(0.0) };
        let char_width = px(font_size * 0.6);

        let relative_y = position.y - px(36.0); // Offset by tab bar (36px)
        let line_idx = (relative_y / line_height).floor() as i32 + editor.scroll_row as i32;
        let line_idx = line_idx.max(0).min(editor.line_count() as i32 - 1) as usize;

        let relative_x = position.x - gutter_width + px(editor.scroll_col as f32 * font_size * 0.6);
        let col_idx = (relative_x / char_width).round() as i32;
        let col_idx = col_idx.max(0) as usize;

        editor.line_col_to_char(line_idx, col_idx)
    }

    fn handle_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        let now = std::time::Instant::now();
        if let Some(last) = self.last_click_at {
            if now.duration_since(last).as_millis() < 300 {
                self.click_count += 1;
            } else {
                self.click_count = 1;
            }
        } else {
            self.click_count = 1;
        }
        self.last_click_at = Some(now);

        let char_pos = self.mouse_pos_to_char_pos(event.position, cx);
        let workspace_read = self.workspace.read(cx);
        let gutter_width = if workspace_read.config.line_numbers { px(52.0) } else { px(0.0) };
        let is_gutter_click = event.position.x < gutter_width;

        self.workspace.update(cx, |w, cx| {
            let editor = match w.active_editor_mut() {
                Some(e) => e,
                None => return,
            };
            if is_gutter_click {
                let (line, _) = editor.char_to_line_col(char_pos);
                editor.select_line(line);
            } else {
                match self.click_count {
                    1 => {
                        editor.cursor = char_pos;
                        if event.modifiers.shift {
                            editor.ensure_selection();
                            editor.update_selection();
                        } else {
                            editor.selection = None;
                            editor.selection_anchor = Some(char_pos);
                        }
                    }
                    2 => editor.select_word(char_pos),
                    3 => {
                        let (line, _) = editor.char_to_line_col(char_pos);
                        editor.select_line(line);
                    }
                    _ => {
                        editor.cursor = char_pos;
                        editor.selection = None;
                        editor.selection_anchor = Some(char_pos);
                    }
                }
            }
            cx.notify();
        });
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.pressed_button.is_some() {
            let char_pos = self.mouse_pos_to_char_pos(event.position, cx);
            self.workspace.update(cx, |w, cx| {
                let editor = match w.active_editor_mut() {
                    Some(e) => e,
                    None => return,
                };
                editor.cursor = char_pos;
                editor.ensure_selection();
                editor.update_selection();
                cx.notify();
            });
        }
    }

    fn handle_scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let line_height_px = px(self.workspace.read(cx).config.line_height);
        let char_width_px = px(self.workspace.read(cx).config.font_size * 0.6);

        self.workspace.update(cx, |w, cx| {
            let editor = match w.active_editor_mut() {
                Some(e) => e,
                None => return,
            };
            let delta = event.delta.pixel_delta(line_height_px);
            
            // Vertical scroll
            if delta.y != px(0.0) {
                let rows = (delta.y / line_height_px).floor() as i32;
                if rows > 0 {
                    editor.scroll_row = editor.scroll_row.saturating_sub(rows as usize);
                } else {
                    editor.scroll_row = (editor.scroll_row + (-rows) as usize).min(editor.line_count().saturating_sub(1));
                }
            }

            // Horizontal scroll
            if delta.x != px(0.0) {
                let cols = (delta.x / char_width_px).floor() as i32;
                if cols > 0 {
                    editor.scroll_col = editor.scroll_col.saturating_sub(cols as usize);
                } else {
                    editor.scroll_col += (-cols) as usize;
                }
            }
            cx.notify();
        });
    }
}

impl EntityInputHandler for EditorView {
    fn text_for_range(&mut self, range: std::ops::Range<usize>, _actual_range: &mut Option<std::ops::Range<usize>>, _window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor()?;
        let total_chars = editor.rope.len_chars();
        if range.start > total_chars || range.end > total_chars {
            return None;
        }
        Some(editor.rope.slice(range).to_string())
    }

    fn selected_text_range(&mut self, _ignore_disabled_input: bool, _window: &mut Window, cx: &mut Context<Self>) -> Option<UTF16Selection> {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor()?;
        
        let range = editor.selection.clone().unwrap_or(editor.cursor..editor.cursor);
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<std::ops::Range<usize>> {
        self.preedit_range.clone()
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.preedit_text = None;
        self.preedit_range = None;
        cx.notify();
    }

    fn replace_text_in_range(&mut self, replacement_range: Option<std::ops::Range<usize>>, text: &str, _window: &mut Window, cx: &mut Context<Self>) {
        let vi_mode_enabled = self.workspace.read(cx).config.vi_mode;
        let is_normal_or_visual = self.workspace.read(cx).active_editor().map(|e| e.vi_mode != zee_core::ViMode::Insert).unwrap_or(false);
        if vi_mode_enabled && is_normal_or_visual {
            return;
        }

        self.workspace.update(cx, |w, cx| {
            let editor = match w.active_editor_mut() {
                Some(e) => e,
                None => return,
            };
            if let Some(range) = replacement_range {
                editor.delete(range);
                editor.insert(editor.cursor, text);
            } else if let Some(range) = editor.selection.clone() {
                editor.delete(range);
                editor.insert(editor.cursor, text);
            } else {
                editor.insert(editor.cursor, text);
            }
            self.preedit_text = None;
            self.preedit_range = None;
            cx.notify();
        });
    }

    fn replace_and_mark_text_in_range(&mut self, range_to_replace: Option<std::ops::Range<usize>>, text: &str, _marked_range: Option<std::ops::Range<usize>>, _window: &mut Window, cx: &mut Context<Self>) {
        if text.is_empty() {
            self.unmark_text(_window, cx);
            return;
        }
        
        let workspace = self.workspace.read(cx);
        let editor = match workspace.active_editor() {
            Some(e) => e,
            None => return,
        };
        let start_pos = range_to_replace.map(|r| r.start).unwrap_or(editor.cursor);
        
        self.preedit_text = Some(text.to_string());
        self.preedit_range = Some(start_pos..start_pos + text.chars().count());
        cx.notify();
    }

    fn bounds_for_range(&mut self, range_utf16: std::ops::Range<usize>, bounds: Bounds<Pixels>, _window: &mut Window, cx: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor()?;
        
        let line_height = px(workspace.config.line_height);
        let gutter_width = if workspace.config.line_numbers { px(52.0) } else { px(0.0) };
        let char_width = px(workspace.config.font_size * 0.6);

        let (line, col) = editor.char_to_line_col(range_utf16.start);
        
        if line < editor.scroll_row {
            return None;
        }
        
        let visual_row = line - editor.scroll_row;
        let visual_col = (col as i32) - (editor.scroll_col as i32);
        if visual_col < 0 {
            return None;
        }

        let origin_x = bounds.origin.x + gutter_width + (char_width * visual_col as f32);
        let origin_y = bounds.origin.y + (line_height * visual_row as f32);

        Some(Bounds {
            origin: Point::new(origin_x, origin_y),
            size: Size::new(char_width * (range_utf16.end.saturating_sub(range_utf16.start)).max(1) as f32, line_height),
        })
    }

    fn character_index_for_point(&mut self, _point: Point<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) -> Option<usize> {
        None
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let theme = &workspace.theme;

        if workspace.editors.is_empty() {
            return div()
                .track_focus(&self.focus_handle)
                .key_context("Editor")
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .bg(led_color_to_gpui(theme.editor.background))
                .text_color(with_alpha(led_color_to_gpui(theme.editor.foreground), 0.4))
                .font_family(crate::widgets::ui_font_family())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .text_size(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(with_alpha(led_color_to_gpui(theme.editor.foreground), 0.6))
                                .child("zee")
                        )
                        .child(
                            div()
                                .text_size(px(13.0))
                                .flex()
                                .items_center()
                                .gap_2()
                                .child("⌘T : New Tab")
                                .child("•")
                                .child("⌘N : New Window")
                                .child("•")
                                .child("⌘O : Open File")
                        )
                )
                .into_any_element();
        }

        let editor = workspace.active_editor().unwrap();
        let font_family: SharedString = match &workspace.config.font_family {
            Some(f) => SharedString::from(f.clone()),
            None => SharedString::from(mono_font_family()),
        };
        let font_size = px(workspace.config.font_size);
        let line_height = px(workspace.config.line_height);

        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle)
            .key_context("Editor")
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.handle_key_down(event, window, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event, window, cx| {
                this.handle_mouse_down(event, window, cx);
            }))
            .on_mouse_move(cx.listener(|this, event, window, cx| {
                this.handle_mouse_move(event, window, cx);
            }))
            .on_scroll_wheel(cx.listener(|this, event, window, cx| {
                this.handle_scroll(event, window, cx);
            }))
            .w_full()
            .h_full()
            .relative()
            .bg(led_color_to_gpui(theme.editor.background))
            .text_color(led_color_to_gpui(theme.editor.foreground))
            .text_size(font_size)
            .line_height(line_height)
            .font_family(font_family.clone())
            .child(
                canvas(
                    move |_bounds, _window, _cx| {
                        ()
                    },
                    move |bounds, (), window, cx| {
                        if focus_handle.is_focused(window) {
                            window.handle_input(&focus_handle, ElementInputHandler::new(bounds, entity.clone()), cx);
                        }
                    }
                )
                .absolute()
                .top_0()
                .left_0()
                .w_full()
                .h_full()
            )
            .child(
                div()
                    .w_full()
                    .h_full()
                    .font_family(font_family)
                    .child(self.render_lines(workspace, editor))
            )
            .child(self.render_scrollbar(workspace, editor))
            .into_any_element()
    }
}

impl EditorView {
    fn render_scrollbar(&self, workspace: &Workspace, editor: &zee_core::buffer::Editor) -> impl IntoElement {
        let line_count = editor.line_count().max(1);
        let scroll_row = editor.scroll_row;
        let theme = &workspace.theme;

        let visible_lines = 35.0_f32;
        let line_count_f = line_count as f32;

        if line_count <= 35 {
            return div().w_0().h_0().into_any_element();
        }

        let thumb_height_ratio = (visible_lines / line_count_f).clamp(0.08, 0.95);
        let thumb_top_ratio = (scroll_row as f32 / line_count_f).min(1.0 - thumb_height_ratio);

        let thumb_color = with_alpha(led_color_to_gpui(theme.ui.status_bar_fg), 0.22);
        let thumb_hover = with_alpha(led_color_to_gpui(theme.ui.status_bar_fg), 0.45);

        div()
            .absolute()
            .top_0()
            .right_0()
            .w(px(8.0))
            .h_full()
            .py_1()
            .pr_1()
            .child(
                div()
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .w(px(5.0))
                            .rounded_full()
                            .bg(thumb_color)
                            .hover(move |s| s.bg(thumb_hover))
                            .top(rems(thumb_top_ratio * 38.0))
                            .h(rems(thumb_height_ratio * 38.0))
                    )
            )
            .into_any_element()
    }

    fn render_lines(&self, workspace: &Workspace, editor: &zee_core::buffer::Editor) -> impl IntoElement {
        let line_count = editor.line_count();
        let scroll_row = editor.scroll_row;
        
        let word_wrap = false; // Placeholder

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .children(
                (scroll_row..line_count.min(scroll_row + 100)).map(|idx| {
                    if word_wrap {
                        self.render_wrapped_line(idx, workspace, editor).into_any_element()
                    } else {
                        self.render_line(idx, workspace, editor).into_any_element()
                    }
                })
            )
    }

    fn render_wrapped_line(&self, line_idx: usize, workspace: &Workspace, editor: &zee_core::buffer::Editor) -> impl IntoElement {
        let theme = &workspace.theme;
        let line = editor.rope.line(line_idx);
        let line_chars: Vec<char> = line.chars().collect();

        let wraps = editor.wrap_line(line_idx, 80, 4); 

        div()
            .w_full()
            .flex_col()
            .children(wraps.into_iter().enumerate().map(move |(vidx, range)| {
                let start = range.start.min(line_chars.len());
                let end = range.end.min(line_chars.len());
                let chunk: String = if end > start {
                    line_chars[start..end].iter().collect()
                } else {
                    String::new()
                };
                div()
                    .w_full()
                    .flex()
                    .h(px(22.0))
                    .text_size(px(14.0))
                    .child(
                        div()
                            .w(px(50.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .px_2()
                            .text_color(led_color_to_gpui(theme.editor.line_number))
                            .font_family(mono_font_family())
                            .child(if vidx == 0 { (line_idx + 1).to_string() } else { "".to_string() })
                    )
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .font_family(mono_font_family())
                            .child(chunk)
                    )
            }))
    }

    fn render_line(&self, line_idx: usize, workspace: &Workspace, editor: &zee_core::buffer::Editor) -> impl IntoElement {
        let theme = &workspace.theme;

        let line = editor.rope.line(line_idx);
        let mut line_str = line.to_string();
        // Strip line endings for rendering
        if line_str.ends_with('\n') {
            line_str.pop();
            if line_str.ends_with('\r') {
                line_str.pop();
            }
        } else if line_str.ends_with('\r') {
            line_str.pop();
        }
        
        let (cursor_line, _) = editor.char_to_line_col(editor.cursor);
        let is_cursor_line = line_idx == cursor_line;

        let editor_bg = led_color_to_gpui(theme.editor.background);
        let bg: Rgba = if is_cursor_line {
            theme.editor.current_line.map(|c| led_color_to_gpui(c)).unwrap_or(editor_bg)
        } else {
            editor_bg
        };

        let gutter_width = if workspace.config.line_numbers { px(52.0) } else { px(0.0) };
        let gutter_border = with_alpha(led_color_to_gpui(theme.editor.line_number), 0.2);

        // Measure average character width for scrolling/cursor
        let char_width = 8.4; // Default fallback

        div()
            .w_full()
            .flex()
            .h(px(22.0))
            .bg(bg)
            .text_color(led_color_to_gpui(theme.editor.foreground))
            .child(
                div()
                    .flex_none()
                    .w(gutter_width)
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2p5()
                    .border_r_1()
                    .border_color(if workspace.config.line_numbers { gutter_border } else { rgba(0x00000000) })
                    .text_color(led_color_to_gpui(theme.editor.line_number))
                    .font_family(mono_font_family())
                    .child(if workspace.config.line_numbers { (line_idx + 1).to_string() } else { "".to_string() })
            )
            .child(
                div()
                    .flex_grow()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(-(editor.scroll_col as f32 * char_width)))
                            .h_full()
                            .flex()
                            .items_center()
                            .children(self.render_line_content(line_idx, &line_str, workspace, editor, is_cursor_line))
                    )
            )
    }

    fn render_line_content(&self, line_idx: usize, line_str: &str, workspace: &Workspace, editor: &zee_core::buffer::Editor, is_cursor_line: bool) -> Vec<AnyElement> {
        let theme = &workspace.theme;

        let selection = if editor.vi_mode == zee_core::ViMode::VisualBlock {
            let ranges = editor.get_visual_block_ranges();
            let line_start = editor.rope.line_to_char(line_idx);
            let line_end = line_start + editor.rope.line(line_idx).len_chars();
            ranges.into_iter().find(|r| r.start >= line_start && r.start <= line_end)
        } else {
            editor.selection.clone()
        };
        let line_start_char = editor.rope.line_to_char(line_idx);
        let (_, cursor_col) = editor.char_to_line_col(editor.cursor);

        let mut elements = Vec::new();
        let mut cursor_rendered = false;

        let is_block_cursor = workspace.config.vi_mode && editor.vi_mode != zee_core::ViMode::Insert;
        let char_width_val = workspace.config.font_size * 0.6;

        let render_cursor = |elements: &mut Vec<AnyElement>| {
            if let Some(ref preedit) = self.preedit_text {
                elements.push(self.render_preedit_element(preedit, theme));
            }
            if is_block_cursor {
                // Block cursor with alpha overlay
                elements.push(
                    div()
                        .relative()
                        .w(px(0.0))
                        .h_full()
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w(px(char_width_val))
                                .h_full()
                                .bg(with_alpha(led_color_to_gpui(theme.editor.cursor), 0.5))
                                .border_1()
                                .border_color(led_color_to_gpui(theme.editor.cursor))
                        )
                        .into_any_element()
                );
            } else {
                // 2px vertical bar cursor
                elements.push(
                    div()
                        .relative()
                        .w(px(0.0))
                        .h_full()
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w(px(2.0))
                                .h_full()
                                .bg(led_color_to_gpui(theme.editor.cursor))
                        )
                        .into_any_element()
                );
            }
        };

        // Helper to render a chunk of text with potential selection highlight
        let mut render_chunk = |text: &str, start_char: usize, token_color: Option<Rgba>, elements: &mut Vec<AnyElement>| {
            if text.is_empty() { return; }
            let chunk_chars: Vec<char> = text.chars().collect();
            let chunk_len = chunk_chars.len();
            let chunk_start_col = start_char - line_start_char;
            
            // If this is the cursor line, we might need to split this chunk to insert the preedit text and cursor
            if is_cursor_line && !cursor_rendered {
                if cursor_col >= chunk_start_col && cursor_col <= chunk_start_col + chunk_len {
                    let split_idx = cursor_col - chunk_start_col;
                    let part1 = chunk_chars[..split_idx].iter().collect::<String>();
                    let part2 = chunk_chars[split_idx..].iter().collect::<String>();
                    
                    self.render_chunk_internal(&part1, start_char, token_color, selection.clone(), theme, elements);
                    render_cursor(elements);
                    cursor_rendered = true;
                    self.render_chunk_internal(&part2, start_char + split_idx, token_color, selection.clone(), theme, elements);
                    return;
                }
            }

            self.render_chunk_internal(text, start_char, token_color, selection.clone(), theme, elements);
        };

        if let Some(Some(tokens)) = editor.line_tokens.get(line_idx) {
            let mut last_offset = 0;
            let str_len = line_str.len();
            for token in tokens {
                let mut start = token.byte_range.start.min(str_len);
                let mut end = token.byte_range.end.min(str_len);

                // Ensure start and end are on valid UTF-8 character boundaries
                while start > 0 && !line_str.is_char_boundary(start) {
                    start -= 1;
                }
                while end > 0 && !line_str.is_char_boundary(end) {
                    end -= 1;
                }

                if start > last_offset {
                    let text = &line_str[last_offset..start];
                    let start_char = start_char_from_byte_offset(line_str, last_offset, line_start_char);
                    render_chunk(text, start_char, None, &mut elements);
                    last_offset = start;
                }

                if end > last_offset {
                    let text = &line_str[last_offset..end];
                    let start_char = start_char_from_byte_offset(line_str, last_offset, line_start_char);
                    render_chunk(text, start_char, Some(self.token_color(token.token, theme)), &mut elements);
                    last_offset = end;
                }
            }
            if last_offset < str_len {
                let text = &line_str[last_offset..];
                let start_char = start_char_from_byte_offset(line_str, last_offset, line_start_char);
                render_chunk(text, start_char, None, &mut elements);
            }
        } else {
            render_chunk(line_str, line_start_char, None, &mut elements);
        }

        // If line is empty or cursor was at the very end and no chunk captured it
        if is_cursor_line && !cursor_rendered {
            render_cursor(&mut elements);
        }

        elements
    }

    fn render_preedit_element(&self, text: &str, _theme: &Theme) -> AnyElement {
        div()
            .h_full()
            .flex()
            .items_center()
            .text_color(gpui::rgb(0xffffff))
            .bg(gpui::rgb(0x0000ff))
            .border_b_1()
            .border_color(gpui::rgb(0xffffff))
            .font_family(mono_font_family())
            .child(text.to_string())
            .into_any_element()
    }

    fn render_chunk_internal(&self, text: &str, start_char: usize, token_color: Option<Rgba>, selection: Option<std::ops::Range<usize>>, theme: &Theme, elements: &mut Vec<AnyElement>) {
        if text.is_empty() { return; }
        let chunk_chars: Vec<char> = text.chars().collect();
        let chunk_len = chunk_chars.len();
        
        let text_color = token_color.unwrap_or(led_color_to_gpui(theme.editor.foreground));

        if let Some(ref sel) = selection {
            let sel_start = if sel.start > start_char { sel.start - start_char } else { 0 };
            let sel_end = if sel.end > start_char { sel.end - start_char } else { 0 };

            if sel_start < chunk_len && sel_end > 0 {
                let highlight_start = sel_start;
                let highlight_end = sel_end.min(chunk_len);

                if highlight_start > 0 {
                    elements.push(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .text_color(text_color)
                            .font_family(mono_font_family())
                            .child(chunk_chars[..highlight_start].iter().collect::<String>())
                            .into_any_element()
                    );
                }

                elements.push(
                    div()
                        .h_full()
                        .flex()
                        .items_center()
                        .bg(led_color_to_gpui(theme.editor.selection))
                        .text_color(text_color)
                        .font_family(mono_font_family())
                        .child(chunk_chars[highlight_start..highlight_end].iter().collect::<String>())
                        .into_any_element()
                );

                if highlight_end < chunk_len {
                    elements.push(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .text_color(text_color)
                            .font_family(mono_font_family())
                            .child(chunk_chars[highlight_end..].iter().collect::<String>())
                            .into_any_element()
                    );
                }
                return;
            }
        }

        elements.push(
            div()
                .h_full()
                .flex()
                .items_center()
                .text_color(text_color)
                .font_family(mono_font_family())
                .child(text.to_string())
                .into_any_element()
        );
    }
}

fn start_char_from_byte_offset(s: &str, byte_offset: usize, line_start_char: usize) -> usize {
    let mut safe_offset = byte_offset.min(s.len());
    while safe_offset > 0 && !s.is_char_boundary(safe_offset) {
        safe_offset -= 1;
    }
    line_start_char + s[..safe_offset].chars().count()
}
