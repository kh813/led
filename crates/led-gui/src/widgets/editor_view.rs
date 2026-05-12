use gpui::*;
use crate::workspace::Workspace;
use led_core::syntax::TokenType;
use led_core::theme::{Theme};
use unicode_width::UnicodeWidthChar;
use crate::widgets::led_color_to_gpui;

pub struct EditorView {
    pub workspace: Entity<Workspace>,
    pub focus_handle: FocusHandle,
    last_click_at: Option<std::time::Instant>,
    click_count: usize,
    preedit_text: Option<String>,
    preedit_range: Option<std::ops::Range<usize>>,
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
        let _control = event.keystroke.modifiers.control;
        let _cmd = event.keystroke.modifiers.platform;

        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
            match key.as_str() {
                "up" => editor.move_cursor_up(shift),
                "down" => editor.move_cursor_down(shift),
                "left" => editor.move_cursor_left(shift),
                "right" => editor.move_cursor_right(shift),
                "home" => editor.move_cursor_home(shift),
                "end" => editor.move_cursor_end(shift),
                "backspace" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    } else if editor.cursor > 0 {
                        editor.delete(editor.cursor - 1..editor.cursor);
                    }
                }
                "enter" => {
                    if let Some(range) = editor.selection.clone() {
                        editor.delete(range);
                    }
                    editor.insert(editor.cursor, "\n");
                }
                _ => {}
            }
            cx.notify();
        });
    }

    fn mouse_pos_to_char_pos(&self, position: Point<Pixels>, cx: &mut Context<Self>) -> usize {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor();
        
        let line_height = px(20.0);
        let gutter_width = if workspace.config.line_numbers { px(50.0) } else { px(0.0) };
        let char_width = px(8.4); // Rough estimate for system monospace 14pt

        let relative_y = position.y - px(32.0) - px(32.0); // Offset by menu and tab bar
        let line_idx = (relative_y / line_height).floor() as i32 + editor.scroll_row as i32;
        let line_idx = line_idx.max(0).min(editor.line_count() as i32 - 1) as usize;

        let relative_x = position.x - gutter_width + px(editor.scroll_col as f32 * 8.4);
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
        let gutter_width = if workspace_read.config.line_numbers { px(50.0) } else { px(0.0) };
        let is_gutter_click = event.position.x < gutter_width;

        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
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
                let editor = w.active_editor_mut();
                editor.cursor = char_pos;
                editor.ensure_selection();
                editor.update_selection();
                cx.notify();
            });
        }
    }

    fn handle_scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
            let delta = event.delta.pixel_delta(px(20.0));
            
            // Vertical scroll
            if delta.y != px(0.0) {
                let rows = (delta.y / px(20.0)).floor() as i32;
                if rows > 0 {
                    editor.scroll_row = editor.scroll_row.saturating_sub(rows as usize);
                } else {
                    editor.scroll_row = (editor.scroll_row + (-rows) as usize).min(editor.line_count().saturating_sub(1));
                }
            }

            // Horizontal scroll
            if delta.x != px(0.0) {
                let cols = (delta.x / px(8.4)).floor() as i32;
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
        let editor = workspace.active_editor();
        if range.end <= editor.rope.len_chars() {
            Some(editor.rope.slice(range).to_string())
        } else {
            None
        }
    }

    fn selected_text_range(&mut self, _ignore_auto_selection: bool, _window: &mut Window, cx: &mut Context<Self>) -> Option<UTF16Selection> {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor();
        let range = if let Some(ref r) = editor.selection {
            r.start..r.end
        } else {
            editor.cursor..editor.cursor
        };
        Some(UTF16Selection { range, reversed: false })
    }

    fn marked_text_range(&self, _window: &mut Window, cx: &mut Context<Self>) -> Option<std::ops::Range<usize>> {
        if self.preedit_text.is_some() {
            let workspace = self.workspace.read(cx);
            let editor = workspace.active_editor();
            Some(editor.cursor..editor.cursor + self.preedit_text.as_ref().map(|t| t.chars().count()).unwrap_or(0))
        } else {
            None
        }
    }

    fn replace_text_in_range(&mut self, range: Option<std::ops::Range<usize>>, text: &str, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| {
            let editor = w.active_editor_mut();
            let range = range.or(editor.selection.clone()).unwrap_or(editor.cursor..editor.cursor);
            editor.delete(range.clone());
            editor.insert(range.start, text);
            editor.cursor = range.start + text.chars().count();
            editor.selection = None;
            cx.notify();
        });
        self.preedit_text = None;
        self.preedit_range = None;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(&mut self, range: Option<std::ops::Range<usize>>, text: &str, new_selected_range: Option<std::ops::Range<usize>>, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = range {
             self.workspace.update(cx, |w, cx| {
                let editor = w.active_editor_mut();
                editor.delete(range);
                cx.notify();
            });
        }
        self.preedit_text = if text.is_empty() { None } else { Some(text.to_string()) };
        self.preedit_range = new_selected_range;
        cx.notify();
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.preedit_text.take() {
            self.workspace.update(cx, |w, _| {
                let editor = w.active_editor_mut();
                if let Some(range) = editor.selection.clone() {
                    editor.delete(range);
                }
                editor.insert(editor.cursor, &text);
                editor.cursor += text.chars().count();
            });
        }
        self.preedit_range = None;
        cx.notify();
    }

    fn bounds_for_range(&mut self, _range: std::ops::Range<usize>, element_bounds: Bounds<Pixels>, _window: &mut Window, cx: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let workspace = self.workspace.read(cx);
        let editor = workspace.active_editor();
        let (line, col) = editor.char_to_line_col(editor.cursor);
        
        let line_height = px(20.0);
        let char_width = px(8.4);
        let gutter_width = if workspace.config.line_numbers { px(50.0) } else { px(0.0) };
        
        let mut visual_col = 0;
        let line_slice = editor.rope.line(line);
        let tab_size = workspace.config.tab_size as usize;
        for (i, c) in line_slice.chars().enumerate() {
            if i >= col { break; }
            if c == '\t' {
                visual_col += tab_size - (visual_col % tab_size);
            } else {
                visual_col += c.width().unwrap_or(0);
            }
        }

        let x = element_bounds.origin.x + gutter_width + px((visual_col as f32 - editor.scroll_col as f32) * 8.4);
        let y = element_bounds.origin.y + px((line as f32 - editor.scroll_row as f32) * 20.0);
        
        Some(Bounds {
            origin: Point::new(x, y),
            size: size(char_width, line_height),
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
            .text_size(px(14.0))
            .line_height(px(22.0))
            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                    .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
                    .child(self.render_lines(workspace))
            )
    }
}

impl EditorView {
    fn render_lines(&self, workspace: &Workspace) -> impl IntoElement {
        let editor = workspace.active_editor();
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
                        self.render_wrapped_line(idx, workspace).into_any_element()
                    } else {
                        self.render_line(idx, workspace).into_any_element()
                    }
                })
            )
    }

    fn render_wrapped_line(&self, line_idx: usize, workspace: &Workspace) -> impl IntoElement {
        let editor = workspace.active_editor();
        let theme = &workspace.theme;
        let line = editor.rope.line(line_idx);
        let line_str = line.to_string();

        let wraps = editor.wrap_line(line_idx, 80, 4); 

        div()
            .w_full()
            .flex_col()
            .children(wraps.into_iter().enumerate().map(|(vidx, range)| {
                let chunk = &line_str[range.start..range.end];
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
                            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
                            .child(if vidx == 0 { (line_idx + 1).to_string() } else { "".to_string() })
                    )
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
                            .child(chunk.to_string())
                    )
            }))
    }

    fn render_line(&self, line_idx: usize, workspace: &Workspace) -> impl IntoElement {
        let editor = workspace.active_editor();
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

        let gutter_width = if workspace.config.line_numbers { px(50.0) } else { px(0.0) };

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
                    .px_2()
                    .text_color(led_color_to_gpui(theme.editor.line_number))
                    .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                            .children(self.render_line_content(line_idx, &line_str, workspace, is_cursor_line))
                    )
            )
    }

    fn render_line_content(&self, line_idx: usize, line_str: &str, workspace: &Workspace, is_cursor_line: bool) -> Vec<AnyElement> {
        let editor = workspace.active_editor();
        let theme = &workspace.theme;

        let selection = editor.selection.clone();
        let line_start_char = editor.rope.line_to_char(line_idx);
        let (_, cursor_col) = editor.char_to_line_col(editor.cursor);

        let mut elements = Vec::new();
        let mut cursor_rendered = false;

        let render_cursor = |elements: &mut Vec<AnyElement>| {
            if let Some(ref preedit) = self.preedit_text {
                elements.push(self.render_preedit_element(preedit, theme));
            }
            // Cursor bar that doesn't take space
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
            for token in tokens {
                if token.byte_range.start > last_offset {
                    let text = &line_str[last_offset..token.byte_range.start];
                    let start_char = line_start_char + line_str[..last_offset].chars().count();
                    render_chunk(text, start_char, None, &mut elements);
                }
                let text = &line_str[token.byte_range.clone()];
                let start_char = start_char_from_byte_offset(&line_str, token.byte_range.start, line_start_char);
                render_chunk(text, start_char, Some(self.token_color(token.token, theme)), &mut elements);
                last_offset = token.byte_range.end;
            }
            if last_offset < line_str.len() {
                let text = &line_str[last_offset..];
                let start_char = start_char_from_byte_offset(&line_str, last_offset, line_start_char);
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
            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                        .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                            .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
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
                .font_family(if cfg!(target_os = "macos") { ".AppleSystemUIFontMonospaced-Regular" } else { "monospace" })
                .child(text.to_string())
                .into_any_element()
        );
    }
}

fn start_char_from_byte_offset(s: &str, byte_offset: usize, line_start_char: usize) -> usize {
    line_start_char + s[..byte_offset].chars().count()
}
