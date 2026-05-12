pub mod config;
pub mod i18n;
pub mod buffer;
pub mod search;
pub mod syntax;
pub mod theme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // File
    New,
    Open,
    Save,
    SaveAs,
    Close,
    Exit,

    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Find,
    Replace,
    SelectAll,

    // View
    GoToLine,
    ToggleLineNumbers,
    ToggleWordWrap,
    ToggleViMode,
    ReopenWithEncoding(Encoding),
    ConvertToEncoding(Encoding),
    SetLineEnding(LineEnding),
    SetTheme(String),
    SetSyntax(String),

    // Help
    About,

    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    ShiftJis,
    EucJp,
    Iso2022Jp,
    Latin1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
    Cr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViMode {
    Normal,
    Insert,
    Visual,
}

pub use config::Config;
pub use i18n::I18n;
