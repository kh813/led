# App Specs: led (lightweight editor)

**led** is a modern TUI text editor, re-implemented from scratch with inspiration from Microsoft Edit and Micro editor. It aims to provide a clean, accessible, and powerful editing experience for terminal users across macOS, Linux, and Windows. A separate GUI binary (`led-gui`) is planned for the future, built on gpui and sharing the same core logic.

---

## 1. Core Properties

- **Binary names**:
  - TUI editor: `led` (from `crates/led-tui`)
  - GUI editor: `led-gui` (from `crates/led-gui`) — future plan, separate binary
  - No other variants (`led-tui`, etc.) shall be introduced
- **Build output names by platform**:

  | Context | Platform | Binary | Notes |
  | :--- | :--- | :--- | :--- |
  | `make` (default, single target) | macOS | `led` (TUI), `led.app` (GUI) | GUI is future |
  | `make` (default, single target) | Linux | `led` (TUI) | GUI future / undecided |
  | `make` (default, single target) | Windows | `led.exe` (GUI only) | TUI not shipped on Windows |
  | `make all` (cross-build from macOS) | macOS arm64 TUI | `led.mac-arm64` | cargo native |
  | `make all` (cross-build from macOS) | macOS x64 TUI | `led.mac-x64` | cargo native |
  | `make all` (cross-build from macOS) | macOS arm64 GUI | `led.app` | cargo native, bundled |
  | `make all` (cross-build from macOS) | Linux x64 TUI | `led.linux-x64` | `cross` (Docker) |
  | `make all` (cross-build from macOS) | Linux arm64 TUI | `led.linux-arm64` | `cross` (Docker) |
  | GitHub Actions `windows-latest` runner | Windows x64 GUI | `led.exe` | `x86_64-pc-windows-msvc`; triggered by release tag |

  > **Windows build**: `x86_64-pc-windows-msvc` target. Cannot be cross-compiled from macOS (MSVC SDK required). Built on a GitHub Actions `windows-latest` runner via `.github/workflows/release-windows.yml`, triggered by the same release tag as `make all`. `make all` does not include a Windows target.

  > **`cross` tool required for `make all`**: Linux targets use the [`cross`](https://github.com/cross-rs/cross) tool (Docker-based cross-compilation). Install with `cargo install cross`. Docker must be running when executing `make all`.
- **TUI-First**: Looks like a genuine modern terminal application, not a relic of the DOS era
- **Menu-Driven**: Features a top-level menu bar that is fully clickable and mouse-selectable, similar to Microsoft Edit
- **Dialog-Based**: Uses GUI-like dialogs for interactions (file opening, saving, settings)
- **Internationalization (i18n)**: Built from the ground up to support multiple languages
- **Configurable**:
  - Configuration path: `~/.config/led/`
  - Format: `.toml`
  - Theme files: `~/.config/led/themes/*.toml`
  - Syntax definition files: `~/.config/led/syntax/*.toml`
  - All config files are loaded at startup only. Changes require a restart.
  - **Config loading behavior**:
    - If `~/.config/led/config.toml` exists, it is loaded and its values override built-in defaults
    - If it does not exist, `led` runs on built-in defaults — the file is **not** auto-generated
    - Unknown keys are silently ignored; missing keys fall back to built-in defaults
  - **Runtime config writes**: When the user changes a persistent setting at runtime (e.g., toggling Line Numbers, switching Theme), `led` writes **only that key** back to `~/.config/led/config.toml`:
    - If the file already exists: the specific key is updated in place (other keys untouched)
    - If the file does not exist: it is **created** with only the changed key(s) and a header comment. This is the one exception to the "do not auto-generate" rule — a user action triggers it explicitly
  - **Template file**: `config.toml.default` is shipped with the source (under `assets/`) and documents every key, its default, and accepted values
- **Tab Support**: Multiple open files via a tab bar, inspired by Micro editor
- **Remote-Friendly (OSC 52)**: Supports OSC 52 for clipboard sharing over SSH

---

## 2. Repository Structure (Cargo Workspace)

`led` is organized as a Cargo workspace. This structure separates platform-specific code from the shared core, enabling future GUI support without rewriting business logic.

```
led/                              ← workspace root
├── Cargo.toml                   ← [workspace] members definition
├── crates/
│   ├── led-core/                ← shared logic (no TUI/GUI deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── buffer.rs        ← ropey wrap + undo/redo stack
│   │       ├── search.rs        ← regex / aho-corasick
│   │       ├── syntax.rs        ← highlight engine (returns Vec<TokenSpan>)
│   │       ├── config.rs        ← toml-span parse + runtime write
│   │       ├── encoding.rs      ← encoding_rs wrap
│   │       ├── theme.rs         ← color structs (RGB values only, no rendering)
│   │       └── i18n.rs          ← locale string loading
│   ├── led-tui/                 ← crossterm TUI, binary: led
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs           ← focus state machine, event dispatch
│   │       ├── renderer.rs      ← diff rendering (double-buffered cells)
│   │       ├── input.rs         ← crossterm Event → Action
│   │       ├── layout.rs        ← region geometry, hit-test
│   │       ├── widgets/
│   │       │   ├── menu.rs
│   │       │   ├── tab_bar.rs
│   │       │   ├── editor_area.rs
│   │       │   ├── find_panel.rs
│   │       │   ├── status_bar.rs
│   │       │   └── dialog.rs
│   │       └── clipboard.rs     ← OSC 52 + platform clipboard
│   └── led-gui/                 ← gpui GUI, binary: led-gui (future)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── app.rs           ← gpui Application + WindowContext
│           ├── editor_view.rs   ← gpui View for editor area
│           ├── menu_bar.rs      ← gpui View for menu bar
│           ├── tab_bar.rs       ← gpui View for tab bar
│           ├── find_panel.rs    ← gpui View for find/replace panel
│           ├── status_bar.rs    ← gpui View for status bar
│           └── dialog.rs        ← gpui modal dialogs
├── assets/
│   ├── config.toml.default
│   ├── themes/
│   │   ├── tokyo-night.toml
│   │   ├── light.toml
│   │   ├── solarized-dark.toml
│   │   ├── solarized-light.toml
│   │   ├── catppuccin-mocha.toml
│   │   └── catppuccin-latte.toml
│   └── syntax/
│       ├── plain-text.toml
│       ├── markdown.toml
│       ├── rust.toml
│       ├── toml.toml
│       ├── python.toml
│       ├── go.toml
│       ├── swift.toml
│       ├── javascript.toml
│       ├── html.toml
│       ├── css.toml
│       └── xml.toml
├── MANUAL.md
└── README.md
```

### led-core Public API (contract between core and frontends)

Both `led-tui` and `led-gui` depend only on `led-core`. Neither frontend contains buffer logic.

```rust
// led-core::buffer
pub struct Editor { /* ropey Rope + undo stack + per-buffer state */ }

impl Editor {
    pub fn insert(&mut self, pos: usize, text: &str) -> EditDelta;
    pub fn delete(&mut self, range: Range<usize>) -> EditDelta;
    pub fn undo(&mut self) -> Option<EditDelta>;
    pub fn redo(&mut self) -> Option<EditDelta>;
    pub fn is_modified(&self) -> bool;    // false after full undo to saved state
    pub fn line_count(&self) -> usize;
    pub fn line(&self, idx: usize) -> RopeSlice;
}

// led-core::search
pub struct SearchQuery { pub pattern: String, pub flags: SearchFlags }
pub struct Match { pub line: usize, pub byte_range: Range<usize> }
impl Editor {
    pub fn search(&self, query: &SearchQuery) -> Vec<Match>;
}

// led-core::syntax
pub struct TokenSpan { pub byte_range: Range<usize>, pub token: TokenType }
impl Editor {
    pub fn highlight_line(&self, line: usize) -> Vec<TokenSpan>;
}
```

`EditDelta` carries enough information for both the TUI diff-renderer and the gpui invalidation system to know exactly which cells/regions changed.

---

## 3. Technical Stack

### Workspace-level Cargo.toml

```toml
[workspace]
members = ["crates/led-core", "crates/led-tui", "crates/led-gui"]
resolver = "2"

[workspace.dependencies]
anyhow        = { version = "1.0", features = ["backtrace"] }
serde         = { version = "1.0", features = ["derive"] }
toml-span     = "0.6"
regex         = "1.12"
aho-corasick  = "1.1"
unicode-width = "0.2"
encoding_rs   = "0.8"
ropey         = "1.6"
rayon         = "1.11"
```

### led-core dependencies

```toml
[dependencies]
anyhow        = { workspace = true }
serde         = { workspace = true }
toml-span     = { workspace = true }
regex         = { workspace = true }
aho-corasick  = { workspace = true }
unicode-width = { workspace = true }
encoding_rs   = { workspace = true }
ropey         = { workspace = true }
rayon         = { workspace = true }
```

### led-tui dependencies

```toml
[dependencies]
led-core  = { path = "../led-core" }
crossterm = "0.28"
anyhow    = { workspace = true }
```

### led-gui dependencies (future)

```toml
[dependencies]
led-core = { path = "../led-core" }
gpui     = { git = "https://github.com/zed-industries/zed", package = "gpui" }
anyhow   = { workspace = true }
```

> **Note on gpui**: gpui is developed as part of Zed editor and does not have independent crates.io releases. It is sourced directly from the Zed repository. The API is not yet stable; pin to a specific commit SHA when starting `led-gui` development to avoid unexpected breakage.

### Crate responsibilities

| Crate | Key deps | Purpose |
| :--- | :--- | :--- |
| **led-core** | ropey, regex, aho-corasick, encoding_rs, rayon, toml-span | All editor logic. No I/O, no rendering, no platform APIs |
| **led-tui** | crossterm, led-core | Terminal rendering, keyboard/mouse input, OSC 52 clipboard |
| **led-gui** | gpui, led-core | GPU-accelerated native GUI window (future) |

### Why crossterm (not libc/windows-sys)

Direct `libc`/`windows-sys` usage requires separate implementations for Unix `termios`, Windows Console API, and SGR mouse protocol negotiation. `crossterm` handles all of this internally and is the de-facto standard for Rust TUI applications (used by Ratatui, Helix, etc.).

---

## 4. UI Layout (TUI)

The screen is divided into fixed regions from top to bottom:

1. **Menu Bar** (top row, always visible)
2. **Tab Bar** (second row, always visible)
3. **Find/Replace Panel** (optional, inserted into layout flow when active)
4. **Editor Area** (middle, scrollable)
5. **Status Bar** (bottom row, always visible)

### Region Heights

| Region | Height | Row range |
| :--- | :--- | :--- |
| Menu Bar | 1 row | row 0 |
| Tab Bar | 1 row | row 1 |
| Find/Replace Panel | 0 / 2 / 3 rows | rows 2..N−1 |
| Editor Area | remaining | rows N..term_rows−2 |
| Status Bar | 1 row | row term_rows−1 |

`N = 2 + panel_height` (0, 2, or 3).
**Editor Area height** = `term_rows − 2 − panel_height − 1`

### Hit Testing (Mouse Click Routing)

```
if row == 0                    → Menu Bar
else if row == 1               → Tab Bar
else if row < 2 + panel_height → Find/Replace Panel
else if row == term_rows - 1   → Status Bar (future: Go to Line on click)
else                           → Editor Area
```

Recomputed after every `Event::Resize`.

**Menu Bar hit detail**:
- 4 items: `File`, `Edit`, `View`, `Help`
- Col ranges stored as `menu_bar_items: [(label, col_start, col_end); 4]`, recomputed each render
- Click inside range → open that menu; click outside → close any open menu

**Tab Bar hit detail**:
- Each tab's col range stored as `tab_rects: Vec<(tab_index, col_start, col_end)>`
- Single click → switch tab; middle-click or `×` glyph click → close tab (unsaved-changes prompt if needed)

**Editor Area hit detail**:
- `editor_origin_row = 2 + panel_height`
- `editor_origin_col` = gutter width (0 if line numbers hidden, else `floor(log10(last_line)) + 2`)
- `col < editor_origin_col` → gutter click → select entire line
- Otherwise → translate to buffer position using scroll offsets and `unicode-width`

### Focus State Machine

Exactly one focus owner holds keyboard input at all times:

| Focus owner | Constant |
| :--- | :--- |
| Editor Area | `Focus::Editor` |
| Menu (any level) | `Focus::Menu` |
| Find/Replace Panel | `Focus::Panel` |
| Dialog | `Focus::Dialog` |

**Transitions**:
```
Focus::Editor
  Alt+F/E/V/H  → Focus::Menu
  Ctrl+F        → Focus::Panel (Find only)
  Ctrl+H        → Focus::Panel (Find & Replace)
  Ctrl+O / etc. → Focus::Dialog

Focus::Menu
  Esc           → Focus::Editor
  Enter         → execute action → Focus::Editor (or Focus::Dialog)
  ←/→           → move between top-level menus
  ↑/↓           → move within dropdown

Focus::Panel
  Esc           → Focus::Editor
  Tab/Shift+Tab → cycle within panel
  click editor  → Focus::Editor (panel stays open)

Focus::Dialog
  Esc/Cancel    → Focus::Editor
  Enter/OK      → execute → Focus::Editor
```

Mouse events always route via hit-test regardless of focus.

---

## 5. Menu Bar

- Items: `File` / `Edit` / `View` / `Help` (4 items only)
- Dropdowns are **borderless** (no box border), inspired by kh813/edit
- Fully mouse-clickable and keyboard-navigable
- `Alt+F/E/V/H` opens the corresponding menu directly
- Once open: `←/→` moves between top-level menus, `↑/↓` within dropdown, `Enter` activates, `Esc` closes

### Menu Items Reference

| Menu | Action | Shortcut | Notes |
| :--- | :--- | :--- | :--- |
| **File** | New | Ctrl+N | |
| | Open… | Ctrl+O | |
| | ───── | | |
| | Save | Ctrl+S | |
| | Save As… | Ctrl+Shift+S | |
| | ───── | | |
| | Close | Ctrl+W | |
| | ───── | | |
| | Exit | Ctrl+Q | |
| **Edit** | Undo | Ctrl+Z | |
| | Redo | Ctrl+Y | |
| | ───── | | |
| | Cut | Ctrl+X | |
| | Copy | Ctrl+C | |
| | Paste | Ctrl+V | |
| | ───── | | |
| | Find… | Ctrl+F | |
| | Replace… | Ctrl+H | |
| | ───── | | |
| | Select All | Ctrl+A | |
| **View** | Go to Line… | Ctrl+G | |
| | ───── | | |
| | `[x]` Line Numbers | — | toggle, on by default |
| | `[ ]` Word Wrap | — | toggle |
| | `[ ]` Vi Mode | — | toggle |
| | ───── | | |
| | Encoding ▶ | — | submenu |
| | Line Ending ▶ | — | submenu |
| | ───── | | |
| | Theme ▶ | — | submenu |
| | Syntax ▶ | — | submenu |
| **Help** | About | — | |

Toggle items show `[ ]` (off) or `[x]` (on); selecting flips state immediately.

### View > Encoding submenu

```
Encoding ▶  Reopen with Encoding ▶  ── reload file from disk with selected encoding
            ─────────────────────────
            Convert to Encoding ▶    ── change save encoding without reloading
```

Supported encodings: `UTF-8`, `UTF-8 with BOM`, `UTF-16 LE`, `UTF-16 BE`, `Shift-JIS`, `EUC-JP`, `ISO-2022-JP`, `Latin-1 (ISO-8859-1)`

- `✓` marks the active encoding in `Convert to Encoding`
- **Reopen**: discards unsaved changes (confirmation dialog if any), reloads from disk
- **Convert**: changes save encoding; reflected in status bar immediately

### View > Line Ending submenu

```
Line Ending ▶  ✓ LF
               CRLF
               CR
```

- Changes apply at save time; `✓` marks current buffer's setting
- Shown in status bar at all times

### View > Theme / Syntax submenus

- Populated at startup by scanning `~/.config/led/themes/` and `~/.config/led/syntax/`
- Built-in entries always listed first, user entries below a separator
- `✓` marks the active selection
- Theme change: applies immediately and persists to `config.toml`
- Syntax change: applies to current buffer only (overrides auto-detection)

---

## 6. Tab Bar

- **Visual**: active tab highlighted with theme accent + bold; inactive tabs dimmed
- Each tab shows `[+]` prefix for unsaved changes, `[RO]` for read-only
- Close button `×` visible on each tab; middle-click also closes
- Scrollable horizontally if tabs exceed screen width (scroll arrows `<` `>` appear at edges when needed; `Ctrl+Tab` always scrolls to keep the active tab visible)
- **Keyboard**: `Ctrl+T` new tab, `Ctrl+W` close, `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle

---

## 7. Find/Replace Panel

An **inline, non-modal panel** inserted between Tab Bar and Editor Area.

**Two modes**:

| Trigger | Mode | Rows |
| :--- | :--- | :--- |
| `Ctrl+F` / `Edit > Find…` | Find only | 2 |
| `Ctrl+H` / `Edit > Replace…` | Find & Replace | 3 |

`Ctrl+H` while Find-only expands in place; `Ctrl+F` while Find & Replace collapses in place.

**Find-only layout** (2 rows):
```
Find:    [______________________________]  < Prev  > Next  Close
[ ] Match Case  [ ] Whole Word  [ ] Use Regex
```

**Find & Replace layout** (3 rows):
```
Find:    [______________________________]  < Prev  > Next  Close
Replace: [______________________________]  Replace  Replace All
[ ] Match Case  [ ] Whole Word  [ ] Use Regex
```

**Search direction**:
- `> Next`: downward (toward EOF); wraps to top → status bar shows `Search wrapped to top`
- `< Prev`: upward (toward BOF); wraps to bottom → status bar shows `Search wrapped to bottom`

**Incremental search**: highlights all matches as the user types; auto-scrolls to the first match at or after cursor. Match count shown in status bar (`3 of 12 matches`). No matches → Find input turns error color + status bar shows `No matches`.

**Keyboard in panel**:
- `Tab` / `Shift+Tab`: cycle focus between fields and toggle buttons
- `Enter` / `F3` in Find input: next match (downward)
- `Shift+Enter` / `Shift+F3` in Find input: previous match (upward)
- `Enter` in Replace input: Replace and advance
- `Esc`: close panel

**Panel is per-tab**: switching tabs keeps panel visible but clears search state.

---

## 8. Editor Area

- Main text buffer, vertically and horizontally scrollable
- Line numbers in left gutter by default (toggleable)
- Cursor: blinking block (normal mode) or beam (insert mode)
- **Storage**: `ropey` Rope — O(log n) insert/delete for large files
- **Rendering**: diff-based double-buffered; only changed cells are emitted per frame

### Undo / Redo

- Each `Editor` instance maintains an undo stack (`Vec<EditDelta>`) and a redo stack
- Stack limit: 1000 operations (oldest entry dropped when exceeded)
- `undo()` reverts the last `EditDelta` and pushes the inverse onto the redo stack
- `redo()` replays the top of the redo stack
- Any new edit clears the redo stack
- `is_modified()` returns `false` when the current state matches the last-saved state (i.e., after undoing all changes since last save). This correctly clears the `[+]` indicator in the tab bar and status bar

### Word Wrap

- When word wrap is **off** (default): lines extend beyond the terminal width; horizontal scroll is available (`Shift + scroll wheel`)
- When word wrap is **on**:
  - Long lines are broken into **visual lines** for display; the underlying buffer line is unchanged
  - `↑` / `↓` move by **visual line** (not logical line) when word wrap is on
  - `Ln {n}` in the status bar always shows **logical line** number
  - `Col {n}` shows **visual column** (display cells from the start of the visual line)
  - Horizontal scroll is disabled when word wrap is on

### Mouse Interaction

| Action | Behavior |
| :--- | :--- |
| Single click | Move cursor |
| Click + drag | Select text range (character-level) |
| Double click | Select word under cursor |
| Triple click | Select entire line |
| Shift + click | Extend/shrink selection to clicked position |
| Scroll wheel up/down | Scroll vertically without moving cursor |
| Shift + scroll wheel | Scroll horizontally (word wrap off only) |
| Click line number gutter | Select entire corresponding line |

Selected text highlighted with theme's selection color. Selection info (`{n} chars`) shown in status bar.

### CJK and Full-Width Characters

- All rendering uses `unicode-width` for display-cell widths (ASCII=1, CJK=2, combining=0)
- `Col {n}` reflects visual column (display cells), not byte offset
- Tab stops default to 4-cell boundary (configurable via `tab_size`)

### Terminal Resize and Minimum Size

- Resize events via `Event::Resize(cols, rows)` — no `SIGWINCH` needed
- **Minimum**: 24 rows × 40 columns. Below threshold, all UI is hidden and a centered message is shown: `Terminal too small ({cols}×{rows}). Please resize.`
- Normal rendering resumes automatically once the terminal is enlarged

---

## 9. Status Bar

Left segment: file name (or `[No Name]`), `[+]` if modified.

Right segment (left to right):
- Search state: `{n} of {total} matches` or `No matches` (only when panel is open)
- Selection info: `{n} chars` (when text is selected; takes priority over search state)
- Cursor: `Ln {line}, Col {col}` (logical line, visual column)
- Encoding: e.g., `UTF-8`
- Line ending: `LF`, `CRLF`, or `CR`
- Syntax: e.g., `Markdown`
- Vi mode indicator (only when vi mode enabled): `NORMAL` / `INSERT` / `VISUAL`

---

## 10. Dialogs

All dialogs are modal, centered, dismissible with `Esc`. Border style: single-line Unicode box-drawing characters.

| Dialog | Trigger |
| :--- | :--- |
| **Open File** | Ctrl+O |
| **Save As** | Ctrl+Shift+S |
| **Go to Line** | Ctrl+G |
| **Unsaved Changes** | Close/Exit with unsaved changes |
| **Reopen Confirmation** | Reopen with Encoding when unsaved changes exist |
| **About** | Help > About |

### Open File / Save As dialog

```
┌─ Open File… ──────────────────────────────────────────────┐
│ /Users/username/projects                                    │
│ [ ] Show Hidden (Alt+H)  |  [x] Detect Encoding (Alt+E)   │
│ Navigation:  ..  |  /  |  ~  |  Documents  |  Downloads   │
│ ──────────────────────────────────────────────────────     │
│ Name ▲                              Size       Modified    │
│ ──────────────────────────────────────────────────────     │
│ ../                                 --         --          │
│ src/                                --         2026-03-20  │
│ Cargo.toml                          1.2 KB     2026-03-19  │
│ README.md                           4.5 KB     2026-03-18  │
│ ──────────────────────────────────────────────────────     │
│ File name: [________________________________________]      │
│                                       [ Open ]  [ Cancel ] │
└────────────────────────────────────────────────────────────┘
```

- Column header click: sort ascending `▲` / descending `▼`; default `Name ▲`
- `↑/↓`: move selection; `Enter` on dir: navigate in; `Enter` on file: confirm; `Backspace`: parent dir; typing: typeahead jump
- Save As: overwrite confirmation if file already exists

---

## 11. Vi Mode

- **Off by default**; opt-in via `vi_mode = true` in config or `View > Vi Mode` toggle
- Modes: `NORMAL`, `INSERT`, `VISUAL`
- Normal mode bindings: `h/j/k/l`, `w/b/e`, `dd`, `yy`, `p`, `u`, `gg`, `G`
- Command-line mode: `:w`, `:q`, `:wq`
- `/` opens Find/Replace Panel in Find-only mode; `Esc` in panel returns to Normal mode
- Vi keybindings suspended while panel has focus
- Current mode shown in status bar

---

## 12. Command-Line Interface

```
led [FILE...]
```

| Invocation | Behavior |
| :--- | :--- |
| `led` | Empty buffer `[No Name]` |
| `led myfile.txt` | Open file; if not found, empty named buffer (not written until Save) |
| `led a.txt b.txt` | Each file in its own tab; first tab active |
| `led /some/dir` | Error dialog + empty buffer |
| `led /unreadable` | Error dialog + empty buffer |

No flags or subcommands in initial release.

---

## 13. Clipboard

- On copy/cut (`Ctrl+C` / `Ctrl+X`): **both** OSC 52 sequence **and** platform clipboard API are attempted concurrently. OSC 52 is not a fallback — it is always emitted alongside the platform write, because there is no reliable acknowledgement mechanism for OSC 52
- On paste (`Ctrl+V`): platform clipboard is read first; bracketed paste from terminal (OSC 52 response) is also accepted
- Platform clipboard implementation is in `led-tui/src/clipboard.rs` and is entirely absent from `led-core`
- `led-gui` uses gpui's native clipboard API instead of OSC 52

---

## 14. Syntax Highlighting

- Engine: `regex` crate (RE2 syntax) + `rayon` for parallelism
- **Highlight strategy**:
  - Line-level parallelism via `rayon`: independent lines are highlighted in parallel
  - Region rules (`start`/`end` spanning multiple lines) require state: these are resolved in a first linear pass to mark region boundaries, then parallel coloring is applied within known boundaries
  - On edit: only lines marked dirty (by `EditDelta`) are re-highlighted. Region boundaries are re-resolved from the first dirty line to the next clean boundary
- Built-in syntax definitions: `Plain Text`, `Markdown`, `Rust`, `TOML`, `Python`, `Go`, `Swift`, `JavaScript`, `HTML`, `CSS`, `XML`
- Auto-detection by file extension on open; overridable via `View > Syntax`

---

## 15. Themes

- Built-in themes: `Tokyo Night` (default), `Light`, `Solarized Dark`, `Solarized Light`, `Catppuccin Mocha`, `Catppuccin Latte`
- Theme selection applies immediately and persists to `config.toml`
- Theme color values in `led-core::theme` are plain RGB structs — no terminal escape codes or gpui types. Both frontends map these to their own color primitives

---

## 16. Internationalization (i18n)

- Locale files: built-in (`en`, `ja`) + user files at `~/.config/led/locales/<code>.toml`
- Missing keys fall back to the `en` built-in
- `language` key in `config.toml` selects the locale at startup

---

## 17. GUI Version (led-gui) — Future Plan

### Overview

`led-gui` is a native GUI editor sharing all business logic with `led-tui` through `led-core`. It is a **separate binary** built in a future development phase after TUI completion.

### Framework: gpui

gpui is the GPU-accelerated UI framework developed by Zed Industries for the Zed editor. It is chosen because:
- It is purpose-built for text editors (Zed itself is a text editor)
- Provides native text rendering with font fallback, ligatures, and CJK support out of the box
- Immediate-mode-like model aligns well with TUI's per-frame rendering philosophy
- Rust-native with no JavaScript bridge
- Cross-platform: macOS (primary), Linux, Windows

**Constraints to be aware of**:
- No independent crates.io release; sourced from the Zed repository via git dep
- API is not yet stable; must pin to a specific commit SHA at project start
- macOS support is most mature; Linux/Windows may have rough edges
- Documentation is sparse; reading Zed's own source code is the primary reference

### Menu Bar: Platform-Specific Design

`led-gui` follows the same platform conventions as Zed editor for menu bar placement:

| Platform | Menu bar location | Implementation |
| :--- | :--- | :--- |
| **macOS** | OS-native menu bar (top of screen, outside the window) | `gpui::App::set_menus()` — registers menus with the macOS NSMenu/NSMenuItem system via gpui. The in-window area where the menu bar would appear is **not rendered**; the window content starts directly with the tab bar. |
| **Windows** | In-window menu bar (inside the window, top row) | Custom gpui View rendered as the topmost row of the window, identical in layout and behavior to the TUI menu bar. |
| **Linux** | In-window menu bar (same as Windows) | Same custom gpui View as Windows. No assumption is made about a desktop environment providing a global menu bar. |

**Why this split?**

On macOS, applications are expected to use the OS menu bar. Rendering a duplicate in-window menu bar would look non-native and violate macOS HIG. On Windows and Linux, there is no reliable OS-level menu bar, so the menu must live inside the window — matching the TUI experience.

This is exactly the approach taken by Zed itself: `app.rs` calls `set_menus()` which on macOS populates `NSMenuBar`, and on other platforms the menu bar is an in-window View.

#### macOS: Native Menu Bar via `gpui::App::set_menus()`

```rust
// crates/led-gui/src/app.rs  (macOS path)
app.set_menus(vec![
    Menu {
        name: "led-gui".into(),   // Application menu (shows app name)
        items: vec![
            MenuItem::action("About led-gui", AboutAction),
            MenuItem::separator(),
            MenuItem::action("Quit led-gui", QuitAction),
        ],
    },
    Menu {
        name: i18n.get("menu.file").into(),   // "File"
        items: vec![
            MenuItem::action(i18n.get("menu.file.new"),    NewAction),
            MenuItem::action(i18n.get("menu.file.open"),   OpenAction),
            MenuItem::separator(),
            MenuItem::action(i18n.get("menu.file.save"),   SaveAction),
            MenuItem::action(i18n.get("menu.file.save_as"), SaveAsAction),
            MenuItem::separator(),
            MenuItem::action(i18n.get("menu.file.close"),  CloseTabAction),
        ],
    },
    Menu {
        name: i18n.get("menu.edit").into(),   // "Edit"
        items: vec![ /* Undo, Redo, Cut, Copy, Paste, Find, Replace, Select All */ ],
    },
    Menu {
        name: i18n.get("menu.view").into(),   // "View"
        items: vec![ /* Go to Line, Line Numbers, Word Wrap, Vi Mode, Theme, Syntax … */ ],
    },
    Menu {
        name: i18n.get("menu.help").into(),   // "Help"
        items: vec![ /* About */ ],
    },
], cx);
```

- Menu labels come from `led-core::i18n` so localization applies to the native menu bar too
- Keyboard shortcuts registered here are enforced by macOS and shown in the native menu (e.g., `⌘S` for Save)
- Toggle items (Line Numbers, Word Wrap, Vi Mode) use `MenuItem::action` with a checked state; the check mark is updated by sending an action that re-calls `set_menus()` with the updated state

#### Windows / Linux: In-Window Menu Bar View

```
┌─────────────────────────────────────────────────┐  ← Window frame
│ File   Edit   View   Help                       │  ← menu_bar.rs (gpui View, row 0)
│ [+] main.rs × │ README.md ×                    │  ← tab_bar.rs  (gpui View, row 1)
│ Find:  [_______________________]  > Next  Close │  ← find_panel.rs (optional)
│                                                 │
│  1 │ fn main() {                                │  ← editor_view.rs
│  2 │     println!("Hello");                     │
│                                                 │
│ [+] main.rs    Ln 2, Col 5  UTF-8  LF  Rust    │  ← status_bar.rs
└─────────────────────────────────────────────────┘
```

- `menu_bar.rs` renders as a single-row gpui View at the top of the window content area
- Dropdowns are rendered as floating gpui Views positioned below the clicked menu item (same borderless style as the TUI)
- Mouse click and keyboard (`Alt+F/E/V/H`, arrow keys, `Enter`, `Esc`) behave identically to the TUI
- Toggle items show a check mark glyph (`✓`) when active, rendered in the dropdown

#### Platform Detection at Runtime

```rust
// crates/led-gui/src/app.rs
#[cfg(target_os = "macos")]
fn setup_menu(app: &mut gpui::App, i18n: &I18n, cx: &mut AppContext) {
    app.set_menus(build_native_menus(i18n), cx);
    // No in-window menu bar rendered
}

#[cfg(not(target_os = "macos"))]
fn setup_menu(_app: &mut gpui::App, _i18n: &I18n, _cx: &mut AppContext) {
    // In-window menu_bar.rs View is added to the window layout instead
    // (handled in window_view.rs)
}
```

#### Menu Action Routing

All menu actions — whether triggered by the native macOS menu, the in-window menu bar, or keyboard shortcuts — dispatch the same `Action` enum values from `led-core`. This ensures no duplicated logic:

```
Native macOS menu item clicked
    → gpui dispatches Action to focused View
    → editor_view.rs / app.rs handles Action
    → calls led-core API (same code path as TUI)

In-window menu item clicked (Windows/Linux)
    → menu_bar.rs dispatches Action
    → same handler as above
```

#### File Drag & Drop (GUI Only)

- **Window Drop**: Accept file drop events on the main window at any time.
- **App Icon Drop**: If supported by the platform, handle files dropped onto the application icon.
- **Multiple Files**: Support dropping multiple files simultaneously.
- **Tab Logic**:
  - For each dropped file:
    - If the file is already open in an existing tab, do not create a new tab; simply switch focus to the existing tab.
    - If the file is not open, create a new tab for it.
  - After processing all dropped files, the last file in the list (or the single dropped file) should become the active tab.
- **Validation**:
  - Only process files that can be read as text.
  - Ignore non-file drops (e.g., text snippets, images) or show a non-intrusive error message if a file cannot be opened.
  - Ensure the drag-and-drop operation does not interrupt any ongoing modal dialogs unless the dialog is dismissed.

### Default Window Size

`led-gui` starts with a default window size optimized for readability:
- **Width**: 1008px
- **Height**: 826px
- **Position**: Centered on the primary display at startup.

### Theme Selection

`led` aims to feel native to its environment by following system-level theme preferences:

- **TUI (`led`)**:
  - Uses the `terminal-default` theme by default, which maps to the terminal's native background and foreground colors (using ANSI escape codes for Reset and standard 16 colors for syntax highlighting).
  - If a specific theme is selected in `config.toml`, it overrides the terminal defaults.
- **GUI (`led-gui`)**:
  - Automatically detects the OS light/dark mode preference on startup if `theme` in `config.toml` is set to `terminal-default` or left empty.
  - Defaults to `Tokyo Night` (dark) for dark mode and `Catppuccin Latte` for light mode.
  - User-selected themes in `config.toml` take precedence.

### Architecture

`led-gui` depends only on `led-core` and `gpui`. It does **not** depend on `crossterm`.

```
led-gui/src/
├── main.rs          entry point; platform-conditional menu setup
├── app.rs           gpui::App + Window creation + Action dispatch
├── window_view.rs   root View; composes all child Views; omits menu_bar on macOS
├── menu_bar.rs      in-window menu bar (Windows/Linux only)
├── editor_view.rs   gpui::View — text rendering, cursor, selection, scroll
├── tab_bar.rs       gpui::View — tab list, click to switch, close button
├── find_panel.rs    gpui::View — inline find/replace panel
├── status_bar.rs    gpui::View — bottom bar
└── dialog.rs        gpui modal dialogs (file open, save as, go to line, etc.)
```

`window_view.rs` is responsible for composing the layout. On macOS, it skips `menu_bar.rs` and begins with `tab_bar.rs`. On Windows/Linux, it places `menu_bar.rs` first.

#### Native GUI Rendering Implementation Details

To ensure visibility and parity with the TUI version:
- **Monospace Font**: `EditorView` MUST explicitly set a monospace font family (e.g., "Menlo", "Consolas", "Courier New") to ensure consistent character widths and height.
- **Line Layout**: Each line is rendered as a `flex-row` div with a fixed height.
- **Chunk Rendering**: Text is split into chunks based on syntax highlighting and selection. Each chunk is rendered in a `div` that MUST inherit the monospace font and have `h_full()` to ensure proper vertical alignment.
- **Scrolling**: Horizontal scrolling is implemented by wrapping the content area of each line in a `relative` div and applying a horizontal offset.
- **Transparency**: The editor background and text colors are derived directly from `led-core::theme` to maintain visual consistency.

### Feature Parity Target

`led-gui` targets full feature parity with `led-tui`:
- All editing operations (undo/redo, selection, search, replace)
- All themes (colors mapped from `led-core::theme` RGB structs to gpui colors)
- All syntax highlighting (same `led-core::syntax` engine)
- All config keys (same `led-core::config` loader)
- File drag & drop (GUI-only addition)
- Platform-appropriate menu bar (native on macOS, in-window on Windows/Linux)
- Native shortcuts: `Cmd+...` on macOS, `Ctrl+...` on Windows/Linux for standard actions (New, Open, Save, Quit, etc.)
- Japanese Inline Input (IME): full support for inline conversion and composition
- Native clipboard (gpui's built-in, no OSC 52 needed)

### What is NOT shared with led-tui

| led-tui only | led-gui only |
| :--- | :--- |
| crossterm raw mode | gpui window/event loop |
| Diff rendering (cell buffer) | GPU compositing |
| OSC 52 clipboard | Native OS clipboard via gpui |
| Terminal resize handling | Window resize via gpui |
| Unicode display-cell width for layout | Font metrics for layout |
| In-window TUI menu bar (all platforms) | macOS: native NSMenu bar; Windows/Linux: in-window gpui View |

---

## 18. Design Philosophy

- **Modern TUI Aesthetics**: Clean, high-contrast, visually pleasing — feels at home in modern terminal emulators
- **Flat Codebase within each crate**: Simple, navigable module structure; no deep nesting
- **Shared Core**: All editor logic lives in `led-core`; frontends are thin rendering/input layers
- **Empowerment through Simplicity**: Tabs, syntax highlighting, themes — without IDE complexity
- **Clean-Room Implementation**: Entirely original; user is the sole copyright holder

---

## 19. Not Planned

- IDE features (git integration, LSP, DAP)
- Vertical/horizontal split views
- Plugin system

---

## 20. References

- Microsoft Edit (original): https://github.com/microsoft/edit
- kh813/edit (UI reference for borderless menus, Tokyo Night style): https://github.com/kh813/edit
- Micro editor: https://github.com/micro-editor/micro
- Fresh editor (file dialog UX): https://getfresh.dev/
- Zed editor (gpui source): https://github.com/zed-industries/zed
