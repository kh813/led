  # led Todo List

## Phase 18: Project Management & Navigation
- [ ] **File Explorer (Side Bar)**:
  - [ ] Implement directory scanning in `led-core`
  - [ ] Create `FileTreeView` in GUI / `SideBar` in TUI
  - [ ] Add `Alt+1` shortcut to toggle side bar focus
- [ ] **Fuzzy Finder (`Ctrl+P`)**:
  - [ ] Implement fast file indexing and fuzzy matching logic
  - [ ] Create searchable list dialog for both TUI and GUI
- [ ] **Recently Opened Files**:
  - [ ] Persist file history in `config.toml` or separate state file
  - [ ] Add `File > Recent Files` submenu
- [ ] **Project-wide Search (`Ctrl+Shift+F`)**:
  - [ ] Basic "grep" functionality across the current workspace directory

## Phase 19: Advanced Editing & Visuals
- [ ] **Bracket Matching**:
  - [ ] Highlight corresponding `()`, `[]`, `{}` pairs
- [ ] **Auto-indentation**:
  - [ ] Maintain indentation level on newline
  - [ ] Language-specific indentation rules (e.g., after `{`)
- [ ] **Comment Toggle (`Ctrl+/`)**:
  - [ ] Support single-line and block comment toggling based on syntax
- [ ] **Soft Tabs / Tab Conversion**:
  - [ ] Option to use spaces instead of tabs
  - [ ] "Convert Tabs to Spaces" action

## Phase 20: Settings & Ecosystem
- [ ] **Settings Dialog (UI-based Configuration)**:
  - [ ] Visual editor for `config.toml` settings
  - [ ] Live preview for theme and font changes
- [ ] **Keybinding Customization**:
  - [ ] Allow users to override default shortcuts in `config.toml`
- [x] **CI/CD & Compilation (GitHub Actions)**:
  - [x] Multi-platform compilation workflow (`release.yml`) for TUI (`led`) and GUI (`led-gui`) across macOS, Linux, and Windows
  - [x] Artifact upload and release packaging for tagged releases
- [ ] **Packaging & Distribution**:
  - [ ] Homebrew (macOS), NSIS (Windows), and .deb/.rpm (Linux) packages
  - [ ] Documentation for installation via `cargo install`

---

## Phase 17: Bugfixes & Polish (TUI & GUI)

### TUI Fixes
- [x] Fix editor visibility (cursor and text) in default themes
- [x] Fix highlight visibility in File Open/Save dialogs
- [x] Fix `Esc` key regression for closing dialogs
- [x] Fix Unsaved Changes dialog:
  - [x] Implement `Tab` navigation
  - [x] Ensure it appears when closing any modified buffer or quitting with any modified buffer
- [x] Fix Menu "Exit" action behavior
- [x] Improve TUI "Terminal Default" theme to match terminal color scheme (ANSI 16 colors)
- [x] Improve Japanese inline input (implemented via hardware cursor positioning)

### GUI Fixes
- [x] Fix editor visibility parity with TUI
- [x] Fix default window position (center on screen)
- [x] Fix automatic theme selection (OS light/dark mode)
- [x] Fix color visibility:
  - [x] Unify `led_color_to_gpui` across all widgets using `gpui::rgb`.
  - [x] Ensure `EditorView` uses consistent color mapping for text and background.
- [x] **Verify native GUI rendering (no invisible text)**
- [x] Implement Japanese inline input support (IME) in `EditorView`:
  - [x] Fix `marked_text_range` to return the composition range.
  - [x] Ensure `replace_and_mark_text_in_range` correctly manages composition state.
  - [x] Improve `bounds_for_range` for accurate candidate window placement.
- [x] Fix native macOS/Windows shortcuts:
  - [x] Verify `cmd-q`, `cmd-o`, `ctrl-q`, `ctrl-o` etc. are correctly bound and handled.
  - [x] Ensure `EditorView` doesn't intercept system/action shortcuts in `on_key_down`.
- [x] Fix app-level menu state when no windows are open:
  - [x] Ensure `New`, `Open`, and `Quit` actions remain enabled in the global menu.
  - [x] Verify `app.on_action` handlers are correctly registered.
- [x] Use OS native dialogs for Open/Save (integrate `rfd` crate) - Already partially done in code, ensure consistency.

### TUI Fixes
- [x] Fix CJK character spacing:
  - [x] Update `Renderer` to handle multi-width characters.
  - [x] Update `App` and `Dialog` to set correct character widths.
- [x] Improve IME candidate window placement by moving hardware cursor to logical cursor position.

- [x] Achieve menu parity with TUI (full Encoding and Line Ending support)
- [x] Fix dialog positioning and window bounding
- [x] **Implement file drag-and-drop support**:
  - [x] Register drag-and-drop event handler in `app.rs` or `window_view.rs`
  - [x] Implement path extraction from drop events
  - [x] Add logic to check for existing tabs before opening new ones
  - [x] Ensure the last dropped file becomes the active tab
  - [x] Verify handling of multiple files dropped at once

### Common / Others
- [x] Integrate application icons:
  - [x] Ensure icon files are in place (`.icns` for Mac, `.ico` for Windows)
  - [x] Set icons for macOS `.app` bundle and Windows `.exe`
- [x] Final performance and UI polish

### ✅ Phase 17 Completion Log

- **Completed**: 2026-05-13
- **Commit**: `(pending)`
- **Implementer**: AI session & Gemini CLI
- **Files created**:
  - `crates/led-gui/build.rs` — Added Windows resource compilation.
  - `crates/led-gui/resources/` — Organized icon assets.
- **Files modified**:
  - `crates/led-gui/Cargo.toml` — Added `winres` build dependency.
  - `crates/led-tui/src/app.rs` — Fixed hardware cursor visibility and placement.
  - `crates/led-gui/src/widgets/editor_view.rs` — Robust rendering and IME fixes.
- **Key decisions made**:
  - Embedded Windows icon via `winres` for native `.exe` appearance.
  - Fixed TUI IME positioning by moving the hardware cursor to the logical cursor.
  - Explicitly set monospace fonts and unified color mapping in GUI to ensure visibility.
- **Known issues / deferred work**:
  - File drag-and-drop support remains limited in current GPUI version.

### ✅ Phase 16 Completion Log

- **Completed**: 2026-05-09
- **Commit**: `d88e60f9c2ef97733417b870ebb15587c966af9b` (and subsequent bugfixes)
- **Implementer**: AI session & Gemini CLI
- **Files created**: None
- **Files modified**:
  - `crates/led-gui/src/app.rs` — Fixed compilation, added dynamic theme selection and parameterized actions.
  - `crates/led-gui/src/window_view.rs` — Fixed dialog overlay, workspace notifications, and encoding/line ending handlers.
  - `crates/led-gui/src/widgets/editor_view.rs` — Improved text visibility, font inheritance, and model observation.
  - `crates/led-gui/src/widgets/dialog.rs` — Full implementation of modal dialogs and file browser.
- **Key decisions made**:
  - Implemented a parameterized `SetTheme` action in GPUI to support dynamic theme selection.
  - Switched to `observe` for the `Workspace` model to ensure correct UI re-renders on state change.
  - Used absolute positioning for the dialog overlay to keep it centered and within window boundaries.
- **Known issues / deferred work**:
  - File drag-and-drop was removed due to API compatibility issues with the pinned GPUI version.
  - Scroll performance for large file lists in dialogs could be improved.
- **Bugfixes addressed**:
  - Grayed-out Encoding, Line Ending, and Theme menus are now functional.
  - Visibility in default theme improved (fixed font/size inheritance).
  - Dialogs no longer exceed window boundaries.
  - Fixed a critical compilation error in `Workspace::new` call.
