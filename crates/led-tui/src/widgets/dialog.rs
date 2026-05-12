use crate::renderer::{Renderer, Cell};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use crossterm::style::Color;
use std::path::PathBuf;
use std::fs;
use std::time::SystemTime;
use chrono::{DateTime, Local};
use unicode_width::UnicodeWidthChar;

pub enum DialogResult<T> {
    Ok(T),
    Cancel,
    Pending,
}

pub trait Dialog {
    fn title(&self) -> &str;
    fn dimensions(&self) -> (u16, u16); // width, height
    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16);
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action>;
    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action>;
    fn set_error(&mut self, _msg: String) {}
    fn cursor_pos(&self) -> Option<(u16, u16)> { None }
}

#[derive(Debug, Clone)]
pub enum Action {
    ConfirmPath(PathBuf),
    ConfirmLine(usize),
    Confirm,
    Save,
    DontSave,
    Discard,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_idx: usize,
    pub show_hidden: bool,
    pub detect_encoding: bool,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
    pub input_text: String,
    pub input_focused: bool,

    // i18n labels
    pub i18n_hidden: String,
    pub i18n_encoding: String,
    pub i18n_name: String,
    pub i18n_size: String,
    pub i18n_modified: String,
    pub i18n_filename: String,
}

impl FileBrowser {
    pub fn new(path: PathBuf) -> Self {
        let mut browser = Self {
            current_dir: path,
            entries: Vec::new(),
            selected_idx: 0,
            show_hidden: false,
            detect_encoding: true,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            input_text: String::new(),
            input_focused: false,

            i18n_hidden: "Show Hidden (Alt+H)".to_string(),
            i18n_encoding: "Detect Encoding (Alt+E)".to_string(),
            i18n_name: "Name".to_string(),
            i18n_size: "Size".to_string(),
            i18n_modified: "Modified".to_string(),
            i18n_filename: "File name: ".to_string(),
        };
        browser.refresh();
        browser
    }

    pub fn localize(&mut self, i18n: &led_core::I18n) {
        self.i18n_hidden = format!("{} (Alt+H)", i18n.get("dialog.show_hidden"));
        self.i18n_encoding = format!("{} (Alt+E)", i18n.get("dialog.detect_encoding"));
        self.i18n_name = i18n.get("dialog.file_browser.name").to_string();
        if self.i18n_name == "dialog.file_browser.name" { self.i18n_name = "Name".to_string(); }
        self.i18n_size = i18n.get("dialog.file_browser.size").to_string();
        if self.i18n_size == "dialog.file_browser.size" { self.i18n_size = "Size".to_string(); }
        self.i18n_modified = i18n.get("dialog.file_browser.modified").to_string();
        if self.i18n_modified == "dialog.file_browser.modified" { self.i18n_modified = "Modified".to_string(); }
        self.i18n_filename = format!("{}: ", i18n.get("dialog.file_browser.filename"));
        if self.i18n_filename == "dialog.file_browser.filename: " { self.i18n_filename = "File name: ".to_string(); }
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        
        // Add parent dir if not at root
        if let Some(_) = self.current_dir.parent() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                modified: None,
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                let metadata = entry.metadata().ok();
                self.entries.push(FileEntry {
                    name,
                    is_dir: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: metadata.and_then(|m| m.modified().ok()),
                });
            }
        }

        self.sort();
        self.selected_idx = self.selected_idx.min(self.entries.len().saturating_sub(1));
    }

    pub fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            // Dirs always before files
            if a.name == ".." { return std::cmp::Ordering::Less; }
            if b.name == ".." { return std::cmp::Ordering::Greater; }
            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir);
            }

            let res = match self.sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => a.size.cmp(&b.size),
                SortBy::Modified => a.modified.cmp(&b.modified),
            };

            if self.sort_order == SortOrder::Descending {
                res.reverse()
            } else {
                res
            }
        });
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PathBuf> {
        if self.input_focused {
            match key.code {
                KeyCode::Char(c) => {
                    self.input_text.push(c);
                    return None;
                }
                KeyCode::Backspace => {
                    self.input_text.pop();
                    return None;
                }
                KeyCode::Enter => {
                    if !self.input_text.is_empty() {
                        return Some(self.current_dir.join(&self.input_text));
                    }
                }
                KeyCode::Tab => {
                    self.input_focused = false;
                    return None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Up => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                    if let Some(entry) = self.entries.get(self.selected_idx) {
                        if !entry.is_dir {
                            self.input_text = entry.name.clone();
                        }
                    }
                }
                None
            }
            KeyCode::Down => {
                if self.selected_idx + 1 < self.entries.len() {
                    self.selected_idx += 1;
                    if let Some(entry) = self.entries.get(self.selected_idx) {
                        if !entry.is_dir {
                            self.input_text = entry.name.clone();
                        }
                    }
                }
                None
            }
            KeyCode::Enter => {
                if let Some(entry) = self.entries.get(self.selected_idx) {
                    let path = self.current_dir.join(&entry.name);
                    if entry.is_dir {
                        if entry.name == ".." {
                            if let Some(parent) = self.current_dir.parent() {
                                self.current_dir = parent.to_path_buf();
                            }
                        } else {
                            self.current_dir = path;
                        }
                        self.refresh();
                        self.selected_idx = 0;
                        None
                    } else {
                        Some(path)
                    }
                } else {
                    None
                }
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh();
                    self.selected_idx = 0;
                }
                None
            }
            KeyCode::Tab => {
                self.input_focused = true;
                None
            }
            KeyCode::Char('h') if key.modifiers == KeyModifiers::ALT => {
                self.show_hidden = !self.show_hidden;
                self.refresh();
                None
            }
            KeyCode::Char('e') if key.modifiers == KeyModifiers::ALT => {
                self.detect_encoding = !self.detect_encoding;
                None
            }
            KeyCode::Char(c) if c.is_alphanumeric() => {
                // Typeahead
                let search = c.to_lowercase().to_string();
                if let Some(idx) = self.entries.iter().position(|e| e.name.to_lowercase().starts_with(&search)) {
                    self.selected_idx = idx;
                    if !self.entries[idx].is_dir {
                        self.input_text = self.entries[idx].name.clone();
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> Option<PathBuf> {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return None;
        }

        let (mx, my) = (mouse.column, mouse.row);

        // Options bar (Show Hidden / Detect Encoding)
        if my == y + 2 {
            let hidden_x = x + 2;
            let hidden_w = 20;
            if mx >= hidden_x && mx < hidden_x + hidden_w {
                self.show_hidden = !self.show_hidden;
                self.refresh();
                return None;
            }
            let enc_x = x + 25;
            let enc_w = 25;
            if mx >= enc_x && mx < enc_x + enc_w {
                self.detect_encoding = !self.detect_encoding;
                return None;
            }
        }

        // Quick-nav bar
        if my == y + 3 {
            let navs = ["..", "/", "~", "Docs", "Downloads"];
            let mut nx = x + 2;
            for nav in navs {
                let nav_len = nav.len() as u16 + 2;
                if mx >= nx && mx < nx + nav_len {
                    match nav {
                        ".." => {
                            if let Some(parent) = self.current_dir.parent() {
                                self.current_dir = parent.to_path_buf();
                            }
                        }
                        "/" => self.current_dir = PathBuf::from("/"),
                        "~" => {
                            if let Some(home) = std::env::var_os("HOME") {
                                self.current_dir = PathBuf::from(home);
                            }
                        }
                        "Docs" => {
                            if let Some(home) = std::env::var_os("HOME") {
                                self.current_dir = PathBuf::from(home).join("Documents");
                            }
                        }
                        "Downloads" => {
                            if let Some(home) = std::env::var_os("HOME") {
                                self.current_dir = PathBuf::from(home).join("Downloads");
                            }
                        }
                        _ => {}
                    }
                    self.refresh();
                    self.selected_idx = 0;
                    return None;
                }
                nx += nav_len + 1;
            }
        }

        // Header click sorting
        if my == y + 4 {
            let name_x = x + 2;
            let size_x = x + w - 25;
            let mod_x = x + w - 14;
            
            if mx >= name_x && mx < size_x {
                if self.sort_by == SortBy::Name {
                    self.sort_order = if self.sort_order == SortOrder::Ascending { SortOrder::Descending } else { SortOrder::Ascending };
                } else {
                    self.sort_by = SortBy::Name;
                    self.sort_order = SortOrder::Ascending;
                }
                self.sort();
            } else if mx >= size_x && mx < mod_x {
                if self.sort_by == SortBy::Size {
                    self.sort_order = if self.sort_order == SortOrder::Ascending { SortOrder::Descending } else { SortOrder::Ascending };
                } else {
                    self.sort_by = SortBy::Size;
                    self.sort_order = SortOrder::Ascending;
                }
                self.sort();
            } else if mx >= mod_x && mx < x + w - 2 {
                if self.sort_by == SortBy::Modified {
                    self.sort_order = if self.sort_order == SortOrder::Ascending { SortOrder::Descending } else { SortOrder::Ascending };
                } else {
                    self.sort_by = SortBy::Modified;
                    self.sort_order = SortOrder::Ascending;
                }
                self.sort();
            }
            return None;
        }

        // Entries
        let visible_count = h.saturating_sub(9) as usize;
        if my >= y + 5 && my < y + 5 + visible_count as u16 {
            let click_idx = (my - (y + 5)) as usize;
            let start_idx = if self.selected_idx >= visible_count {
                self.selected_idx - visible_count + 1
            } else {
                0
            };
            let idx = start_idx + click_idx;
            if idx < self.entries.len() {
                self.selected_idx = idx;
                self.input_focused = false;
                if let Some(entry) = self.entries.get(idx) {
                    if !entry.is_dir {
                        self.input_text = entry.name.clone();
                    }
                }
            }
        }

        // Input field
        if my == y + h - 2 {
            self.input_focused = true;
        }

        None
    }

    pub fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        let dialog_bg = to_ct_color(theme.ui.dialog_bg, theme);
        let dialog_fg = to_ct_color(theme.ui.panel_fg, theme);
        let active_bg = to_ct_color(theme.ui.button_active_bg, theme);
        let active_fg = to_ct_color(theme.ui.button_active_fg, theme);

        // Render current dir
        let dir_str = self.current_dir.to_string_lossy();
        let mut cur_dir_x = x + 2;
        for c in dir_str.chars() {
            let cw = c.width().unwrap_or(0) as u16;
            if cur_dir_x + cw < x + w - 2 {
                renderer.set_cell(cur_dir_x, y + 1, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, width: cw as u8, ..Default::default() });
                cur_dir_x += cw;
            } else {
                break;
            }
        }

        // Options bar
        let hidden_text = format!("[{}] {}", if self.show_hidden { "x" } else { " " }, self.i18n_hidden);
        for (i, c) in hidden_text.chars().enumerate() {
            renderer.set_cell(x + 2 + i as u16, y + 2, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
        }
        let enc_text = format!("[{}] {}", if self.detect_encoding { "x" } else { " " }, self.i18n_encoding);
        for (i, c) in enc_text.chars().enumerate() {
            renderer.set_cell(x + 30 + i as u16, y + 2, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
        }

        // Quick-nav bar
        let navs = ["..", "/", "~", "Docs", "Downloads"];
        let mut nx = x + 2;
        let ny = y + 3;
        for nav in navs {
            let nav_str = format!(" {} ", nav);
            for c in nav_str.chars() {
                renderer.set_cell(nx, ny, Cell { ch: c, bg: to_ct_color(theme.ui.status_bar_bg, theme), fg: to_ct_color(theme.ui.status_bar_fg, theme), ..Default::default() });
                nx += 1;
            }
            nx += 1;
        }

        // Header
        let name_indicator = if self.sort_by == SortBy::Name { if self.sort_order == SortOrder::Ascending { " ▲" } else { " ▼" } } else { "" };
        let size_indicator = if self.sort_by == SortBy::Size { if self.sort_order == SortOrder::Ascending { " ▲" } else { " ▼" } } else { "" };
        let mod_indicator = if self.sort_by == SortBy::Modified { if self.sort_order == SortOrder::Ascending { " ▲" } else { " ▼" } } else { "" };

        let name_head = format!("{}{}", self.i18n_name, name_indicator);
        let size_head = format!("{}{}", self.i18n_size, size_indicator);
        let mod_head = format!("{}{}", self.i18n_modified, mod_indicator);

        for (i, c) in name_head.chars().enumerate() {
            renderer.set_cell(x + 2 + i as u16, y + 4, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, bold: true, ..Default::default() });
        }
        for (i, c) in size_head.chars().enumerate() {
            renderer.set_cell(x + w - 25 + i as u16, y + 4, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, bold: true, ..Default::default() });
        }
        for (i, c) in mod_head.chars().enumerate() {
            renderer.set_cell(x + w - 14 + i as u16, y + 4, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, bold: true, ..Default::default() });
        }

        // Entries
        let visible_count = h.saturating_sub(9) as usize;
        let start_idx = if self.selected_idx >= visible_count {
            self.selected_idx - visible_count + 1
        } else {
            0
        };

        for i in 0..visible_count {
            let idx = start_idx + i;
            if let Some(entry) = self.entries.get(idx) {
                let iy = y + 5 + i as u16;
                let is_selected = idx == self.selected_idx;
                let bg = if is_selected { 
                    if self.input_focused {
                        // Inactive but selected
                        to_ct_color(theme.ui.tab_inactive_bg, theme)
                    } else {
                        active_bg 
                    }
                } else { 
                    dialog_bg 
                };
                let fg = if is_selected { 
                    if self.input_focused {
                        to_ct_color(theme.ui.tab_inactive_fg, theme)
                    } else {
                        active_fg
                    }
                } else { 
                    dialog_fg 
                };

                for dx in 1..w - 1 {
                    renderer.set_cell(x + dx, iy, Cell { ch: ' ', bg, ..Default::default() });
                }

                let name = if entry.is_dir && entry.name != ".." {
                    format!("{}/", entry.name)
                } else {
                    entry.name.clone()
                };

                let mut cur_name_x = x + 2;
                for (_j, c) in name.chars().enumerate() {
                    let cw = c.width().unwrap_or(0) as u16;
                    if (cur_name_x + cw) < x + w - 25 {
                        renderer.set_cell(cur_name_x, iy, Cell { ch: c, bg, fg, width: cw as u8, ..Default::default() });
                        cur_name_x += cw;
                    } else {
                        break;
                    }
                }

                if !entry.is_dir {
                    let size_str = if entry.size < 1024 {
                        format!("{} B", entry.size)
                    } else if entry.size < 1024 * 1024 {
                        format!("{:.1} KB", entry.size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", entry.size as f64 / (1024.0 * 1024.0))
                    };
                    for (j, c) in size_str.chars().enumerate() {
                        if (j as u16) < 10 {
                            renderer.set_cell(x + w - 25 + j as u16, iy, Cell { ch: c, bg, fg, ..Default::default() });
                        }
                    }

                    if let Some(m) = entry.modified {
                        let now = SystemTime::now();
                        let duration = now.duration_since(m).unwrap_or(std::time::Duration::from_secs(0));
                        let mod_str = if duration.as_secs() < 24 * 3600 {
                            if duration.as_secs() < 60 {
                                "just now".to_string()
                            } else if duration.as_secs() < 3600 {
                                format!("{}m ago", duration.as_secs() / 60)
                            } else {
                                format!("{}h ago", duration.as_secs() / 3600)
                            }
                        } else {
                            let datetime: DateTime<Local> = m.into();
                            datetime.format("%Y-%m-%d").to_string()
                        };
                        for (j, c) in mod_str.chars().enumerate() {
                            if (j as u16) < 12 {
                                renderer.set_cell(x + w - 14 + j as u16, iy, Cell { ch: c, bg, fg, ..Default::default() });
                            }
                        }
                    }
                }
            }
        }

        // Input field
        let iy = y + h - 2;
        let input_bg = if self.input_focused { active_bg } else { to_ct_color(theme.ui.panel_bg, theme) };
        let input_fg = if self.input_focused { active_fg } else { to_ct_color(theme.ui.panel_fg, theme) };
        
        for (i, c) in self.i18n_filename.chars().enumerate() {
            renderer.set_cell(x + 2 + i as u16, iy, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
        }
        
        let input_x = x + 2 + self.i18n_filename.chars().count() as u16;
        let input_w = w.saturating_sub(self.i18n_filename.chars().count() as u16 + 4);
        for dx in 0..input_w {
            renderer.set_cell(input_x + dx, iy, Cell { ch: ' ', bg: input_bg, ..Default::default() });
        }
        for (i, c) in self.input_text.chars().enumerate() {
            if (i as u16) < input_w {
                renderer.set_cell(input_x + i as u16, iy, Cell { ch: c, bg: input_bg, fg: input_fg, ..Default::default() });
            }
        }
    }
}

fn to_ct_color(c: led_core::theme::Color, theme: &led_core::theme::Theme) -> Color {
    if theme.meta.name == "Terminal Default" {
        match c {
            led_core::theme::Color::Rgb(0, 0, 0) | led_core::theme::Color::Rgb(255, 255, 255) => {
                return Color::Reset;
            }
            led_core::theme::Color::Ansi(i) => {
                return Color::AnsiValue(i);
            }
            _ => {}
        }
    }
    match c {
        led_core::theme::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
        led_core::theme::Color::Ansi(i) => Color::AnsiValue(i),
    }
}

pub fn render_base_dialog(renderer: &mut Renderer, theme: &led_core::theme::Theme, title: &str, x: u16, y: u16, w: u16, h: u16) {
    let bg = to_ct_color(theme.ui.dialog_bg, theme);
    let border_fg = to_ct_color(theme.ui.dialog_border, theme);

    // Fill background
    for dy in 0..h {
        for dx in 0..w {
            renderer.set_cell(x + dx, y + dy, Cell {
                ch: ' ',
                bg,
                ..Default::default()
            });
        }
    }

    // Draw borders (Unicode box-drawing)
    renderer.set_cell(x, y, Cell { ch: '┌', fg: border_fg, bg, ..Default::default() });
    renderer.set_cell(x + w - 1, y, Cell { ch: '┐', fg: border_fg, bg, ..Default::default() });
    renderer.set_cell(x, y + h - 1, Cell { ch: '└', fg: border_fg, bg, ..Default::default() });
    renderer.set_cell(x + w - 1, y + h - 1, Cell { ch: '┘', fg: border_fg, bg, ..Default::default() });

    for dx in 1..w - 1 {
        renderer.set_cell(x + dx, y, Cell { ch: '─', fg: border_fg, bg, ..Default::default() });
        renderer.set_cell(x + dx, y + h - 1, Cell { ch: '─', fg: border_fg, bg, ..Default::default() });
    }

    for dy in 1..h - 1 {
        renderer.set_cell(x, y + dy, Cell { ch: '│', fg: border_fg, bg, ..Default::default() });
        renderer.set_cell(x + w - 1, y + dy, Cell { ch: '│', fg: border_fg, bg, ..Default::default() });
    }

    let title_str = format!(" {} ", title);
    for (i, c) in title_str.chars().enumerate() {
        if 2 + (i as u16) < w - 2 {
            renderer.set_cell(x + 2 + (i as u16), y, Cell { ch: c, fg: border_fg, bg, ..Default::default() });
        }
    }
}

pub struct OpenDialog {
    pub browser: FileBrowser,
    pub error_message: Option<String>,
    pub i18n_title: String,
}

impl OpenDialog {
    pub fn new(i18n: &led_core::I18n) -> Self {
        let mut browser = FileBrowser::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
        browser.localize(i18n);
        Self {
            browser,
            error_message: None,
            i18n_title: i18n.get("dialog.open_file").to_string(),
        }
    }
}

impl Dialog for OpenDialog {
    fn title(&self) -> &str {
        &self.i18n_title
    }

    fn dimensions(&self) -> (u16, u16) {
        (80, 22)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);
        self.browser.render(renderer, theme, x, y, w, h);

        if let Some(ref msg) = self.error_message {
            let msg_x = x + 2;
            let msg_y = y + h - 3;
            for (i, c) in msg.chars().enumerate() {
                if (i as u16) < w - 4 {
                    renderer.set_cell(msg_x + i as u16, msg_y, Cell { ch: c, bg: to_ct_color(theme.ui.dialog_bg, theme), fg: Color::Red, ..Default::default() });
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        if let Some(path) = self.browser.handle_key(key) {
            return DialogResult::Ok(Action::ConfirmPath(path));
        }
        match key.code {
            KeyCode::Esc => DialogResult::Cancel,
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action> {
        if let Some(path) = self.browser.handle_mouse(mouse, x, y, w, h) {
            return DialogResult::Ok(Action::ConfirmPath(path));
        }
        DialogResult::Pending
    }

    fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    fn cursor_pos(&self) -> Option<(u16, u16)> {
        if self.browser.input_focused {
            let label_len = self.browser.i18n_filename.chars().count() as u16;
            Some((2 + label_len + self.browser.input_text.chars().count() as u16, 22 - 2))
        } else {
            None
        }
    }
}

pub struct SaveAsDialog {
    pub browser: FileBrowser,
    pub error_message: Option<String>,
    pub i18n_title: String,
}

impl SaveAsDialog {
    pub fn new(current_path: Option<&PathBuf>, i18n: &led_core::I18n) -> Self {
        let mut browser = FileBrowser::new(
            current_path.and_then(|p| p.parent()).map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))));
        
        if let Some(path) = current_path {
            if let Some(name) = path.file_name() {
                browser.input_text = name.to_string_lossy().to_string();
            }
        }
        
        browser.input_focused = true;
        browser.localize(i18n);
        
        Self { 
            browser,
            error_message: None,
            i18n_title: i18n.get("dialog.save_as").to_string(),
        }
    }
}

impl Dialog for SaveAsDialog {
    fn title(&self) -> &str {
        &self.i18n_title
    }

    fn dimensions(&self) -> (u16, u16) {
        (80, 22)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);
        self.browser.render(renderer, theme, x, y, w, h);

        if let Some(ref msg) = self.error_message {
            let msg_x = x + 2;
            let msg_y = y + h - 3;
            for (i, c) in msg.chars().enumerate() {
                if (i as u16) < w - 4 {
                    renderer.set_cell(msg_x + i as u16, msg_y, Cell { ch: c, bg: to_ct_color(theme.ui.dialog_bg, theme), fg: Color::Red, ..Default::default() });
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        if let Some(path) = self.browser.handle_key(key) {
            return DialogResult::Ok(Action::ConfirmPath(path));
        }
        match key.code {
            KeyCode::Esc => DialogResult::Cancel,
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action> {
        if let Some(path) = self.browser.handle_mouse(mouse, x, y, w, h) {
            return DialogResult::Ok(Action::ConfirmPath(path));
        }
        DialogResult::Pending
    }

    fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    fn cursor_pos(&self) -> Option<(u16, u16)> {
        if self.browser.input_focused {
            let label_len = self.browser.i18n_filename.chars().count() as u16;
            Some((2 + label_len + self.browser.input_text.chars().count() as u16, 22 - 2))
        } else {
            None
        }
    }
}

pub struct MessageDialog {
    pub title: String,
    pub message: String,
    pub buttons: Vec<(String, Action)>,
    pub selected_btn: usize,
}

impl MessageDialog {
    pub fn new(title: String, message: String, buttons: Vec<(String, Action)>) -> Self {
        Self {
            title,
            message,
            buttons,
            selected_btn: 0,
        }
    }
}

impl Dialog for MessageDialog {
    fn title(&self) -> &str {
        &self.title
    }

    fn dimensions(&self) -> (u16, u16) {
        (40, 8)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);

        let dialog_bg = to_ct_color(theme.ui.dialog_bg, theme);
        let dialog_fg = to_ct_color(theme.ui.panel_fg, theme);

        use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};
        let msg_w = self.message.width() as u16;
        let lx = x + (w.saturating_sub(msg_w)) / 2;
        let ly = y + 2;
        let mut cur_lx = lx;
        for c in self.message.chars() {
            let cw = c.width().unwrap_or(0) as u16;
            if cur_lx + cw <= x + w - 1 {
                renderer.set_cell(cur_lx, ly, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
            }
            cur_lx += cw;
        }

        // Buttons
        let total_btns_width: u16 = self.buttons.iter().map(|(s, _)| s.len() as u16 + 4).sum::<u16>() + (self.buttons.len() as u16 - 1);
        let mut bx = x + (w.saturating_sub(total_btns_width)) / 2;
        let by = y + h - 2;

        for (i, (s, _)) in self.buttons.iter().enumerate() {
            let btn_text = format!("[ {} ]", s);
            let is_selected = i == self.selected_btn;
            let bg = if is_selected { to_ct_color(theme.ui.button_active_bg, theme) } else { to_ct_color(theme.ui.panel_bg, theme) };
            let fg = if is_selected { to_ct_color(theme.ui.button_active_fg, theme) } else { to_ct_color(theme.ui.panel_fg, theme) };

            for c in btn_text.chars() {
                renderer.set_cell(bx, by, Cell { ch: c, bg, fg, ..Default::default() });
                bx += 1;
            }
            bx += 1; // Spacer
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                if self.selected_btn > 0 {
                    self.selected_btn -= 1;
                } else {
                    self.selected_btn = self.buttons.len().saturating_sub(1);
                }
                DialogResult::Pending
            }
            KeyCode::Right | KeyCode::Tab => {
                if self.selected_btn + 1 < self.buttons.len() {
                    self.selected_btn += 1;
                } else {
                    self.selected_btn = 0;
                }
                DialogResult::Pending
            }
            KeyCode::Enter => DialogResult::Ok(self.buttons[self.selected_btn].1.clone()),
            KeyCode::Esc => DialogResult::Cancel,
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action> {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return DialogResult::Pending;
        }

        let (mx, my) = (mouse.column, mouse.row);
        if my == y + h - 2 {
            let total_btns_width: u16 = self.buttons.iter().map(|(s, _)| s.len() as u16 + 4).sum::<u16>() + (self.buttons.len() as u16 - 1);
            let mut bx = x + (w.saturating_sub(total_btns_width)) / 2;
            for (i, (s, _)) in self.buttons.iter().enumerate() {
                let btn_len = s.len() as u16 + 4;
                if mx >= bx && mx < bx + btn_len {
                    return DialogResult::Ok(self.buttons[i].1.clone());
                }
                bx += btn_len + 1;
            }
        }
        DialogResult::Pending
    }
}

pub struct AboutDialog {
    pub i18n_about: String,
    pub i18n_ok: String,
    pub i18n_version: String,
    pub i18n_license: String,
}

impl AboutDialog {
    pub fn new(i18n: &led_core::I18n) -> Self {
        Self {
            i18n_about: i18n.get("dialog.about").to_string(),
            i18n_ok: i18n.get("dialog.ok").to_string(),
            i18n_version: i18n.get("about.version").to_string(),
            i18n_license: i18n.get("about.license").to_string(),
        }
    }
}

impl Dialog for AboutDialog {
    fn title(&self) -> &str {
        &self.i18n_about
    }

    fn dimensions(&self) -> (u16, u16) {
        (40, 10)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);

        let dialog_bg = to_ct_color(theme.ui.dialog_bg, theme);
        let dialog_fg = to_ct_color(theme.ui.panel_fg, theme);

        let content = [
            format!("led editor v0.1.0"),
            format!("{}: v0.1.0", self.i18n_version),
            "A lightweight, modern TUI editor.".to_string(),
            "".to_string(),
            format!("{}: MIT", self.i18n_license),
        ];

        for (i, line) in content.iter().enumerate() {
            use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};
            let line_w = line.width() as u16;
            let lx = x + (w.saturating_sub(line_w)) / 2;
            let ly = y + 2 + i as u16;
            let mut cur_lx = lx;
            for c in line.chars() {
                let cw = c.width().unwrap_or(0) as u16;
                if cur_lx + cw <= x + w - 1 {
                    renderer.set_cell(cur_lx, ly, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
                }
                cur_lx += cw;
            }
        }

        let btn_text = format!("[ {} ]", self.i18n_ok);
        let bx = x + (w.saturating_sub(btn_text.chars().count() as u16)) / 2;
        let by = y + h - 2;
        for (i, c) in btn_text.chars().enumerate() {
            renderer.set_cell(bx + i as u16, by, Cell {
                ch: c,
                bg: to_ct_color(theme.ui.button_active_bg, theme),
                fg: to_ct_color(theme.ui.button_active_fg, theme),
                ..Default::default()
            });
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => DialogResult::Ok(Action::Confirm),
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action> {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return DialogResult::Pending;
        }
        let (mx, my) = (mouse.column, mouse.row);
        let btn_text = format!("[ {} ]", self.i18n_ok);
        let bx = x + (w.saturating_sub(btn_text.len() as u16)) / 2;
        let by = y + h - 2;
        if my == by && mx >= bx && mx < bx + btn_text.len() as u16 {
            return DialogResult::Ok(Action::Confirm);
        }
        DialogResult::Pending
    }
}

pub struct ReopenConfirmationDialog {
    pub i18n_title: String,
    pub i18n_message: String,
    pub i18n_discard: String,
    pub i18n_cancel: String,
    pub selected_btn: usize,
}

impl ReopenConfirmationDialog {
    pub fn new(i18n: &led_core::I18n) -> Self {
        Self {
            i18n_title: i18n.get("dialog.reopen_file").to_string(),
            i18n_message: i18n.get("dialog.discard_reopen_prompt").to_string(),
            i18n_discard: i18n.get("dialog.discard_reopen").to_string(),
            i18n_cancel: i18n.get("dialog.cancel").to_string(),
            selected_btn: 0,
        }
    }
}

impl Dialog for ReopenConfirmationDialog {
    fn title(&self) -> &str {
        &self.i18n_title
    }

    fn dimensions(&self) -> (u16, u16) {
        (40, 8)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);

        let dialog_bg = to_ct_color(theme.ui.dialog_bg, theme);
        let dialog_fg = to_ct_color(theme.ui.panel_fg, theme);

        use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};
        let msg_w = self.i18n_message.width() as u16;
        let lx = x + (w.saturating_sub(msg_w)) / 2;
        let ly = y + 2;
        let mut cur_lx = lx;
        for c in self.i18n_message.chars() {
            let cw = c.width().unwrap_or(0) as u16;
            if cur_lx + cw <= x + w - 1 {
                renderer.set_cell(cur_lx, ly, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
            }
            cur_lx += cw;
        }

        let buttons = [&self.i18n_discard, &self.i18n_cancel];
        let total_btns_width: u16 = buttons.iter().map(|s| s.len() as u16 + 4).sum::<u16>() + 1;
        let mut bx = x + (w.saturating_sub(total_btns_width)) / 2;
        let by = y + h - 2;

        for (i, s) in buttons.iter().enumerate() {
            let btn_text = format!("[ {} ]", s);
            let is_selected = i == self.selected_btn;
            let bg = if is_selected { to_ct_color(theme.ui.button_active_bg, theme) } else { to_ct_color(theme.ui.panel_bg, theme) };
            let fg = if is_selected { to_ct_color(theme.ui.button_active_fg, theme) } else { to_ct_color(theme.ui.panel_fg, theme) };

            for c in btn_text.chars() {
                renderer.set_cell(bx, by, Cell { ch: c, bg, fg, ..Default::default() });
                bx += 1;
            }
            bx += 1;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        match key.code {
            KeyCode::Left | KeyCode::BackTab | KeyCode::Right | KeyCode::Tab => {
                self.selected_btn = 1 - self.selected_btn;
                DialogResult::Pending
            }
            KeyCode::Enter => {
                if self.selected_btn == 0 {
                    DialogResult::Ok(Action::Discard)
                } else {
                    DialogResult::Cancel
                }
            }
            KeyCode::Esc => DialogResult::Cancel,
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, x: u16, y: u16, w: u16, h: u16) -> DialogResult<Action> {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return DialogResult::Pending;
        }
        let (mx, my) = (mouse.column, mouse.row);
        if my == y + h - 2 {
            let buttons = [&self.i18n_discard, &self.i18n_cancel];
            let total_btns_width: u16 = buttons.iter().map(|s| s.len() as u16 + 4).sum::<u16>() + 1;
            let mut bx = x + (w.saturating_sub(total_btns_width)) / 2;
            for (i, s) in buttons.iter().enumerate() {
                let btn_len = s.len() as u16 + 4;
                if mx >= bx && mx < bx + btn_len {
                    if i == 0 { return DialogResult::Ok(Action::Discard); }
                    else { return DialogResult::Cancel; }
                }
                bx += btn_len + 1;
            }
        }
        DialogResult::Pending
    }
}

pub struct GoToLineDialog {
    pub input_text: String,
    pub i18n_title: String,
    pub i18n_goto: String,
}

impl GoToLineDialog {
    pub fn new(i18n: &led_core::I18n) -> Self {
        Self {
            input_text: String::new(),
            i18n_title: i18n.get("dialog.go_to_line").to_string(),
            i18n_goto: i18n.get("dialog.go_to_line").to_string(),
        }
    }
}

impl Dialog for GoToLineDialog {
    fn title(&self) -> &str {
        &self.i18n_title
    }

    fn dimensions(&self) -> (u16, u16) {
        (30, 6)
    }

    fn render(&self, renderer: &mut Renderer, theme: &led_core::theme::Theme, x: u16, y: u16, w: u16, h: u16) {
        render_base_dialog(renderer, theme, self.title(), x, y, w, h);

        let dialog_bg = to_ct_color(theme.ui.dialog_bg, theme);
        let dialog_fg = to_ct_color(theme.ui.panel_fg, theme);
        let input_bg = to_ct_color(theme.ui.panel_bg, theme);
        let input_fg = to_ct_color(theme.ui.panel_fg, theme);

        let label = format!("{}: ", self.i18n_goto);
        let lx = x + 2;
        let ly = y + 2;
        for (i, c) in label.chars().enumerate() {
            renderer.set_cell(lx + i as u16, ly, Cell { ch: c, bg: dialog_bg, fg: dialog_fg, ..Default::default() });
        }

        let input_x = lx + label.chars().count() as u16;
        let input_w = w.saturating_sub(label.chars().count() as u16 + 4);
        for dx in 0..input_w {
            renderer.set_cell(input_x + dx, ly, Cell { ch: ' ', bg: input_bg, ..Default::default() });
        }
        for (i, c) in self.input_text.chars().enumerate() {
            if (i as u16) < input_w {
                renderer.set_cell(input_x + i as u16, ly, Cell { ch: c, bg: input_bg, fg: input_fg, ..Default::default() });
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> DialogResult<Action> {
        match key.code {
            KeyCode::Char(c) if c.is_digit(10) => {
                self.input_text.push(c);
                DialogResult::Pending
            }
            KeyCode::Backspace => {
                self.input_text.pop();
                DialogResult::Pending
            }
            KeyCode::Enter => {
                if let Ok(line) = self.input_text.parse::<usize>() {
                    DialogResult::Ok(Action::ConfirmLine(line))
                } else {
                    DialogResult::Cancel
                }
            }
            KeyCode::Esc => DialogResult::Cancel,
            _ => DialogResult::Pending,
        }
    }

    fn handle_mouse(&mut self, _mouse: MouseEvent, _x: u16, _y: u16, _w: u16, _h: u16) -> DialogResult<Action> {
        DialogResult::Pending
    }

    fn cursor_pos(&self) -> Option<(u16, u16)> {
        let label_len = format!("{}: ", self.i18n_goto).chars().count() as u16;
        Some((2 + label_len + self.input_text.chars().count() as u16, 2))
    }
}

