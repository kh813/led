# led Devlog

## 2026-05-11

### GUI Stability & UX (GUI)
- **GUI Crash Fix (SIGABRT)**: Resolved a crash caused by recursive action dispatch and re-entrant window updates in the `Quit` and `Exit` handlers.
- **Centralized Quit Logic**: Refactored the multi-window quit coordination to the application level in `app.rs`. `WindowView` no longer listens for `Quit` or `Exit` actions directly; instead, the global handler coordinates checking each window for modified buffers and prompting the user sequentially.
- **Focus Restoration**: Fixed a bug where focus was not properly restored to the editor after closing a dialog. Now explicitly focusing the editor's `FocusHandle` in all dialog close paths.
- **Window Centering & Theme Detection**: Implemented automatic window centering on the primary display and added OS light/dark mode detection for the default theme.

## 2026-05-10

### GUI Rendering & Documentation (GUI)
- **GUI Visibility**: Applied robust rendering fixes to `EditorView`. Text chunks now use `h_full` and `items_center` within a fixed-height flex-row to ensure vertical visibility. Explicitly set `font_family("Menlo")` for consistent monospace rendering.
- **Horizontal Scrolling**: Refactored `render_line` to use an absolute positioned wrapper for the content area, fixing horizontal scroll offset application.
- **Documentation**: Updated `app_specs.md` with detailed GUI rendering implementation requirements and refreshed `app_todo.md`.
    - **GUI**: Updated `bounds_for_range` and `render_line` to use visual column calculations (via `unicode-width`) instead of raw character indices. This ensures the IME candidate window appears at the correct cursor position for CJK text.
    - **TUI**: Added hardware cursor movement to the logical cursor position after each render pass, allowing terminal emulators to properly position the IME candidate window.

### GUI Bug Fixes & UX Improvements
- **GUI Re-entrancy Crash Fix**: Fixed a `panic_already_borrowed` crash caused by synchronous file dialogs blocking the GPUI event loop. Replaced `rfd` synchronous calls with `AsyncFileDialog` and `cx.spawn` in both `app.rs` and `window_view.rs`.
- **Japanese Inline IME Support**: Implemented `marked_text_range` in `EditorView` and refined composition handling to support native inline input on macOS.
- **Color Consistency**: Unified `led_color_to_gpui` across all widgets to ensure alpha is explicitly set to 1.0, avoiding accidental transparency issues. Replaced hardcoded colors in `FindPanel` with theme-aware colors.
- **Standard Shortcuts**: Added missing macOS shortcuts (`Cmd+C`, `Cmd+V`, `Cmd+X`, `Cmd+Shift+Z`) to ensure full platform parity and resolve non-functional shortcut reports.
- **App Lifecycle & Menu State**: Moved action handlers for `New`, `Open`, `About`, and `Quit` to the application level. This ensures the menu remains enabled even when all windows are closed, and allows reopening the app via the menu.
- **Improved Dialogs**: Updated `WindowView` to ensure `About` dialog can be triggered globally and improved background dimming for modal dialogs.

## 2026-05-08
...
### Completed Phase 13: led-gui — gpui Setup & Window Skeleton
- Pinned `gpui` to commit `6766514`.
- Implemented `led-gui` entry point and main application loop.
- Set up platform-conditional menu system: native NSMenu for macOS, placeholder in-window menu bar for others.
- Created `WindowView` root component that composes child views based on platform.
- Implemented stubs for `EditorView`, `TabBar`, `StatusBar`, `FindPanel`, and `MenuBar`.
- Resolved macOS build issues by implementing an `xcrun` shim that redirects Metal tool calls to the correct toolchain path.
- Updated `Makefile` to integrate shims and ensure stable builds for both TUI and GUI targets.

### Completed Phase 12: i18n & Final Polish
- Implemented full i18n framework in `led-core`.
- Added built-in Japanese (`ja`) locale and support for external TOML locales.
- Localized all UI elements in `led-tui`, including menus, dialogs, and status messages.
- Added `README.md` with installation and SSH usage instructions.
- Verified build and localization across all targets.

### Completed Phase 11: Syntax Highlighting & Themes
- Implemented regex-based syntax highlighting engine in `led-core` with `rayon` parallelization.
- Added 11 built-in syntax definitions (Rust, Python, Go, etc.) and 6 built-in themes (Tokyo Night, Solarized, etc.).
- Wired theme and syntax selection in `led-tui`, with live application and persistence.
- Verified highlighting performance and correctness for all supported languages.

## 2026-05-06
### Completed Phase 10: Vi Mode
- Implemented `Normal`, `Insert`, and `Visual` modes.
- Added support for mode switching via `i`, `a`, `o`, `v`, and `Esc`.
- Implemented core Vi navigation: `h`, `j`, `k`, `l`, `w`, `b`, `e`, `gg`, and `G`.
- Added editing commands: `dd` (delete line), `yy` (yank line), `p` (paste), `u` (undo), and `x` (delete char).
- Implemented basic command-line mode for `:w`, `:q`, and `:wq`.
- Integrated search (`/`) into Vi mode, opening the Find panel.
- Added a Vi mode indicator to the status bar and wired the `View > Vi Mode` toggle.
- Refactored TUI handlers to resolve borrow checker issues while calling editor actions.
- Verified build and basic functionality across all modes.

### Completed Phase 9: Find/Replace Panel
- Implemented inline Find/Replace panel with support for incremental search.
- Added support for `Match Case`, `Whole Word`, and `Use Regex` flags.
- Implemented `Next`/`Prev` navigation with wrap-around messages in the status bar.
- Added `Replace` and `Replace All` functionality, with occurrence count reporting.
- Made search results and status per-buffer, ensuring correct behavior when switching tabs.
- Integrated search highlighting into both word-wrapped and non-wrapped rendering modes.
- Wired `Edit > Find` and `Edit > Replace` menu items and keyboard shortcuts (`Ctrl+F`, `Ctrl+H`).

### Completed Phase 8: Encoding & Line Ending Support
- Improved encoding auto-detection in `led-core` to support UTF-8 (with/without BOM), UTF-16 LE/BE, Shift-JIS, EUC-JP, ISO-2022-JP, and Latin-1.
- Expanded TUI menu system to include all supported encoding and line ending options.
- Implemented `Reopen with Encoding` (disk reload) and `Convert to Encoding` (session change) actions.
- Added radio-style menu checkmarks (`✓`) for mutually exclusive options like Encoding, Line Ending, and Theme.
- Ensured the menu system and status bar stay synchronized with buffer state changes (tab switching, encoding/line ending updates).
- Verified core encoding detection logic in `led-core`.

### Completed Phase 7: Word Wrap
- Implemented logical vs. visual line splitting for display.
- Added visual line wrapping based on terminal width and `unicode-width`.
- Implemented visual navigation (`↑/↓`) when word wrap is enabled, moving by visual line rather than logical line.
- Updated the status bar to show visual column relative to the current visual line.
- Wired the `View > Word Wrap` toggle and ensured it persists to the config.
- Automatically disabled horizontal scrolling when word wrap is active.
- Verified wrapping and navigation logic with unit tests in `led-core`.

### Completed Phase 6: Buffer Management, Editing & Undo/Redo
- Implemented `Editor` core logic using `ropey` for efficient large file support.
- Implemented robust undo/redo stack with a 1000-entry limit.
- Added support for character-level text selection via mouse (drag, double/triple click, Shift+click) and keyboard.
- Implemented real-time buffer rendering in TUI, handling CJK width, tabs, and scrolling.
- Integrated selection and cursor info into the status bar.
- Verified core logic with unit tests for insertion, deletion, and undo/redo behavior.

### Completed Phase 4: Dialogs & File I/O
...

- Refined `FileBrowser` with `Detect Encoding` toggle, sorting, and relative modification times.
- Implemented robust `Buffer` loading and saving with `encoding_rs` (auto-detection and conversion).
- Enhanced `OpenDialog` and `SaveAsDialog` with inline error messages.
- Implemented `ReopenConfirmationDialog` for safe encoding changes.
- Added overwrite confirmation logic to `SaveAs` operation.
- Integrated `chrono` and `humantime` for UI time formatting.

### Completed Phase 3: Menu Bar
- Implemented top-level menu bar (`File`, `Edit`, `View`, `Help`) with localization support.
- Implemented borderless dropdown rendering for a modern TUI look.
- Added support for separators and toggle indicators.
- Implemented nested submenus with recursive rendering.
- Added mouse hit-testing for dropdowns via `dropdown_rects`.
- Implemented `Alt+F/E/V/H` keyboard shortcuts and arrow key navigation.
- Shared `Action` enum in `led-core` for cross-frontend consistency.
- Resolved borrow checker issues related to nested state access in `App`.
