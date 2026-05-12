use std::io::{self, Write};
use base64::{Engine as _, engine::general_purpose};
use arboard::Clipboard;

pub fn set_clipboard(text: &str) -> io::Result<()> {
    // 1. OSC 52 (for remote environments like SSH)
    let b64 = general_purpose::STANDARD.encode(text);
    let osc52 = format!("\x1b]52;c;{}\x07", b64);
    io::stdout().write_all(osc52.as_bytes())?;
    io::stdout().flush()?;

    // 2. Platform clipboard (local)
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text.to_string());
    }
    
    Ok(())
}

pub fn get_clipboard() -> Option<String> {
    if let Ok(mut clipboard) = Clipboard::new() {
        clipboard.get_text().ok()
    } else {
        None
    }
}
