use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::ops::Range;
use anyhow::Result;
use ropey::Rope;
use crate::{Encoding, LineEnding};
use encoding_rs::*;

#[derive(Debug, Clone)]
pub struct EditDelta {
    pub char_range: Range<usize>,
    pub old_text: String,
    pub new_text: String,
    pub line_range: Range<usize>,
}

pub struct Editor {
    pub rope: Rope,
    pub path: Option<PathBuf>,
    pub encoding: Encoding,
    pub line_ending: LineEnding,
    pub read_only: bool,
    pub vi_mode: crate::ViMode,
    
    pub undo_stack: Vec<EditDelta>,
    pub redo_stack: Vec<EditDelta>,
    pub saved_undo_len: usize,
    pub modified_since_save: bool,

    pub cursor: usize, // char index
    pub selection_anchor: Option<usize>,
    pub selection: Option<Range<usize>>, // char index range
    pub scroll_row: usize,
    pub scroll_vrow: usize, // Visual row offset within the logical line
    pub scroll_col: usize,

    pub find_results: Vec<crate::search::Match>,
    pub current_match_idx: Option<usize>,
    pub search_status: Option<String>,

    pub syntax_highlighter: Option<crate::syntax::SyntaxHighlighter>,
    pub line_states: Vec<crate::syntax::LineState>,
    pub line_tokens: Vec<Option<Vec<crate::syntax::TokenSpan>>>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            path: None,
            encoding: Encoding::Utf8,
            line_ending: LineEnding::Lf,
            read_only: false,
            vi_mode: crate::ViMode::Normal,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_undo_len: 0,
            modified_since_save: false,
            cursor: 0,
            selection_anchor: None,
            selection: None,
            scroll_row: 0,
            scroll_vrow: 0,
            scroll_col: 0,
            find_results: Vec::new(),
            current_match_idx: None,
            search_status: None,
            syntax_highlighter: None,
            line_states: vec![crate::syntax::LineState::Normal],
            line_tokens: vec![None],
        }
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let (encoding, content) = Self::decode_bytes(&bytes);

        let line_ending = if content.contains("\r\n") {
            LineEnding::Crlf
        } else if content.contains('\r') {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        };

        let line_count = content.lines().count().max(1);
        Ok(Self {
            rope: Rope::from_str(&content),
            path: Some(path),
            encoding,
            line_ending,
            read_only: false,
            vi_mode: crate::ViMode::Normal,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_undo_len: 0,
            modified_since_save: false,
            cursor: 0,
            selection_anchor: None,
            selection: None,
            scroll_row: 0,
            scroll_vrow: 0,
            scroll_col: 0,
            find_results: Vec::new(),
            current_match_idx: None,
            search_status: None,
            syntax_highlighter: None,
            line_states: vec![crate::syntax::LineState::Normal; line_count],
            line_tokens: vec![None; line_count],
        })
    }

    pub fn update_syntax(&mut self, highlighter: Option<crate::syntax::SyntaxHighlighter>) {
        self.syntax_highlighter = highlighter;
        self.line_tokens.fill(None);
        self.update_line_states(0, self.rope.len_lines());
    }

    pub fn highlight_line(&mut self, line_idx: usize) -> Vec<crate::syntax::TokenSpan> {
        if line_idx >= self.line_count() {
            return vec![];
        }
        if let Some(tokens) = self.line_tokens.get(line_idx).and_then(|t| t.as_ref()) {
            return tokens.clone();
        }

        if let Some(ref highlighter) = self.syntax_highlighter {
            let state = self.line_states[line_idx];
            let text = self.rope.line(line_idx).to_string();
            let tokens = highlighter.highlight_line(&text, state);
            if line_idx < self.line_tokens.len() {
                self.line_tokens[line_idx] = Some(tokens.clone());
            }
            tokens
        } else {
            vec![]
        }
    }

    fn update_line_states(&mut self, from_line: usize, dirty_to_line: usize) {
        let line_count = self.rope.len_lines();
        if self.line_states.len() != line_count {
            self.line_states.resize(line_count, crate::syntax::LineState::Normal);
        }
        if self.line_tokens.len() != line_count {
            self.line_tokens.resize(line_count, None);
        }

        let mut affected_range = from_line..dirty_to_line;

        if let Some(ref highlighter) = self.syntax_highlighter {
            let mut current_state = if from_line == 0 {
                crate::syntax::LineState::Normal
            } else {
                // If we're starting from a middle line, we use the state that was at the START of that line.
                // But wait, if the line before it changed its NEXT state, we should have started from there.
                self.line_states[from_line]
            };

            for i in from_line..line_count {
                let old_state = self.line_states[i];
                self.line_states[i] = current_state;

                let text = self.rope.line(i).to_string();
                let next_state = highlighter.next_line_state(&text, current_state);
                
                if i >= dirty_to_line && i + 1 < line_count && next_state == self.line_states[i+1] && current_state == old_state {
                    // State stabilized
                    affected_range.end = i + 1;
                    break;
                }

                current_state = next_state;
                if i == line_count - 1 {
                    affected_range.end = line_count;
                }
            }

            // Parallel highlighting for the affected range using rayon
            use rayon::prelude::*;
            let highlighter_ref = highlighter;
            let lines: Vec<String> = (affected_range.start..affected_range.end)
                .map(|i| self.rope.line(i).to_string())
                .collect();
            let states: Vec<crate::syntax::LineState> = (affected_range.start..affected_range.end)
                .map(|i| self.line_states[i])
                .collect();

            let new_tokens: Vec<Vec<crate::syntax::TokenSpan>> = lines.par_iter().zip(states.par_iter())
                .map(|(line, state)| highlighter_ref.highlight_line(line, *state))
                .collect();

            for (i, tokens) in (affected_range.start..affected_range.end).zip(new_tokens) {
                if i < self.line_tokens.len() {
                    self.line_tokens[i] = Some(tokens);
                }
            }
        }
    }

    pub fn wrap_line(&self, line_idx: usize, width: usize, tab_size: usize) -> Vec<Range<usize>> {
        if width == 0 { return vec![0..self.line(line_idx).len_chars()]; }
        let line = self.line(line_idx);
        let mut result = Vec::new();
        let mut start = 0;
        let mut cur_width = 0;

        use unicode_width::UnicodeWidthChar;

        for (i, c) in line.chars().enumerate() {
            let char_w = if c == '\t' {
                tab_size - (cur_width % tab_size)
            } else if c == '\n' || c == '\r' {
                0
            } else {
                c.width().unwrap_or(0)
            };

            if cur_width + char_w > width && i > start {
                result.push(start..i);
                start = i;
                cur_width = 0;
            }
            
            cur_width += char_w;
        }
        
        result.push(start..line.len_chars());
        result
    }

    pub fn get_visual_col(&self, line_idx: usize, char_offset: usize, range: &Range<usize>, tab_size: usize) -> usize {
        let line = self.rope.line(line_idx);
        let mut visual_x = 0;
        use unicode_width::UnicodeWidthChar;

        for (i, c) in line.chars().enumerate() {
            if i < range.start { continue; }
            if i >= char_offset { break; }
            if c == '\t' {
                visual_x += tab_size - (visual_x % tab_size);
            } else {
                visual_x += c.width().unwrap_or(0);
            }
        }
        visual_x
    }

    pub fn get_char_at_vcol(&self, line_idx: usize, range: Range<usize>, target_vcol: usize, tab_size: usize) -> usize {
        let line = self.rope.line(line_idx);
        let mut visual_x = 0;
        let mut char_idx = self.rope.line_to_char(line_idx) + range.start;
        use unicode_width::UnicodeWidthChar;

        for (i, c) in line.chars().enumerate() {
            if i < range.start { continue; }
            if i >= range.end { break; }
            
            let char_w = if c == '\t' {
                tab_size - (visual_x % tab_size)
            } else {
                c.width().unwrap_or(0)
            };

            if visual_x + char_w > target_vcol {
                return char_idx;
            }

            visual_x += char_w;
            char_idx += 1;
            
            if c == '\n' || c == '\r' {
                return char_idx.saturating_sub(1);
            }
        }
        char_idx.saturating_sub(if range.end > range.start && self.is_line_ending(line.char(range.end - 1)) { 1 } else { 0 })
    }

    fn decode_bytes(bytes: &[u8]) -> (Encoding, String) {
        // Use encoding_rs for BOM detection
        if let Some((enc, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
            let (content, _, _) = enc.decode(&bytes[bom_len..]);
            let encoding = if enc == UTF_8 {
                Encoding::Utf8Bom
            } else if enc == UTF_16LE {
                Encoding::Utf16Le
            } else if enc == UTF_16BE {
                Encoding::Utf16Be
            } else {
                Encoding::Utf8 // Should not happen with for_bom
            };
            return (encoding, content.into_owned());
        }

        // Try UTF-8 first
        let (res, _enc, malformed) = UTF_8.decode(bytes);
        if !malformed {
            return (Encoding::Utf8, res.into_owned());
        }

        // Japanese encodings are common, try them
        for enc in &[SHIFT_JIS, EUC_JP, ISO_2022_JP] {
            let (res, _, malformed) = enc.decode(bytes);
            if !malformed {
                let encoding = if *enc == SHIFT_JIS {
                    Encoding::ShiftJis
                } else if *enc == EUC_JP {
                    Encoding::EucJp
                } else {
                    Encoding::Iso2022Jp
                };
                return (encoding, res.into_owned());
            }
        }

        // Fallback to Latin-1 (which never fails for any byte sequence)
        let (res, _, _) = WINDOWS_1252.decode(bytes);
        (Encoding::Latin1, res.into_owned())
    }

    pub fn line(&self, idx: usize) -> ropey::RopeSlice<'_> {
        self.rope.line(idx)
    }

    pub fn is_modified(&self) -> bool {
        self.modified_since_save || self.undo_stack.len() != self.saved_undo_len
    }

    pub fn char_to_line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.rope.len_chars());
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        (line, pos - line_start)
    }

    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        
        // Don't allow cursor after line ending unless it's the last line without one
        let mut max_col = line_len;
        if line < self.rope.len_lines() - 1 || (line_len > 0 && self.is_line_ending(self.rope.char(line_start + line_len - 1))) {
            max_col = self.get_line_max_col(line);
        }
        
        line_start + col.min(max_col)
    }

    fn is_line_ending(&self, c: char) -> bool {
        c == '\n' || c == '\r'
    }

    pub fn get_line_max_col(&self, line: usize) -> usize {
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        let mut max_col = line_len;
        if max_col > 0 {
            let last = self.rope.char(line_start + max_col - 1);
            if last == '\n' || last == '\r' {
                max_col -= 1;
                if max_col > 0 {
                    let last2 = self.rope.char(line_start + max_col - 1);
                    if last2 == '\r' {
                        max_col -= 1;
                    }
                }
            }
        }
        max_col
    }

    pub fn move_cursor_left(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        if self.cursor > 0 {
            self.cursor -= 1;
        }

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_cursor_right(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        if self.cursor < self.rope.len_chars() {
            self.cursor += 1;
        }

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_cursor_up(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        let (line, col) = self.char_to_line_col(self.cursor);
        if line > 0 {
            self.cursor = self.line_col_to_char(line - 1, col);
        }

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_cursor_down(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        let (line, col) = self.char_to_line_col(self.cursor);
        if line < self.rope.len_lines().saturating_sub(1) {
            self.cursor = self.line_col_to_char(line + 1, col);
        } else if line == self.rope.len_lines().saturating_sub(1) {
             // Already on last line, but might want to move to end of line if we're not there
             self.cursor = self.line_col_to_char(line, self.get_line_max_col(line));
        }

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_cursor_home(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        let line = self.rope.char_to_line(self.cursor);
        self.cursor = self.rope.line_to_char(line);

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_cursor_end(&mut self, extend_selection: bool) {
        if extend_selection {
            self.ensure_selection();
        } else {
            self.selection = None;
        }

        let line = self.rope.char_to_line(self.cursor);
        self.cursor = self.line_col_to_char(line, self.get_line_max_col(line));

        if extend_selection {
            self.update_selection();
        }
    }

    pub fn move_word_forward(&mut self, extend_selection: bool) {
        if extend_selection { self.ensure_selection(); } else { self.selection = None; }
        let mut pos = self.cursor;
        let len = self.rope.len_chars();
        if pos >= len { return; }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let start_char = self.rope.char(pos);
        
        if is_word_char(start_char) {
            while pos < len && is_word_char(self.rope.char(pos)) { pos += 1; }
        } else if !start_char.is_whitespace() {
            while pos < len && !is_word_char(self.rope.char(pos)) && !self.rope.char(pos).is_whitespace() { pos += 1; }
        }
        while pos < len && self.rope.char(pos).is_whitespace() { pos += 1; }
        
        self.cursor = pos;
        if extend_selection { self.update_selection(); }
    }

    pub fn move_word_backward(&mut self, extend_selection: bool) {
        if extend_selection { self.ensure_selection(); } else { self.selection = None; }
        let mut pos = self.cursor;
        if pos == 0 { return; }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        
        // Skip leading whitespace
        while pos > 0 && self.rope.char(pos - 1).is_whitespace() { pos -= 1; }
        if pos == 0 { self.cursor = 0; return; }

        let start_char = self.rope.char(pos - 1);
        if is_word_char(start_char) {
            while pos > 0 && is_word_char(self.rope.char(pos - 1)) { pos -= 1; }
        } else {
            while pos > 0 && !is_word_char(self.rope.char(pos - 1)) && !self.rope.char(pos - 1).is_whitespace() { pos -= 1; }
        }
        
        self.cursor = pos;
        if extend_selection { self.update_selection(); }
    }

    pub fn move_word_end(&mut self, extend_selection: bool) {
        if extend_selection { self.ensure_selection(); } else { self.selection = None; }
        let mut pos = self.cursor;
        let len = self.rope.len_chars();
        if pos >= len.saturating_sub(1) { return; }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        
        // Move to next char if we're at the start of current word
        pos += 1;
        while pos < len && self.rope.char(pos).is_whitespace() { pos += 1; }
        if pos >= len { self.cursor = len.saturating_sub(1); return; }

        let start_char = self.rope.char(pos);
        if is_word_char(start_char) {
            while pos < len - 1 && is_word_char(self.rope.char(pos + 1)) { pos += 1; }
        } else {
            while pos < len - 1 && !is_word_char(self.rope.char(pos + 1)) && !self.rope.char(pos + 1).is_whitespace() { pos += 1; }
        }
        
        self.cursor = pos;
        if extend_selection { self.update_selection(); }
    }

    pub fn ensure_selection(&mut self) {
        if self.selection.is_none() {
            self.selection_anchor = Some(self.cursor);
            self.selection = Some(self.cursor..self.cursor);
        }
    }

    pub fn update_selection(&mut self) {
        if let Some(anchor) = self.selection_anchor {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            self.selection = Some(start..end);
            if start == end {
                self.selection = None;
                self.selection_anchor = None;
            }
        }
    }

    pub fn select_word(&mut self, pos: usize) {
        let range = self.find_word_bounds(pos);
        self.selection_anchor = Some(range.start);
        self.selection = Some(range.clone());
        self.cursor = range.end;
    }

    pub fn select_line(&mut self, line: usize) {
        if line >= self.rope.len_lines() { return; }
        let start = self.rope.line_to_char(line);
        let end = if line < self.rope.len_lines() - 1 {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        self.selection_anchor = Some(start);
        self.selection = Some(start..end);
        self.cursor = end;
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.selection = Some(0..self.rope.len_chars());
        self.cursor = self.rope.len_chars();
    }

    pub fn find_word_bounds(&self, pos: usize) -> Range<usize> {
        if self.rope.len_chars() == 0 { return 0..0; }
        let pos = pos.min(self.rope.len_chars().saturating_sub(1));
        let mut start = pos;
        let mut end = pos;
        
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let initial_char = self.rope.char(pos);
        let target_is_word = is_word_char(initial_char);
        
        while start > 0 {
            if is_word_char(self.rope.char(start - 1)) != target_is_word {
                break;
            }
            start -= 1;
        }
        while end < self.rope.len_chars() {
            if is_word_char(self.rope.char(end)) != target_is_word {
                break;
            }
            end += 1;
        }
        start..end
    }

    pub fn insert(&mut self, pos: usize, text: &str) -> EditDelta {
        let old_line_start = self.rope.char_to_line(pos);
        let old_text = "".to_string();
        
        self.rope.insert(pos, text);
        self.redo_stack.clear();
        
        let new_char_count = text.chars().count();
        let new_line_end = self.rope.char_to_line(pos + new_char_count);
        
        self.update_line_states(old_line_start, new_line_end + 1);

        let delta = EditDelta {
            char_range: pos..pos,
            old_text,
            new_text: text.to_string(),
            line_range: old_line_start..new_line_end + 1,
        };
        
        self.push_undo(delta.clone());
        self.cursor = pos + new_char_count;
        self.selection = None;
        self.selection_anchor = None;
        delta
    }

    pub fn delete(&mut self, range: Range<usize>) -> EditDelta {
        let old_line_start = self.rope.char_to_line(range.start);
        let old_text = self.rope.slice(range.clone()).to_string();
        
        self.rope.remove(range.clone());
        self.redo_stack.clear();
        
        let new_line_count = self.rope.len_lines();
        let new_line_end = old_line_start.min(new_line_count.saturating_sub(1));
        
        self.update_line_states(old_line_start, new_line_end + 1);

        let delta = EditDelta {
            char_range: range.clone(),
            old_text,
            new_text: "".to_string(),
            line_range: old_line_start..old_line_start + 1, // Range after delete
        };
        
        self.push_undo(delta.clone());
        self.cursor = range.start;
        self.selection = None;
        self.selection_anchor = None;
        delta
    }

    fn push_undo(&mut self, delta: EditDelta) {
        self.undo_stack.push(delta);
        if self.undo_stack.len() > 1000 {
            self.undo_stack.remove(0);
            if self.saved_undo_len > 0 {
                self.saved_undo_len -= 1;
            } else {
                // If saved_undo_len was 0 and we dropped the oldest,
                // we can't get back to saved state.
                // For now just set it to a value that will never match.
                self.saved_undo_len = usize::MAX;
            }
        }
    }

    pub fn undo(&mut self) -> Option<EditDelta> {
        if let Some(delta) = self.undo_stack.pop() {
            let inverse = self.invert_delta(&delta);
            
            let pos = delta.char_range.start;
            let old_line_start = self.rope.char_to_line(pos);

            self.rope.remove(delta.char_range.start..(delta.char_range.start + delta.new_text.chars().count()));
            self.rope.insert(delta.char_range.start, &delta.old_text);
            
            let new_line_end = self.rope.char_to_line(pos + delta.old_text.chars().count());
            self.update_line_states(old_line_start, new_line_end + 1);

            self.redo_stack.push(inverse);
            self.cursor = delta.char_range.start;
            self.selection = None;
            self.selection_anchor = None;
            Some(delta)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<EditDelta> {
        if let Some(delta) = self.redo_stack.pop() {
            let inverse = self.invert_delta(&delta);
            
            let pos = delta.char_range.start;
            let old_line_start = self.rope.char_to_line(pos);

            self.rope.remove(delta.char_range.start..(delta.char_range.start + delta.new_text.chars().count()));
            self.rope.insert(delta.char_range.start, &delta.old_text);
            
            let new_line_end = self.rope.char_to_line(pos + delta.old_text.chars().count());
            self.update_line_states(old_line_start, new_line_end + 1);

            self.undo_stack.push(inverse);
            self.cursor = delta.char_range.start + delta.old_text.chars().count();
            self.selection = None;
            self.selection_anchor = None;
            Some(delta)
        } else {
            None
        }
    }

    fn invert_delta(&self, delta: &EditDelta) -> EditDelta {
        EditDelta {
            char_range: delta.char_range.start..(delta.char_range.start + delta.new_text.chars().count()),
            old_text: delta.new_text.clone(),
            new_text: delta.old_text.clone(),
            line_range: delta.line_range.clone(),
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = self.path.clone() {
            self.save_as(path)
        } else {
            anyhow::bail!("No path associated with buffer")
        }
    }

    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = self.rope.to_string();
        let content = match self.line_ending {
            LineEnding::Lf => content.replace("\r\n", "\n").replace('\r', "\n"),
            LineEnding::Crlf => content.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n"),
            LineEnding::Cr => content.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r"),
        };

        let encoder = match self.encoding {
            Encoding::Utf8 | Encoding::Utf8Bom => UTF_8,
            Encoding::Utf16Le => UTF_16LE,
            Encoding::Utf16Be => UTF_16BE,
            Encoding::ShiftJis => SHIFT_JIS,
            Encoding::EucJp => EUC_JP,
            Encoding::Iso2022Jp => ISO_2022_JP,
            Encoding::Latin1 => WINDOWS_1252,
        };

        let mut bytes = match self.encoding {
            Encoding::Utf8Bom => vec![0xEF, 0xBB, 0xBF],
            Encoding::Utf16Le => vec![0xFF, 0xFE],
            Encoding::Utf16Be => vec![0xFE, 0xFF],
            _ => vec![],
        };

        let (encoded_bytes, _, _malformed) = encoder.encode(&content);
        bytes.extend_from_slice(&encoded_bytes);

        let mut file = File::create(path.as_ref())?;
        file.write_all(&bytes)?;
        file.flush()?;

        self.path = Some(path.as_ref().to_path_buf());
        self.modified_since_save = false;
        self.saved_undo_len = self.undo_stack.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert_undo_redo() {
        let mut editor = Editor::new();
        assert_eq!(editor.rope.to_string(), "");
        assert!(!editor.is_modified());

        editor.insert(0, "Hello");
        assert_eq!(editor.rope.to_string(), "Hello");
        assert!(editor.is_modified());

        editor.undo();
        assert_eq!(editor.rope.to_string(), "");
        assert!(!editor.is_modified());

        editor.redo();
        assert_eq!(editor.rope.to_string(), "Hello");
        assert!(editor.is_modified());
    }

    #[test]
    fn test_editor_delete_undo_redo() {
        let mut editor = Editor::new();
        editor.insert(0, "Hello World");
        editor.saved_undo_len = editor.undo_stack.len();
        editor.modified_since_save = false;
        assert!(!editor.is_modified());

        editor.delete(5..11);
        assert_eq!(editor.rope.to_string(), "Hello");
        assert!(editor.is_modified());

        editor.undo();
        assert_eq!(editor.rope.to_string(), "Hello World");
        assert!(!editor.is_modified());

        editor.redo();
        assert_eq!(editor.rope.to_string(), "Hello");
        assert!(editor.is_modified());
    }

    #[test]
    fn test_undo_limit() {
        let mut editor = Editor::new();
        for i in 0..1005 {
            editor.insert(i, "a");
        }
        assert_eq!(editor.undo_stack.len(), 1000);
        assert!(editor.is_modified());
        
        // Undo all available
        for _ in 0..1000 {
            editor.undo();
        }
        assert_eq!(editor.undo_stack.len(), 0);
        // Should still be modified because we can't get back to original empty state
        assert!(editor.is_modified());
    }

    #[test]
    fn test_wrap_line() {
        let mut editor = Editor::new();
        editor.insert(0, "abcdefghij"); // 10 chars
        let wraps = editor.wrap_line(0, 4, 4); // width 4
        assert_eq!(wraps.len(), 3);
        assert_eq!(wraps[0], 0..4); // abcd
        assert_eq!(wraps[1], 4..8); // efgh
        assert_eq!(wraps[2], 8..10); // ij
    }

    #[test]
    fn test_visual_col_helpers() {
        let mut editor = Editor::new();
        editor.insert(0, "a\tbc"); // a (0), \t (1-3), b (4), c (5)
        let range = 0..4; // "a\tb"
        assert_eq!(editor.get_visual_col(0, 0, &range, 4), 0);
        assert_eq!(editor.get_visual_col(0, 1, &range, 4), 1);
        assert_eq!(editor.get_visual_col(0, 2, &range, 4), 4); // after tab
        
        assert_eq!(editor.get_char_at_vcol(0, 0..4, 0, 4), 0);
        assert_eq!(editor.get_char_at_vcol(0, 0..4, 1, 4), 1);
        assert_eq!(editor.get_char_at_vcol(0, 0..4, 2, 4), 1); // during tab
        assert_eq!(editor.get_char_at_vcol(0, 0..4, 4, 4), 2); // after tab
    }
}
