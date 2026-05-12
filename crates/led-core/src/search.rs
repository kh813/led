use std::ops::Range;
use regex::RegexBuilder;
use crate::buffer::Editor;

#[derive(Debug, Clone, Default)]
pub struct SearchFlags {
    pub match_case: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub pattern: String,
    pub flags: SearchFlags,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub char_range: Range<usize>,
}

impl Editor {
    pub fn search(&self, query: &SearchQuery) -> Vec<Match> {
        if query.pattern.is_empty() {
            return Vec::new();
        }

        let pattern = if query.flags.use_regex {
            query.pattern.clone()
        } else {
            regex::escape(&query.pattern)
        };

        let pattern = if query.flags.whole_word {
            format!(r"\b{}\b", pattern)
        } else {
            pattern
        };

        let mut builder = RegexBuilder::new(&pattern);
        builder.case_insensitive(!query.flags.match_case);
        builder.multi_line(true);

        let re = match builder.build() {
            Ok(re) => re,
            Err(_) => return Vec::new(), // Invalid regex
        };

        let mut matches = Vec::new();
        let text = self.rope.to_string(); // TODO: Optimize this to avoid full string conversion

        for m in re.find_iter(&text) {
            let start_byte = m.start();
            let end_byte = m.end();
            
            let start_char = self.rope.byte_to_char(start_byte);
            let end_char = self.rope.byte_to_char(end_byte);
            
            matches.push(Match {
                char_range: start_char..end_char,
            });
        }

        matches
    }
}
