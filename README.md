# zee (ZEpto Editor)

**zee** is a modern and lightweight text editor built in Rust. It provides a native, hardware-accelerated **GUI** desktop experience (`zeeg`) alongside a feature-packed **TUI** (Terminal User Interface, `zee`) that shares the exact same shortcuts, menus, dialogs, and intuitive feel.

Whether launching `Zee.app` on your desktop or running `zee` over SSH in a terminal, **zee** gives you a consistent, distraction-free editing workflow.

## Features

- **Dual GUI & TUI Experience**: Native GPU-accelerated desktop GUI (`zeeg` on macOS, Windows, and Linux [Experimental]) and a responsive terminal TUI (`zee`) sharing identical workflows and shortcuts.
- **Modern Aesthetics & Themes**: High-contrast, beautifully themed UI with customizable colors.
- **Menu & Dialog Driven**: Intuitive top-level menu bar, dropdowns, and modal dialogs (Goto line, Open, Save As, Settings).
- **Find & Replace Panel**: Interactive search with match counting, regex, case sensitivity, and whole word support.
- **Tabs Support**: Open and switch between multiple files with tabs.
- **Internationalization (i18n)**: Full English and Japanese support out of the box.
- **Syntax Highlighting**: Fast syntax highlighting for Rust, Markdown, Python, JSON, TOML, YAML, JavaScript/TypeScript, and more.
- **Vi Mode**: Optional Vi modal editing for power users.
- **OSC 52 Clipboard**: Seamless system clipboard support locally and over remote SSH sessions.

## Installation

### Build from Source

**Requirements**: Rust toolchain (`rustup`), C compiler / build-essential

#### Using Make (macOS / Linux):
```bash
git clone https://github.com/kh813/zee.git
cd zee

# Build both GUI and TUI binaries into dist/
make

# Or build individual components:
make gui    # Builds zeeg (and dist/Zee.app on macOS)
make tui    # Builds TUI binary (dist/zee)

# Install to system (~/.local/bin and ~/Applications on macOS)
make install
```

#### On Windows (PowerShell):
```powershell
# Using the make.ps1 build script (no make required)
.\make.ps1           # Builds Windows GUI into dist/zeeg.exe
.\make.ps1 tui       # Builds Windows TUI into dist/zee.exe
.\make.ps1 test      # Runs tests
.\make.ps1 package   # Creates dist/zee-windows-x64.zip

# Or directly with Cargo:
cargo build --release -p zee-gui   # Windows GUI: target/release/zeeg.exe
cargo build --release -p zee-tui   # Terminal TUI: target/release/zee.exe
```

## Usage

### Desktop GUI
- **macOS**: Launch `dist/Zee.app` (or open from Launchpad / Applications folder).
- **Windows**: Launch `zeeg.exe [FILE...]`.
- **Linux (Experimental)**: Launch **zee** from your Desktop Application Menu, or run `zeeg [FILE...]` from terminal.

### Terminal TUI / Remote (SSH)
- **All Platforms**: Run `zee [FILE...]` in your terminal.

For complete shortcut references and configuration guides, see the [User Manual](MANUAL.md).

## Configuration

Configuration is stored in `~/.config/zee/config.toml` and shared across both GUI and TUI:
```toml
theme = "default_dark"
tab_size = 4
line_numbers = true
language = "en"  # "en" or "ja"
```

## Remote Usage (SSH)

zee's TUI works great over SSH. To prevent `Ctrl+S` (Save) from triggering terminal software flow control, add the following to your shell profile (`~/.bashrc` or `~/.zshrc`):

```bash
stty -ixon
```

## License

MIT

