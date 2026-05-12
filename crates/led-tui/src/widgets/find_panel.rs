use led_core::search::SearchFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelField {
    FindInput,
    ReplaceInput,
    MatchCase,
    WholeWord,
    Regex,
    Prev,
    Next,
    Close,
    ReplaceBtn,
    ReplaceAllBtn,
}

pub struct FindPanel {
    pub find_text: String,
    pub replace_text: String,
    pub is_replace_mode: bool,
    pub flags: SearchFlags,
    pub focused_field: PanelField,
}

impl FindPanel {
    pub fn new() -> Self {
        Self {
            find_text: String::new(),
            replace_text: String::new(),
            is_replace_mode: false,
            flags: SearchFlags::default(),
            focused_field: PanelField::FindInput,
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            PanelField::FindInput => {
                if self.is_replace_mode { PanelField::ReplaceInput } else { PanelField::MatchCase }
            }
            PanelField::ReplaceInput => PanelField::MatchCase,
            PanelField::MatchCase => PanelField::WholeWord,
            PanelField::WholeWord => PanelField::Regex,
            PanelField::Regex => PanelField::Prev,
            PanelField::Prev => PanelField::Next,
            PanelField::Next => {
                if self.is_replace_mode { PanelField::ReplaceBtn } else { PanelField::Close }
            }
            PanelField::ReplaceBtn => PanelField::ReplaceAllBtn,
            PanelField::ReplaceAllBtn => PanelField::Close,
            PanelField::Close => PanelField::FindInput,
        };
    }

    pub fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            PanelField::FindInput => PanelField::Close,
            PanelField::ReplaceInput => PanelField::FindInput,
            PanelField::MatchCase => {
                if self.is_replace_mode { PanelField::ReplaceInput } else { PanelField::FindInput }
            }
            PanelField::WholeWord => PanelField::MatchCase,
            PanelField::Regex => PanelField::WholeWord,
            PanelField::Prev => PanelField::Regex,
            PanelField::Next => PanelField::Prev,
            PanelField::Close => {
                if self.is_replace_mode { PanelField::ReplaceAllBtn } else { PanelField::Next }
            }
            PanelField::ReplaceBtn => PanelField::Next,
            PanelField::ReplaceAllBtn => PanelField::ReplaceBtn,
        };
    }
}
