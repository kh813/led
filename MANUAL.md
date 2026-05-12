# led User Manual

**led** is a lightweight, modern TUI text editor for plain text, Markdown, and config files.  
It runs on macOS, Linux, and Windows — including over SSH.

---

## Table of Contents

1. [Installation](#1-installation)
2. [Basic Usage](#2-basic-usage)
3. [Keyboard Shortcuts](#3-keyboard-shortcuts)
4. [Configuration](#4-configuration)
5. [Theme File Format](#5-theme-file-format)
6. [Syntax Definition File Format](#6-syntax-definition-file-format)
7. [Internationalization (i18n)](#7-internationalization-i18n)
8. [SSH Usage Notes](#8-ssh-usage-notes)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Installation

### Pre-built binaries (recommended)

Download the appropriate binary from the [releases page] and place it in your `PATH`.

| Platform | Binary | Notes |
| :--- | :--- | :--- |
| macOS Apple Silicon | `led.mac-arm64` | TUI. Rename to `led` and `chmod +x`. |
| macOS Intel | `led.mac-x64` | TUI. Rename to `led` and `chmod +x`. |
| macOS (GUI) | `led.app` | Double-click or drag to `/Applications`. |
| Linux x86-64 | `led.linux-x64` | TUI. Rename to `led` and `chmod +x`. |
| Linux ARM64 | `led.linux-arm64` | TUI. Rename to `led` and `chmod +x`. |
| Windows x86-64 | `led.exe` | GUI (`windows-msvc`). Run directly or add to `PATH`. |

**macOS / Linux quick install:**
```bash
# Example: macOS Apple Silicon
curl -L https://github.com/yourname/led/releases/latest/download/led.mac-arm64 -o led
chmod +x led
sudo mv led /usr/local/bin/led
```

> **macOS Gatekeeper**: On first run, macOS may block the binary. Right-click → Open to allow, or run:
> ```bash
> xattr -d com.apple.quarantine /usr/local/bin/led
> ```

### Build from source

**Requirements**: Rust toolchain (`rustup`), Xcode Command Line Tools (macOS)

```bash
git clone https://github.com/yourname/led.git
cd led

# TUI only (current host)
make

# All targets from macOS (requires Docker for Linux cross-compilation)
# cargo install cross  ← install cross tool first if not already installed
make all

# Outputs land in dist/
ls dist/
```

**Available make targets:**

| Target | Description |
| :--- | :--- |
| `make` | Build `led` (TUI) for the current host into `dist/` |
| `make all` | Cross-build all targets (macOS only; Docker required for Linux) |
| `make gui` | Build `led.app` (GUI) for current macOS host |
| `make clean` | Remove `dist/` |
| `make help` | List all targets |

> **Windows builds**: `led.exe` uses the `x86_64-pc-windows-msvc` target and cannot be cross-compiled from macOS. It is built automatically on a GitHub Actions `windows-latest` runner, triggered by the same release tag as `make all`. The resulting `led.exe` is uploaded as a release asset alongside the macOS and Linux binaries.

---

## 2. Basic Usage

```bash
led                      # Open with empty buffer
led myfile.txt           # Open a file (creates empty named buffer if it does not exist)
led file1.txt file2.txt  # Open multiple files in tabs
```

### Command-Line Behavior

| Invocation | Result |
| :--- | :--- |
| `led` | Opens with a single empty `[No Name]` buffer |
| `led myfile.txt` | Opens `myfile.txt`; if it does not exist, creates an empty buffer pre-named `myfile.txt` (not written to disk until you save) |
| `led a.txt b.txt` | Opens both files in separate tabs; first tab is active |
| `led /some/dir` | Shows an error dialog (`"..." is a directory`) and opens an empty buffer |

### UI Layout

```
┌─────────────────────────────────────────────┐
│ File   Edit   View   Help                   │  ← Menu Bar
├─────────────────────────────────────────────┤
│ [+] main.rs × │ README.md ×                │  ← Tab Bar
├─────────────────────────────────────────────┤
│ Find:    [___________________________] ...  │  ← Find/Replace Panel (optional)
├─────────────────────────────────────────────┤
│  1 │ fn main() {                            │
│  2 │     println!("Hello");                 │  ← Editor Area
│  3 │ }                                      │
├─────────────────────────────────────────────┤
│ [+] main.rs        Ln 2, Col 5  UTF-8  LF  Rust │  ← Status Bar
└─────────────────────────────────────────────┘
```

- **Tab Bar**: `[+]` prefix means unsaved changes. Click tabs to switch; middle-click to close.
- **Status Bar**: shows cursor position, encoding, line ending, and active syntax.
- **Find/Replace Panel**: appears when `Ctrl+F` or `Ctrl+H` is pressed; pushes the editor area down (never overlaps text).

---

## 3. Keyboard Shortcuts

### Menu Bar

| Action | Shortcut |
| :--- | :--- |
| Open File menu | `Alt+F` |
| Open Edit menu | `Alt+E` |
| Open View menu | `Alt+V` |
| Open Help menu | `Alt+H` |
| Move between menus (when open) | `←` / `→` |
| Move within dropdown | `↑` / `↓` |
| Activate item | `Enter` |
| Close menu | `Esc` |

### File

| Action | Shortcut |
| :--- | :--- |
| New tab | `Ctrl+N` |
| Open… | `Ctrl+O` |
| Save | `Ctrl+S` |
| Save As… | `Ctrl+Shift+S` |
| Close tab | `Ctrl+W` |
| Exit | `Ctrl+Q` |

> **Note**: On macOS, use the Command key (`⌘`) instead of `Ctrl` for all shortcuts (e.g., `⌘S` for Save).

### Edit

| Action | Shortcut |
| :--- | :--- |
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Y` |
| Cut | `Ctrl+X` |
| Copy | `Ctrl+C` |
| Paste | `Ctrl+V` |
| Select All | `Ctrl+A` |
| Find | `Ctrl+F` |
| Find & Replace | `Ctrl+H` |

> **Paste behavior**: Line endings in pasted text are automatically normalized to match the current buffer's line ending setting (LF, CRLF, or CR).

### Navigation

| Action | Shortcut |
| :--- | :--- |
| Go to Line… | `Ctrl+G` |
| New tab | `Ctrl+T` |
| Next tab | `Ctrl+Tab` |
| Previous tab | `Ctrl+Shift+Tab` |
| Toggle Line Numbers | `View > [x] Line Numbers` |

### Find/Replace Panel

| Action | Shortcut |
| :--- | :--- |
| Next match (downward) | `Enter` / `F3` |
| Previous match (upward) | `Shift+Enter` / `Shift+F3` |
| Close panel | `Esc` |
| Cycle focus (inputs ↔ toggles) | `Tab` / `Shift+Tab` |

### Mouse

| Action | Behavior |
| :--- | :--- |
| Click | Move cursor |
| Click + drag | Select text |
| Double-click | Select word |
| Triple-click | Select line |
| Shift + click | Extend selection |
| Scroll wheel | Scroll vertically |
| Shift + scroll | Scroll horizontally (word wrap off only) |
| Click line number | Select entire line |
| Middle-click tab | Close tab |

---

## 4. Configuration

All configuration lives in `~/.config/led/config.toml`.

- If the file **exists**, **led** reads it at startup and applies any keys it finds over the built-in defaults.
- If the file **does not exist**, **led** runs on built-in defaults — the file is not created automatically on startup.
- Unknown keys are silently ignored. Missing keys fall back to their default values.

**Runtime config writes**: When you change a persistent setting at runtime (e.g., toggling Line Numbers or switching Theme via the View menu), **led** writes that change back to `~/.config/led/config.toml` automatically. If the file does not exist yet, it is created at that point with only the changed key(s).

To start customizing manually, copy the template shipped with **led** and edit it:

```bash
cp /path/to/led/assets/config.toml.default ~/.config/led/config.toml
```

### All Configuration Keys

```toml
# ~/.config/led/config.toml

# UI language (see Section 7 for available locales)
language = "en"

# Active theme (must match a filename in ~/.config/led/themes/ without .toml)
# Built-in themes: "tokyo-night", "light", "solarized-dark", "solarized-light",
#                  "catppuccin-mocha", "catppuccin-latte"
theme = "tokyo-night"

# Show line numbers in the left gutter (can be toggled at runtime via View menu)
line_numbers = true

# Enable vi keybindings (Normal / Insert / Visual modes)
vi_mode = false

# Wrap long lines in the editor area
word_wrap = false

# Tab stop width in display cells (1–16)
# Controls how wide a tab character (\t) appears on screen.
tab_size = 4

# Expand tab key press to spaces on input.
# true  = pressing the Tab key inserts spaces (tab_size spaces wide).
# false = pressing the Tab key inserts a literal \t character.
# Note: this affects what is inserted when you press Tab. The actual characters
# already in the file are always preserved as-is on disk regardless of this setting.
expand_tab = false
```

> **Note**: All config files are loaded at startup only. Changes to `config.toml` take effect after restarting `led`, except for settings changed via the View menu which are applied immediately.

### Directory Structure

```
~/.config/led/
├── config.toml               ← main config
├── themes/
│   ├── tokyo-night.toml      ← built-in
│   ├── light.toml            ← built-in
│   ├── solarized-dark.toml   ← built-in
│   ├── solarized-light.toml  ← built-in
│   ├── catppuccin-mocha.toml ← built-in
│   ├── catppuccin-latte.toml ← built-in
│   └── my-theme.toml         ← your custom theme
├── syntax/
│   ├── plain-text.toml  ← built-in
│   ├── markdown.toml    ← built-in
│   ├── rust.toml        ← built-in
│   ├── toml.toml        ← built-in
│   ├── python.toml      ← built-in
│   ├── go.toml          ← built-in
│   ├── swift.toml       ← built-in
│   ├── javascript.toml  ← built-in
│   ├── html.toml        ← built-in
│   ├── css.toml         ← built-in
│   ├── xml.toml         ← built-in
│   └── my-lang.toml     ← your custom syntax definition
└── locales/
    └── fr.toml          ← your custom locale (optional)
```

---

## 5. Theme File Format

Theme files live in `~/.config/led/themes/*.toml`.  
Colors are specified as 24-bit hex RGB strings (`"#rrggbb"`).

### Full Schema

```toml
# ~/.config/led/themes/my-theme.toml

[meta]
name        = "My Theme"          # Display name shown in View > Theme menu
author      = "Your Name"         # Optional
version     = "1.0"               # Optional

[editor]
background  = "#1a1b26"           # Editor area background
foreground  = "#c0caf5"           # Default text color
cursor      = "#c0caf5"           # Cursor color (block / beam)
selection   = "#283457"           # Selected text background
line_number = "#3b4261"           # Gutter line number color
current_line = "#1e2030"          # Background of the line the cursor is on (optional)

[ui]
menu_bar_bg         = "#16161e"   # Menu bar background
menu_bar_fg         = "#c0caf5"   # Menu bar text
menu_item_active_bg = "#7aa2f7"   # Highlighted menu item background
menu_item_active_fg = "#1a1b26"   # Highlighted menu item text
tab_bar_bg          = "#16161e"   # Tab bar background
tab_active_bg       = "#1a1b26"   # Active tab background
tab_active_fg       = "#c0caf5"   # Active tab text
tab_inactive_bg     = "#16161e"   # Inactive tab background
tab_inactive_fg     = "#565f89"   # Inactive tab text
status_bar_bg       = "#16161e"   # Status bar background
status_bar_fg       = "#c0caf5"   # Status bar text
panel_bg            = "#16161e"   # Find/Replace panel background
panel_fg            = "#c0caf5"   # Find/Replace panel text
panel_error_fg      = "#f7768e"   # Find input text color when no matches found
dialog_bg           = "#1e2030"   # Dialog background
dialog_border       = "#7aa2f7"   # Dialog border color
button_active_bg    = "#7aa2f7"   # Active/focused button background
button_active_fg    = "#1a1b26"   # Active/focused button text

[syntax]
# Token type colors. All keys are optional; omitted keys fall back to editor.foreground.
keyword     = "#bb9af7"
type_name   = "#2ac3de"
function    = "#7aa2f7"
string      = "#9ece6a"
number      = "#ff9e64"
comment     = "#565f89"
operator    = "#89ddff"
punctuation = "#c0caf5"
constant    = "#ff9e64"
attribute   = "#bb9af7"
error       = "#f7768e"
```

### Minimal Theme (only required keys)

```toml
[meta]
name = "Minimal Dark"

[editor]
background  = "#1c1c1c"
foreground  = "#d4d4d4"
cursor      = "#d4d4d4"
selection   = "#264f78"
line_number = "#5a5a5a"

[ui]
status_bar_bg = "#007acc"
status_bar_fg = "#ffffff"
```

All other keys default to reasonable fallback values derived from `editor.foreground` and `editor.background`.

---

## 6. Syntax Definition File Format

Syntax definition files live in `~/.config/led/syntax/*.toml`.  
They map **file extensions** to a set of **regex-based token rules**.

### Full Schema

```toml
# ~/.config/led/syntax/python.toml

[meta]
name        = "Python"
extensions  = ["py", "pyw"]

# Rules are evaluated in order; the first match wins.
# The regex engine is Rust's `regex` crate (RE2 syntax — no backtracking).

[[rule]]
token = "comment"
pattern = "#.*"

[[rule]]
token = "string"
start   = '"""'
end     = '"""'

[[rule]]
token = "string"
start   = "'''"
end     = "'''"

[[rule]]
token = "string"
pattern = '"(?:[^"\\]|\\.)*"'

[[rule]]
token = "string"
pattern = "'(?:[^'\\]|\\.)*'"

[[rule]]
token = "keyword"
pattern = "def|class|if|elif|else|for|while|return|import|from|as|with|in|not|and|or|is|None|True|False|lambda|yield|pass|break|continue|try|except|finally|raise|del|global|nonlocal|assert"
word_boundary = true

[[rule]]
token = "type_name"
pattern = '\b[A-Z][a-zA-Z0-9_]*\b'

[[rule]]
token = "number"
pattern = '\b(?:0x[0-9a-fA-F]+|0o[0-7]+|0b[01]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)\b'

[[rule]]
token = "function"
pattern = '(?<=def )[a-zA-Z_][a-zA-Z0-9_]*'

[[rule]]
token = "operator"
pattern = '[+\-*/%=<>!&|^~]+'

[[rule]]
token = "attribute"
pattern = '@[a-zA-Z_][a-zA-Z0-9_.]*'
```

### Token Types Reference

| Token | Suggested use |
| :--- | :--- |
| `keyword` | Reserved words (`fn`, `if`, `class`, …) |
| `type_name` | Type names, structs, classes |
| `function` | Function and method names |
| `string` | String and character literals |
| `number` | Numeric literals |
| `comment` | Line comments and block comments |
| `operator` | Operators and arrows |
| `punctuation` | Brackets, commas, semicolons |
| `constant` | Constants, enum variants, `true`/`false` |
| `attribute` | Decorators, annotations, attributes |
| `error` | Syntax error markers |

### Rule Types Reference

| Key | Behavior |
| :--- | :--- |
| `pattern` | Single regex match; highlights the entire match |
| `start` + `end` | Region match; highlights from `start` to `end` (inclusive), spanning multiple lines |
| `word_boundary = true` | Wraps `pattern` in `\b...\b` automatically (default: `false`) |
| `escape = "..."` | Region rules only. A single character that escapes the next character inside a region, preventing `end` from matching. Example: `escape = "\\"` means `\"` does not close a `"..."` string. |

### Rule Matching Priority

Rules are evaluated in file order — **first matching rule wins**.

1. More specific rules should come before general ones
2. Region rules (`start`/`end`) take precedence over `pattern` rules at the same position
3. Regions do not nest by default

> **Tip**: Place comment and string region rules near the top of your rule list.

> **Performance note**: Rules are compiled once at startup using the `regex` crate and applied in parallel with `rayon`. Prefer anchored patterns; avoid heavy backtracking.

---

## 7. Internationalization (i18n)

**led** loads UI strings from a locale file at startup.

### Selecting a Language

```toml
# ~/.config/led/config.toml
language = "ja"   # Japanese
```

### Built-in Locales

| Code | Language |
| :--- | :--- |
| `en` | English (default) |
| `ja` | Japanese |

### Custom Locale File Format

Create `~/.config/led/locales/<code>.toml`. Any key omitted falls back to the `en` built-in.

```toml
# ~/.config/led/locales/en.toml
# Complete English built-in locale — use as reference for translations.

[meta]
language    = "en"
name        = "English"
author      = "led contributors"

[menu]
file        = "File"
edit        = "Edit"
view        = "View"
help        = "Help"

[menu.file]
new         = "New"
open        = "Open…"
save        = "Save"
save_as     = "Save As…"
close       = "Close"
exit        = "Exit"

[menu.edit]
undo        = "Undo"
redo        = "Redo"
cut         = "Cut"
copy        = "Copy"
paste       = "Paste"
find        = "Find…"
replace     = "Replace…"
select_all  = "Select All"

[menu.view]
go_to_line   = "Go to Line…"
line_numbers = "Line Numbers"
word_wrap    = "Word Wrap"
vi_mode      = "Vi Mode"
encoding     = "Encoding"
line_ending  = "Line Ending"
theme        = "Theme"
syntax       = "Syntax"

[panel]
find            = "Find:"
replace         = "Replace:"
prev            = "< Prev"
next            = "> Next"
replace_one     = "Replace"
replace_all     = "Replace All"
close           = "Close"
match_case      = "Match Case"
whole_word      = "Whole Word"
use_regex       = "Use Regex"

[status]
no_name               = "[No Name]"
no_matches            = "No matches"
search_wrapped_top    = "Search wrapped to top"
search_wrapped_bottom = "Search wrapped to bottom"
matches               = "{current} of {total} matches"
replaced_count        = "{n} replacement(s) made"
terminal_too_small    = "Terminal too small ({cols}x{rows}). Please resize."
cursor                = "Ln {line}, Col {col}"
selection             = "{n} chars"

[error]
cannot_open_dir       = "Cannot open directory: {path}"
failed_to_open        = "Failed to open {path}: {error}"

[dialog]
ok                      = "OK"
cancel                  = "Cancel"
yes                     = "Yes"
no                      = "No"
save                    = "Save"
dont_save               = "Don't Save"
discard_reopen          = "Discard & Reopen"
discard_reopen_prompt   = "Discard unsaved changes and reopen?"
reopen_file             = "Reopen File"
open_file               = "Open File…"
save_as                 = "Save As…"
go_to_line              = "Go to Line"
about                   = "About"
show_hidden             = "Show Hidden"
detect_encoding         = "Detect Encoding"
overwrite_prompt        = "File already exists. Overwrite?"
unsaved_changes_title   = "Unsaved Changes"
unsaved_changes         = "Unsaved changes in \"{filename}\"."

[dialog.file_browser]
name                    = "Name"
size                    = "Size"
modified                = "Modified"
filename                = "File name"

[about]
version     = "Version"
license     = "License"
```

---

## 8. SSH Usage Notes

### Clipboard over SSH (OSC 52)

**led** supports clipboard sharing over SSH via the **OSC 52** escape sequence.  
When you copy text (`Ctrl+C`), **led** sends an OSC 52 sequence that instructs your **local** terminal emulator to place the text in your local clipboard. This happens alongside a regular platform clipboard write — both are always attempted.

**Supported terminal emulators**:
- iTerm2 (macOS) — enable in Preferences → General → Applications in terminal may access clipboard
- WezTerm — enabled by default
- Windows Terminal — enabled by default

If your terminal does not support OSC 52, `Ctrl+C` still copies to the remote clipboard (accessible within the same SSH session).

### Ctrl+S Freezing

Some shell configurations interpret `Ctrl+S` as `XOFF` (pause output), causing the terminal to appear frozen. **led** disables this via raw mode, but as a precaution add this to your `~/.bashrc` or `~/.zshrc`:

```bash
stty -ixon
```

### Performance

**led** uses diff-based rendering (only changed screen cells are redrawn), minimizing bytes sent over the network. It performs well on connections with up to several hundred milliseconds of latency.

---

## 9. Troubleshooting

### Terminal appears frozen after Ctrl+S
Press `Ctrl+Q` to unfreeze (XON), then add `stty -ixon` to your shell profile. See [Section 8](#8-ssh-usage-notes).

### Japanese/Chinese text displays with wrong column alignment
Ensure your terminal emulator uses a **monospace font with CJK support** (e.g., Noto Mono, Sarasa Mono, HackGen). **led** uses Unicode display-cell widths (CJK = 2 cells); the terminal font must agree.

### Terminal too small message
Resize your terminal to at least **40 columns × 24 rows**. **led** resumes automatically.

### Theme or syntax not appearing in menu
Check that the `.toml` file is in the correct directory and restart **led**. Config files are loaded at startup only.

### Mouse not working over SSH
Ensure your SSH client passes through mouse escape sequences. In PuTTY, enable "xterm-style mouse reporting" in Terminal → Features settings.

### Undo clears the unsaved-changes indicator
This is expected behavior. When you undo all changes since the last save, the file is back to its saved state and `[+]` is removed from the tab and status bar.
